use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::command::CommandType;
use twilight_util::builder::command::CommandBuilder;

use crate::{
    cmd_defs::{
        admin::AdminCommand,
        card::{CardCommand, GuildCardCommand},
        config::ConfigCommand,
        experience::XpCommand,
        gdpr::GdprCommand,
        levels::{LeaderboardCommand, RankCommand},
        manage::ManageCommand,
    },
    SlashState,
};

pub mod admin;
pub mod card;
pub mod config;
pub mod experience;
pub mod gdpr;
pub mod levels;
pub mod manage;
pub mod rewards;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "help",
    desc = "Learn about how to use experienced",
    dm_permission = true
)]
pub struct HelpCommand;

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
            ManageCommand::create_command().into(),
            ConfigCommand::create_command().into(),
            GuildCardCommand::create_command().into(),
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
