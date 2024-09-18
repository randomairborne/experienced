use twilight_cache_inmemory::{CacheableRole, InMemoryCache};
use twilight_model::{
    guild::Permissions,
    id::{
        marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};

#[macro_use]
extern crate tracing;

pub fn can_add_roles(
    cache: &InMemoryCache,
    bot_id: Id<UserMarker>,
    guild_id: Id<GuildMarker>,
    targets: &[Id<RoleMarker>],
) -> Result<CanAddRole, PermissionCheckError> {
    if targets.is_empty() {
        return Ok(CanAddRole::Yes);
    }
    if !cache
        .permissions()
        .root(bot_id, guild_id)?
        .contains(Permissions::MANAGE_ROLES)
    {
        debug!(guild = ?guild_id, "No permissions to add role to any user");
        return Ok(CanAddRole::NoManageRoles);
    }

    let highest_role = cache
        .member_highest_role(guild_id, bot_id)
        .ok_or(PermissionCheckError::NoHighestRoleForSelf)?;

    let my_position = cache
        .role(highest_role)
        .ok_or(PermissionCheckError::UnknownPositionForOwnHighestRole)?
        .position();
    let (max_position, max_role) = {
        let mut max_position = i64::MIN;
        let mut max_role = targets[0];
        for role in targets {
            let role = cache
                .role(*role)
                .ok_or(PermissionCheckError::NoTargetRoleInCache)?;
            if role.managed {
                return Ok(CanAddRole::RoleIsManaged);
            }
            if role.position() > max_position {
                max_position = role.position();
                max_role = role.id();
            }
        }
        (max_position, max_role)
    };

    if my_position > max_position || max_role.get() < bot_id.get() {
        Ok(CanAddRole::Yes)
    } else {
        Ok(CanAddRole::HighestRoleIsLowerRoleThanTarget)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CanAddRole {
    Yes,
    NoManageRoles,
    HighestRoleIsLowerRoleThanTarget,
    RoleIsManaged,
}

impl CanAddRole {
    pub fn can_add_role(&self) -> bool {
        matches!(self, CanAddRole::Yes)
    }
}

pub fn can_create_message(
    cache: &InMemoryCache,
    bot_id: Id<UserMarker>,
    channel_id: Id<ChannelMarker>,
) -> Result<bool, PermissionCheckError> {
    cache
        .permissions()
        .in_channel(bot_id, channel_id)
        .map(|v| {
            trace!(channel = ?channel_id, permissions = v.bits(), "Got permissions in channel");
            v.contains(Permissions::SEND_MESSAGES)
        })
        .map_err(Into::into)
}

#[derive(Debug, thiserror::Error)]
pub enum PermissionCheckError {
    #[error("Unknown role: <@&{0}>")]
    UnknownRole(Id<RoleMarker>),
    #[error("Could not load permissions")]
    CacheRootError(#[from] twilight_cache_inmemory::permission::RootError),
    #[error("Could not load channels")]
    CacheChannelError(#[from] twilight_cache_inmemory::permission::ChannelError),
    #[error("Highest known role for self was not found in cache!")]
    NoHighestRoleForSelf,
    #[error("Target role was not found in cache!")]
    NoTargetRoleInCache,
    #[error("Got unknown role for own highest role!")]
    UnknownPositionForOwnHighestRole,
}
