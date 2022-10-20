use crate::AppState;
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
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
    Ok(InteractionResponseDataBuilder::new().build())
}

#[derive(Debug, thiserror::Error)]
pub enum CommandProcessorError {}
