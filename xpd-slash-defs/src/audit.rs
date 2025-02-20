use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    guild::Permissions,
    id::{Id, marker::UserMarker},
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "audit",
    desc = "Download audit logs for your server",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
pub struct AuditLogCommand {
    #[command(desc = "Fetch audit logs triggered by this moderator")]
    pub moderator: Option<Id<UserMarker>>,
    #[command(desc = "Fetch audit logs acting on this user")]
    pub user: Option<Id<UserMarker>>,
}

impl AuditLogCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::MODERATE_MEMBERS
    }
}
