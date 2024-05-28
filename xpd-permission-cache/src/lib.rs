use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::{Mutex, RwLock},
};

use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::{event::Event, Intents},
    guild::{Permissions, Role},
    id::{
        marker::{ApplicationMarker, GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};
use xpd_common::RequiredEvents;

type LockingMap<K, V> = RwLock<HashMap<K, V>>;

#[derive(Copy, Clone, Debug)]
pub struct RoleMetadata {
    pub position: i64,
    pub permissions: Permissions,
}

pub struct PermissionCache {
    role_cache: LockingMap<Id<RoleMarker>, RoleMetadata>,
    guild_role_cache: Mutex<HashMap<Id<GuildMarker>, HashSet<Id<RoleMarker>>>>,
    me_cache: LockingMap<Id<GuildMarker>, Vec<Id<RoleMarker>>>,
    current_application_id: Id<ApplicationMarker>,
}

impl PermissionCache {
    pub fn new(current_application_id: Id<ApplicationMarker>) -> Self {
        let guild_role_cache = Mutex::new(HashMap::new());
        let role_cache = RwLock::new(HashMap::new());
        let me_cache = RwLock::new(HashMap::new());

        Self {
            guild_role_cache,
            role_cache,
            me_cache,
            current_application_id,
        }
    }

    pub fn update_cache(&self, uc: &Event) -> Result<(), Error> {
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
        if self.current_application_id.cast() != user_id {
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
        if self.current_application_id.cast() != user_id {
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
        let role_list = [target_role];
        self.can_add_roles(guild_id, role_list.as_slice())
    }

    pub fn can_add_roles(
        &self,
        guild_id: Id<GuildMarker>,
        target_roles: &[Id<RoleMarker>],
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

        let mut can_assign = true;
        for target_role in target_roles {
            let target = rc
                .get(target_role)
                .ok_or(Error::UnknownRole(*target_role))?;

            // if the target's position is more than our virtual position
            if target.position >= virtual_role.position {
                can_assign = false;
            }
        }

        Ok(can_assign)
    }

    pub fn my_roles(&self, guild_id: Id<GuildMarker>) -> Result<Vec<Id<RoleMarker>>, Error> {
        Ok(self
            .me_cache
            .read()?
            .get(&guild_id)
            .cloned()
            .unwrap_or_else(Vec::new))
    }
}

impl RequiredEvents for PermissionCache {
    fn required_intents() -> Intents {
        Intents::GUILDS
    }

    fn required_events() -> EventTypeFlags {
        EventTypeFlags::ROLE_CREATE
            | EventTypeFlags::ROLE_UPDATE
            | EventTypeFlags::ROLE_DELETE
            | EventTypeFlags::GUILD_CREATE
            | EventTypeFlags::GUILD_UPDATE
            | EventTypeFlags::GUILD_DELETE
            | EventTypeFlags::MEMBER_ADD
            | EventTypeFlags::MEMBER_UPDATE
            | EventTypeFlags::MEMBER_REMOVE
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("RwLock Poisioned, please report this bug")]
    LockPoisoned,
    #[error("Unknown role: <@&{0}>")]
    UnknownRole(Id<RoleMarker>),
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned
    }
}
