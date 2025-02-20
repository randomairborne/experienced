use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    guild::{Permissions, Role},
    id::{Id, marker::RoleMarker},
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rewards",
    desc = "Manage automatic role rewards",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
pub enum RewardsCommand {
    #[command(name = "add")]
    Add(RewardsCommandAdd),
    #[command(name = "remove")]
    Remove(RewardsCommandRemove),
    #[command(name = "list")]
    List(RewardsCommandList),
}

impl RewardsCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "add",
    desc = "Add a new leveling reward",
    dm_permission = false
)]
pub struct RewardsCommandAdd {
    #[command(desc = "What level to grant the role reward at", min_value = 1)]
    pub level: i64,
    #[command(desc = "What role to grant", min_value = 1)]
    pub role: Role,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "remove",
    desc = "Remove a leveling reward",
    dm_permission = false
)]
pub struct RewardsCommandRemove {
    #[command(desc = "What level of role reward to remove", min_value = 1)]
    pub level: Option<i64>,
    #[command(desc = "What role reward to remove")]
    pub role: Option<Id<RoleMarker>>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "list",
    desc = "Show a list of leveling rewards",
    dm_permission = false
)]
pub struct RewardsCommandList;
