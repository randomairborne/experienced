use std::fmt::Display;

use crate::AppState;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{
            application_command::CommandData, Interaction, InteractionData, InteractionType,
        },
    },
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData},
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
    let data = if let Some(data) = interaction.data {
        if let InteractionData::ApplicationCommand(cmd) = data {
            *cmd
        } else {
            return err("This bot does not support ModalSubmit or MessageComponent interactions!");
        }
    } else {
        return err("Discord didn't send interaction data!");
    };
    match data.kind {
        CommandType::ChatInput => process_slash_cmd(data),
        CommandType::User => process_user_cmd(data),
        CommandType::Message => process_msg_cmd(data),
        _ => err("Discord sent unknown kind of interaction!"),
    }
}

fn process_slash_cmd(data: CommandData) -> Result<InteractionResponseData, CommandProcessorError> {

}

fn process_msg_cmd(data: CommandData) -> Result<InteractionResponseData, CommandProcessorError> {}

fn process_user_cmd(data: CommandData) -> Result<InteractionResponseData, CommandProcessorError> {}

#[derive(Debug, thiserror::Error)]
pub enum CommandProcessorError {}

fn err(msg: impl Display) -> Result<InteractionResponseData, CommandProcessorError> {
    Ok(InteractionResponseDataBuilder::new()
        .content(format!("Oops! {msg}"))
        .flags(MessageFlags::EPHEMERAL)
        .build())
}
