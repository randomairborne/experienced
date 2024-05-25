use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    channel::Attachment,
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
    #[command(name = "config")]
    Config(XpCommandRewardsConfig),
    #[command(name = "reset_config")]
    ResetConfig(XpCommandRewardsResetConfig),
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "add",
    desc = "Add a new leveling reward",
    dm_permission = false
)]
pub struct XpCommandRewardsAdd {
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
pub struct XpCommandRewardsRemove {
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
pub struct XpCommandRewardsList;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "config",
    desc = "Configure role reward behavior",
    dm_permission = false
)]
pub struct XpCommandRewardsConfig {
    #[command(desc = "Remove all existing Experienced-managed roles when assigning a new one")]
    pub one_at_a_time: Option<bool>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "reset_config", desc = "Reset your guild's configuration")]
pub struct XpCommandRewardsResetConfig;

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
    #[command(name = "export")]
    Export(XpCommandExperienceExport),
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
    desc = "Import leveling data from another Discord bot or other source",
    dm_permission = false
)]
pub struct XpCommandExperienceImport {
    #[command(desc = "Leveling JSON file")]
    pub levels: Attachment,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "export",
    desc = "Export this server's leveling data into a JSON file",
    dm_permission = false
)]
pub struct XpCommandExperienceExport;
