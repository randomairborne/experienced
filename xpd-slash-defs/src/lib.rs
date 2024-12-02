pub mod admin;
pub mod card;
pub mod config;
pub mod experience;
pub mod gdpr;
pub mod levels;
pub mod manage;
pub mod rewards;

use admin::AdminCommand;
use rewards::RewardsCommand;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::command::{Command, CommandType},
    id::Id,
};

use crate::{
    card::{CardCommand, GuildCardCommand},
    config::ConfigCommand,
    experience::XpCommand,
    gdpr::GdprCommand,
    levels::{LeaderboardCommand, RankCommand},
    manage::ManageCommand,
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "help",
    desc = "Learn about how to use experienced",
    dm_permission = true
)]
pub struct HelpCommand;

pub fn get_commands() -> Vec<Command> {
    vec![
        XpCommand::create_command().into(),
        RankCommand::create_command().into(),
        CardCommand::create_command().into(),
        HelpCommand::create_command().into(),
        GdprCommand::create_command().into(),
        ManageCommand::create_command().into(),
        ConfigCommand::create_command().into(),
        GuildCardCommand::create_command().into(),
        LeaderboardCommand::create_command().into(),
        RewardsCommand::create_command().into(),
        context_cmd("Get level", CommandType::User),
        context_cmd("Get author level", CommandType::Message),
    ]
}

pub fn admin_commands() -> Vec<Command> {
    vec![AdminCommand::create_command().into()]
}

fn context_cmd(name: impl Into<String>, kind: CommandType) -> Command {
    Command {
        name: name.into(),
        kind,
        application_id: None,
        default_member_permissions: None,
        dm_permission: None,
        description: String::new(),
        description_localizations: None,
        guild_id: None,
        id: None,
        name_localizations: None,
        nsfw: None,
        options: Vec::new(),
        version: Id::new(1),
    }
}

#[test]
fn ensure_limits_match() {
    use twilight_model::application::command::CommandOptionValue;
    let cmd = ConfigCommand::create_command();
    eprintln!("{cmd:?}");
    let levels_cmd = cmd.options.iter().find(|v| v.name == "levels").unwrap();
    let levels_cmd_opts = levels_cmd.options.as_ref().unwrap();
    let cooldown_value = levels_cmd_opts
        .iter()
        .find(|v| v.name == "message_cooldown")
        .unwrap()
        .max_value
        .unwrap();
    assert_eq!(
        cooldown_value,
        CommandOptionValue::Integer(xpd_common::MAX_MESSAGE_COOLDOWN.into())
    );
}

#[test]
fn validate_commands() {
    for command in get_commands().iter().chain(admin_commands().iter()) {
        twilight_validate::command::command(command)
            .unwrap_or_else(|e| panic!("Command {} is invalid: {e}", command.name));
    }
}
