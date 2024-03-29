use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    guild::Role,
    id::{
        marker::{RoleMarker, UserMarker},
        Id,
    },
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rewards",
    desc = "Manage automatic role rewards",
    dm_permission = false
)]
pub enum XpCommandRewards {
    #[command(name = "add")]
    Add(XpCommandRewardsAdd),
    #[command(name = "remove")]
    Remove(XpCommandRewardsRemove),
    #[command(name = "list")]
    List(XpCommandRewardsList),
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "add", desc = "Add a new leveling reward")]
pub struct XpCommandRewardsAdd {
    #[command(desc = "What level to grant the role reward at", min_value = 1)]
    pub level: i64,
    #[command(desc = "What role to grant", min_value = 1)]
    pub role: Role,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "remove", desc = "Remove a leveling reward")]
pub struct XpCommandRewardsRemove {
    #[command(desc = "What level of role reward to remove", min_value = 1)]
    pub level: Option<i64>,
    #[command(desc = "What role reward to remove")]
    pub role: Option<Id<RoleMarker>>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "list", desc = "Show a list of leveling rewards")]
pub struct XpCommandRewardsList;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "experience",
    desc = "Manage user experience in this guild",
    dm_permission = false
)]
pub enum XpCommandExperience {
    #[command(name = "add")]
    Add(XpCommandExperienceAdd),
    #[command(name = "remove")]
    Remove(XpCommandExperienceRemove),
    #[command(name = "reset")]
    Reset(XpCommandExperienceReset),
    #[command(name = "import")]
    Import(XpCommandExperienceImport),
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "add",
    desc = "Add experience points to a user",
    dm_permission = false
)]
pub struct XpCommandExperienceAdd {
    #[command(desc = "User to add experience to")]
    pub user: Id<UserMarker>,
    #[command(desc = "Amount of experience to add", min_value = 1)]
    pub amount: i64,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "remove",
    desc = "Remove experience points from a user",
    dm_permission = false
)]
pub struct XpCommandExperienceRemove {
    #[command(desc = "User to remove experience from")]
    pub user: Id<UserMarker>,
    #[command(desc = "Amount of experience to remove", min_value = 1)]
    pub amount: i64,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "reset",
    desc = "Reset a user's experienced progress & remove them from the leaderboard",
    dm_permission = false
)]
pub struct XpCommandExperienceReset {
    #[command(desc = "User to remove")]
    pub user: Id<UserMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "import",
    desc = "Import your server's mee6 leveling data. WARNING: THIS WILL OVERWRITE ANY CURRENT LEVELING DATA!",
    dm_permission = false
)]
pub struct XpCommandExperienceImport;
