use std::fmt::Display;

use crate::AppState;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{
            application_command::{CommandData, CommandOptionValue},
            Interaction, InteractionData, InteractionType,
        },
    },
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::UserMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub fn process(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    Ok(if interaction.kind == InteractionType::ApplicationCommand {
        InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(process_app_cmd(interaction, state)?),
        }
    } else {
        InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        }
    })
}

fn process_app_cmd(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    let author_id = interaction.author_id();
    let data = if let Some(data) = interaction.data {
        if let InteractionData::ApplicationCommand(cmd) = data {
            *cmd
        } else {
            return err("This bot does not support ModalSubmit or MessageComponent interactions!");
        }
    } else {
        return err("Discord didn't send interaction data!");
    };
    let author_id = match data.kind {
        CommandType::ChatInput => process_slash_cmd(data, author_id),
        CommandType::User => process_user_cmd(data),
        CommandType::Message => process_msg_cmd(data),
        _ => return err("Discord sent unknown kind of interaction!"),
    }?;
    Ok(InteractionResponseDataBuilder::new()
        .flags(MessageFlags::EPHEMERAL)
        .content(format!("ID of user to check level of: {author_id}"))
        .build())
}

fn process_slash_cmd(
    data: CommandData,
    invoker: Option<Id<UserMarker>>,
) -> Result<Id<UserMarker>, CommandProcessorError> {
    if &data.name != "level" && &data.name != "rank" {
        return Err(CommandProcessorError::UnrecognizedCommand);
    };
    for option in data.options {
        if option.name == "user" {
            if let CommandOptionValue::User(val) = option.value {
                return Ok(val);
            };
        }
    }
    if let Some(val) = invoker {
        Ok(val)
    } else {
        Err(CommandProcessorError::NoInvokerId)
    }
}

fn process_msg_cmd(data: CommandData) -> Result<Id<UserMarker>, CommandProcessorError> {
    Err(CommandProcessorError::NoInvokerId)
}

fn process_user_cmd(data: CommandData) -> Result<Id<UserMarker>, CommandProcessorError> {
    Err(CommandProcessorError::NoInvokerId)
}
#[derive(Debug, thiserror::Error)]
pub enum CommandProcessorError {
    #[error("Discord sent a command that is not known!")]
    UnrecognizedCommand,
    #[error("Discord did not send a user ID for the command invoker when it was required!")]
    NoInvokerId,
}

fn err(msg: impl Display) -> Result<InteractionResponseData, CommandProcessorError> {
    Ok(InteractionResponseDataBuilder::new()
        .content(format!("Oops! {msg}"))
        .flags(MessageFlags::EPHEMERAL)
        .build())
}
