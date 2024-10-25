use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};
use twilight_model::{application::command::CommandType, guild::Permissions};
use twilight_util::builder::command::CommandBuilder;

use crate::SlashState;

pub mod admin;
pub mod card;
pub mod config;
pub mod gdpr;
pub mod manage;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "help",
    desc = "Learn about how to use experienced",
    dm_permission = true
)]
pub struct HelpCommand;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "leaderboard",
    desc = "See the leaderboard for this server",
    dm_permission = false
)]
pub struct LeaderboardCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
    #[command(desc = "Page to jump to", min_value = 1)]
    pub page: Option<i64>,
    #[command(desc = "Want to show this off to everyone?")]
    pub show_off: Option<bool>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rank",
    desc = "Check someone's rank and level",
    dm_permission = false
)]
pub struct RankCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
    #[command(desc = "Show off this card publicly")]
    pub showoff: Option<bool>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "admin",
    desc = "Globally manage the bot",
    dm_permission = false
)]
#[allow(clippy::large_enum_variant)]
pub enum AdminCommand {
    #[command(name = "leave")]
    Leave(admin::AdminCommandLeave),
    #[command(name = "resetguild")]
    ResetGuild(admin::AdminCommandResetGuild),
    #[command(name = "resetuser")]
    ResetUser(admin::AdminCommandResetUser),
    #[command(name = "setnick")]
    SetNick(admin::AdminCommandSetNick),
    #[command(name = "banguild")]
    BanGuild(admin::AdminCommandBanGuild),
    #[command(name = "pardonguild")]
    PardonGuild(admin::AdminCommandPardonGuild),
    #[command(name = "guildstats")]
    GuildStats(admin::AdminCommandGuildStats),
    #[command(name = "stats")]
    Stats(admin::AdminCommandStats),
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "card",
    desc = "Set hex codes for different color schemes in your rank card.",
    dm_permission = true
)]
#[allow(clippy::large_enum_variant)]
pub enum CardCommand {
    #[command(name = "reset")]
    Reset(card::CardCommandReset),
    #[command(name = "fetch")]
    Fetch(card::CardCommandFetch),
    #[command(name = "edit")]
    Edit(card::CardCommandEdit),
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "config",
    desc = "Configure the behavior of the bot in your server",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
#[allow(clippy::large_enum_variant)]
pub enum ConfigCommand {
    #[command(name = "reset")]
    Reset(config::ConfigCommandReset),
    #[command(name = "get")]
    Get(config::ConfigCommandGet),
    #[command(name = "rewards")]
    Rewards(config::ConfigCommandRewards),
    #[command(name = "levels")]
    Levels(config::ConfigCommandLevels),
    #[command(name = "perms_checkup")]
    PermsCheckup(config::ConfigCommandPermsCheckup),
}

impl ConfigCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "guild-card",
    desc = "Set hex codes for different color schemes in your server's default rank card.",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
#[allow(clippy::large_enum_variant)]
pub enum GuildCardCommand {
    #[command(name = "reset")]
    Reset(card::CardCommandReset),
    #[command(name = "fetch")]
    Fetch(card::GuildCardCommandFetch),
    #[command(name = "edit")]
    Edit(card::CardCommandEdit),
}

impl GuildCardCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "xp",
    desc = "Manage administrator-only bot functions",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
pub enum XpCommand {
    #[command(name = "rewards")]
    Rewards(manage::XpCommandRewards),
    #[command(name = "experience")]
    Experience(manage::XpCommandExperience),
}

impl XpCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "gdpr",
    desc = "Exercise your rights under the GDPR",
    dm_permission = true
)]
pub enum GdprCommand {
    #[command(name = "delete")]
    Delete(gdpr::GdprCommandDelete),
    #[command(name = "download")]
    Download(gdpr::GdprCommandDownload),
}

impl SlashState {
    /// # Panics
    /// Can panic if setting the global commands fails
    pub async fn register_slashes(&self) {
        let cmds = vec![
            XpCommand::create_command().into(),
            RankCommand::create_command().into(),
            CardCommand::create_command().into(),
            HelpCommand::create_command().into(),
            GdprCommand::create_command().into(),
            GuildCardCommand::create_command().into(),
            ConfigCommand::create_command().into(),
            LeaderboardCommand::create_command().into(),
            CommandBuilder::new("Get level", "", CommandType::User).build(),
            CommandBuilder::new("Get author level", "", CommandType::Message).build(),
        ];
        for command in &cmds {
            twilight_validate::command::command(command).expect("invalid command. idiot.");
        }

        let client = self.client.interaction(self.app_id);

        client
            .set_global_commands(&cmds)
            .await
            .expect("Failed to set global commands for bot!");

        let admin_command = AdminCommand::create_command().into();
        twilight_validate::command::command(&admin_command).expect("invalid admin command. idiot.");
        client
            .set_guild_commands(self.control_guild, &[admin_command])
            .await
            .expect("Failed to set admin commands");
    }
}
