use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};
use twilight_model::guild::Permissions;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "xp",
    desc = "Manage user experience in this guild",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
pub enum XpCommand {
    #[command(name = "add")]
    Add(XpCommandAdd),
    #[command(name = "remove")]
    Remove(XpCommandRemove),
    #[command(name = "reset")]
    Reset(XpCommandReset),
    #[command(name = "set")]
    Set(XpCommandSet),
}

impl XpCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::MODERATE_MEMBERS
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "add",
    desc = "Add experience points to a user",
    dm_permission = false
)]
pub struct XpCommandAdd {
    #[command(desc = "User to add experience to")]
    pub user: ResolvedUser,
    #[command(desc = "Amount of experience to add", min_value = 1)]
    pub amount: i64,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "remove",
    desc = "Remove experience points from a user",
    dm_permission = false
)]
pub struct XpCommandRemove {
    #[command(desc = "User to remove experience from")]
    pub user: ResolvedUser,
    #[command(desc = "Amount of experience to remove", min_value = 1)]
    pub amount: i64,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "reset",
    desc = "Reset a user's experienced progress & remove them from the leaderboard",
    dm_permission = false
)]
pub struct XpCommandReset {
    #[command(desc = "User to remove")]
    pub user: ResolvedUser,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "set",
    desc = "Set a user's experience value",
    dm_permission = false
)]
pub struct XpCommandSet {
    #[command(desc = "User to set XP of")]
    pub user: ResolvedUser,
    #[command(desc = "value to set their current XP to", min_value = 1)]
    pub xp: i64,
}
