pub mod card;
pub mod manage;
use twilight_model::{application::command::CommandType, guild::Permissions};
use twilight_util::builder::command::CommandBuilder;

use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rank",
    desc = "Check someone's rank and level",
    dm_permission = false
)]
pub struct RankCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "card",
    desc = "Set hex codes for different color schemes in your rank card.",
    dm_permission = true
)]
pub enum CardCommand {
    #[command(name = "reset")]
    Reset(card::CommandReset),
    #[command(name = "fetch")]
    Fetch(card::CommandFetch),
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

pub async fn register(http: twilight_http::client::InteractionClient<'_>) {
    let cmds: [twilight_model::application::command::Command; 5] = [
        RankCommand::create_command().into(),
        CardCommand::create_command().into(),
        XpCommand::create_command().into(),
        CommandBuilder::new("Get level", "", CommandType::User).build(),
        CommandBuilder::new("Get author level", "", CommandType::Message).build(),
    ];
    http.set_global_commands(&cmds)
        .await
        .expect("Failed to set global commands for bot!");
}
