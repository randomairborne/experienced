use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};
use twilight_model::{application::command::CommandType, guild::Permissions};
use twilight_util::builder::command::CommandBuilder;

use crate::SlashState;

pub mod admin;
pub mod card;
pub mod gdpr;
pub mod manage;

#[derive(CommandModel, CreateCommand)]
#[command(name = "help", desc = "Learn about how to use experienced")]
pub struct HelpCommand;

#[derive(CommandModel, CreateCommand)]
#[command(name = "leaderboard", desc = "See the leaderboard for this server")]
pub struct LeaderboardCommand;

#[derive(CommandModel, CreateCommand)]
#[command(name = "rank", desc = "Check someone's rank and level")]
pub struct RankCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "admin", desc = "Globally manage the bot")]
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
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "card",
    desc = "Set hex codes for different color schemes in your rank card."
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
    name = "guild-card",
    desc = "Set hex codes for different color schemes in your server's rank card."
)]
#[allow(clippy::large_enum_variant)]
pub enum GuildCardCommand {
    #[command(name = "reset")]
    Reset(card::CardCommandReset),
    #[command(name = "fetch")]
    Fetch(card::CardCommandFetch),
    #[command(name = "edit")]
    Edit(card::CardCommandEdit),
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
            LeaderboardCommand::create_command().into(),
            CommandBuilder::new("Get level", "", CommandType::User).build(),
            CommandBuilder::new("Get author level", "", CommandType::Message).build(),
        ];
        self.client
            .interaction(self.my_id)
            .set_global_commands(&cmds)
            .await
            .expect("Failed to set global commands for bot!");
        self.client
            .interaction(self.my_id)
            .set_guild_commands(self.control_guild, &[AdminCommand::create_command().into()])
            .await
            .expect("Failed to set admin commands");
    }
}
