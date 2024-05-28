use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, RwLock},
};

use expiringmap::ExpiringSet;
use sqlx::{query, query_as, PgPool};
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::{event::Event, Intents},
    id::{
        marker::{ApplicationMarker, GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};
use xpd_common::{db_to_id, id_to_db, GuildConfig, RequiredEvents, RoleReward};
use xpd_permission_cache::PermissionCache;

mod message;

#[macro_use]
extern crate tracing;

type SentMessages = ExpiringSet<(Id<GuildMarker>, Id<UserMarker>)>;
type LockingMap<K, V> = RwLock<HashMap<K, V>>;

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

impl RequiredEvents for XpdListener {
    fn required_intents() -> Intents {
        XpdListenerInner::required_intents()
    }

    fn required_events() -> EventTypeFlags {
        XpdListenerInner::required_events()
    }
}

pub struct XpdListenerInner {
    db: PgPool,
    messages: RwLock<SentMessages>,
    http: Arc<twilight_http::Client>,
    cache: PermissionCache,
    configs: LockingMap<Id<GuildMarker>, GuildConfig>,
    rewards: LockingMap<Id<GuildMarker>, Arc<Vec<RoleReward>>>,
}

impl XpdListenerInner {
    pub(crate) fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        current_application_id: Id<ApplicationMarker>,
    ) -> Self {
        let messages = RwLock::new(SentMessages::new());
        let configs = RwLock::new(HashMap::new());
        let rewards = RwLock::new(HashMap::new());
        let cache = PermissionCache::new(current_application_id);

        Self {
            db,
            messages,
            http,
            configs,
            rewards,
            cache,
        }
    }

    pub fn update_cache(&self, uc: &Event) {
        if let Err(source) = self.cache.update_cache(uc) {
            error!(?source, "Failed to update cache");
        }
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

impl RequiredEvents for XpdListenerInner {
    fn required_intents() -> Intents {
        PermissionCache::required_intents() | Intents::GUILD_MESSAGES
    }

    fn required_events() -> EventTypeFlags {
        PermissionCache::required_events() | EventTypeFlags::MESSAGE_CREATE
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
    #[error("Could not add roles in guild {0}")]
    NoPermsToAddRoles(Id<GuildMarker>, xpd_permission_cache::CanAddRolesInfo),
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned
    }
}
