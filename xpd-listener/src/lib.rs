use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
};

use expiringmap::ExpiringSet;
use sqlx::{query, query_as, PgPool};
use twilight_model::{
    gateway::event::Event,
    guild::{Permissions, Role},
    id::{
        marker::{ApplicationMarker, GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};
use xpd_common::{db_to_id, id_to_db, GuildConfig, RoleReward};

mod message;

#[macro_use]
extern crate tracing;

type SentMessages = ExpiringSet<(Id<GuildMarker>, Id<UserMarker>)>;
type LockingMap<K, V> = RwLock<HashMap<K, V>>;

#[derive(Copy, Clone, Debug)]
struct RoleMetadata {
    pub position: i64,
    pub permissions: Permissions,
}

#[derive(Clone)]
pub struct XpdListener(Arc<XpdListenerInner>);

impl XpdListener {
    pub fn new(db: PgPool, http: Arc<twilight_http::Client>, me: Id<ApplicationMarker>) -> Self {
        Self(Arc::new(XpdListenerInner::new(db, http, me)))
    }
}

impl Deref for XpdListener {
    type Target = XpdListenerInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

pub struct XpdListenerInner {
    db: PgPool,
    messages: RwLock<SentMessages>,
    http: Arc<twilight_http::Client>,
    role_cache: LockingMap<Id<RoleMarker>, RoleMetadata>,
    guild_role_cache: Mutex<HashMap<Id<GuildMarker>, HashSet<Id<RoleMarker>>>>,
    me_cache: LockingMap<Id<GuildMarker>, Vec<Id<RoleMarker>>>,
    configs: LockingMap<Id<GuildMarker>, GuildConfig>,
    rewards: LockingMap<Id<GuildMarker>, Arc<Vec<RoleReward>>>,
    me: Id<ApplicationMarker>,
}

impl XpdListenerInner {
    pub(crate) fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        me: Id<ApplicationMarker>,
    ) -> Self {
        let messages = RwLock::new(SentMessages::new());
        let configs = RwLock::new(HashMap::new());
        let rewards = RwLock::new(HashMap::new());
        let guild_role_cache = Mutex::new(HashMap::new());
        let role_cache = RwLock::new(HashMap::new());
        let me_cache = RwLock::new(HashMap::new());

        Self {
            db,
            messages,
            http,
            configs,
            rewards,
            guild_role_cache,
            role_cache,
            me_cache,
            me,
        }
    }

    pub fn update_cache(&self, uc: &Event) {
        if let Err(source) = self.inner_update_cache(uc) {
            error!(?source, "Failed to update cache");
        }
    }

    fn inner_update_cache(&self, uc: &Event) -> Result<(), Error> {
        match uc {
            Event::RoleCreate(rc) => self.cache_insert_role(rc.guild_id, &rc.role),
            Event::RoleUpdate(ru) => self.cache_insert_role(ru.guild_id, &ru.role),
            Event::RoleDelete(rd) => self.cache_remove_role(rd.guild_id, rd.role_id),
            Event::GuildCreate(gc) => self.cache_reset_guild(gc.id, &gc.roles),
            Event::GuildUpdate(gu) => self.cache_reset_guild(gu.id, &gu.roles),
            Event::GuildDelete(gd) => self.cache_delete_guild(gd.id),
            Event::MemberUpdate(mu) => self.cache_insert_self(mu.user.id, mu.guild_id, &mu.roles),
            Event::MemberAdd(ma) => self.cache_insert_self(ma.user.id, ma.guild_id, &ma.roles),
            Event::MemberRemove(mr) => self.cache_remove_self(mr.user.id, mr.guild_id),
            _ => Ok(()),
        }
    }

    fn cache_insert_role(&self, guild_id: Id<GuildMarker>, role: &Role) -> Result<(), Error> {
        let role_meta = RoleMetadata {
            position: role.position,
            permissions: role.permissions,
        };
        self.role_cache.write()?.insert(role.id, role_meta);
        match self.guild_role_cache.lock()?.entry(guild_id) {
            Entry::Occupied(mut o) => {
                o.get_mut().insert(role.id);
            }
            Entry::Vacant(e) => {
                let mut new_set = HashSet::new();
                new_set.insert(role.id);
                e.insert(new_set);
            }
        };
        Ok(())
    }

    fn cache_remove_role(
        &self,
        guild_id: Id<GuildMarker>,
        role_id: Id<RoleMarker>,
    ) -> Result<(), Error> {
        self.role_cache.write()?.remove(&role_id);
        let mut grc = self.guild_role_cache.lock()?;
        if let Some(roles) = grc.get_mut(&guild_id) {
            roles.remove(&role_id);
            if roles.is_empty() {
                grc.remove(&guild_id);
            }
        }
        Ok(())
    }

    fn cache_reset_guild(&self, guild_id: Id<GuildMarker>, roles: &[Role]) -> Result<(), Error> {
        let mut new_guild_role_set = HashSet::with_capacity(roles.len());
        for role in roles {
            new_guild_role_set.insert(role.id);
        }

        let old_roles = self
            .guild_role_cache
            .lock()?
            .insert(guild_id, new_guild_role_set);

        let mut rcw = self.role_cache.write()?;

        if let Some(mut old_roles) = old_roles {
            for role in roles {
                old_roles.remove(&role.id);
            }
        }

        for role in roles {
            let role_meta = RoleMetadata {
                position: role.position,
                permissions: role.permissions,
            };
            rcw.insert(role.id, role_meta);
        }

        Ok(())
    }

    fn cache_delete_guild(&self, guild: Id<GuildMarker>) -> Result<(), Error> {
        if let Some(grc_set) = self.guild_role_cache.lock()?.remove(&guild) {
            let mut role_map = self.role_cache.write()?;
            for item in grc_set {
                role_map.remove(&item);
            }
        }
        self.me_cache.write()?.remove(&guild);
        Ok(())
    }

    fn cache_insert_self(
        &self,
        user_id: Id<UserMarker>,
        guild_id: Id<GuildMarker>,
        roles: &[Id<RoleMarker>],
    ) -> Result<(), Error> {
        if self.me.cast() != user_id {
            return Ok(());
        }
        self.me_cache.write()?.insert(guild_id, roles.to_vec());
        Ok(())
    }

    fn cache_remove_self(
        &self,
        user_id: Id<UserMarker>,
        guild_id: Id<GuildMarker>,
    ) -> Result<(), Error> {
        if self.me.cast() != user_id {
            return Ok(());
        }
        self.me_cache.write()?.remove(&guild_id);
        Ok(())
    }

    pub fn can_add_role(
        &self,
        guild_id: Id<GuildMarker>,
        target_role: Id<RoleMarker>,
    ) -> Result<bool, Error> {
        let Some(my_roles) = self.me_cache.read()?.get(&guild_id).cloned() else {
            return Ok(false);
        };

        let rc = self.role_cache.read()?;

        let everyone_role: Id<RoleMarker> = guild_id.cast();
        let baseline = rc
            .get(&everyone_role)
            .ok_or(Error::UnknownRole(everyone_role))?;

        let mut virtual_role = RoleMetadata {
            position: baseline.position,
            permissions: baseline.permissions,
        };

        for role_id in my_roles {
            if let Some(role) = rc.get(&role_id) {
                virtual_role.position = std::cmp::max(virtual_role.position, role.position);
                virtual_role.permissions |= role.permissions;
            }
        }

        if !virtual_role.permissions.contains(Permissions::MANAGE_ROLES)
            && !virtual_role
                .permissions
                .contains(Permissions::ADMINISTRATOR)
        {
            return Ok(false);
        }

        let target = rc
            .get(&target_role)
            .ok_or(Error::UnknownRole(target_role))?;

        // if the target's position is less than our virtual position
        Ok(target.position < virtual_role.position)
    }

    pub fn my_roles(&self, guild_id: Id<GuildMarker>) -> Result<Vec<Id<RoleMarker>>, Error> {
        Ok(self
            .me_cache
            .read()?
            .get(&guild_id)
            .cloned()
            .unwrap_or_else(Vec::new))
    }

    pub fn update_config(&self, guild: Id<GuildMarker>, config: GuildConfig) -> Result<(), Error> {
        self.configs.write()?.insert(guild, config);
        Ok(())
    }

    pub async fn get_guild_config(&self, guild: Id<GuildMarker>) -> Result<GuildConfig, Error> {
        if let Some(cfg) = self.configs.read()?.get(&guild) {
            return Ok(cfg.clone());
        }
        let config = query_as!(
            GuildConfig,
            "SELECT one_at_a_time FROM guild_configs WHERE id = $1",
            id_to_db(guild)
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or_else(GuildConfig::default);
        self.configs.write()?.insert(guild, config.clone());
        Ok(config)
    }

    pub async fn invalidate_rewards(&self, guild: Id<GuildMarker>) -> Result<(), Error> {
        let mut new_rewards = self.get_guild_rewards_uncached(guild).await?;
        new_rewards.sort_by(xpd_common::sort_rewards);
        self.rewards.write()?.insert(guild, Arc::new(new_rewards));
        Ok(())
    }

    pub async fn get_guild_rewards(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Arc<Vec<RoleReward>>, Error> {
        if let Some(rewards) = self.rewards.read()?.get(&guild_id) {
            return Ok(rewards.clone());
        }
        let mut rewards = self.get_guild_rewards_uncached(guild_id).await?;
        rewards.sort_by(xpd_common::sort_rewards);

        let new_copy = Arc::new(rewards);
        self.rewards.write()?.insert(guild_id, new_copy.clone());
        Ok(new_copy)
    }

    async fn get_guild_rewards_uncached(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Vec<RoleReward>, Error> {
        let rewards: Vec<RoleReward> = query!(
            "SELECT id, requirement FROM role_rewards WHERE guild = $1",
            id_to_db(guild_id),
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|v| RoleReward {
            id: db_to_id(v.id),
            requirement: v.requirement,
        })
        .collect();
        Ok(rewards)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Discord error")]
    Twilight(#[from] twilight_http::Error),
    #[error("RwLock Poisioned, please report: https://valk.sh/discord")]
    LockPoisoned,
    #[error("Discord did not send a member where they MUST send a member")]
    NoMember,
    #[error("Unknown role: <@&{0}>")]
    UnknownRole(Id<RoleMarker>),
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned
    }
}
