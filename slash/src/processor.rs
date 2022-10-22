use std::fmt::Display;

use crate::AppState;
use sqlx::query;
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

pub async fn process(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    Ok(if interaction.kind == InteractionType::ApplicationCommand {
        InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(process_app_cmd(interaction, state).await?),
        }
    } else {
        InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        }
    })
}

async fn process_app_cmd(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    let invoker_id = interaction.author_id();
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
        CommandType::ChatInput => process_slash_cmd(data, invoker_id),
        CommandType::User => process_user_cmd(&data),
        CommandType::Message => process_msg_cmd(&data),
        _ => return err("Discord sent unknown kind of interaction!"),
    }?;
    get_level(author_id, invoker_id, state).await
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
    invoker.ok_or(CommandProcessorError::NoInvokerId)
}

fn process_msg_cmd(data: &CommandData) -> Result<Id<UserMarker>, CommandProcessorError> {
    let msg_id = data
        .target_id
        .ok_or(CommandProcessorError::NoMessageTargetId)?;
    Ok(data
        .resolved
        .as_ref()
        .ok_or(CommandProcessorError::NoResolvedData)?
        .messages
        .get(&msg_id.cast())
        .ok_or(CommandProcessorError::NoInvokerId)?
        .author
        .id)
}

const fn process_user_cmd(data: &CommandData) -> Result<Id<UserMarker>, CommandProcessorError> {
    if let Some(target_id) = data.target_id {
        return Ok(target_id.cast());
    }
    Err(CommandProcessorError::NoInvokerId)
}

async fn get_level(
    user: Id<UserMarker>,
    invoker: Option<Id<UserMarker>>,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    // Select current XP from the database, return 0 if there is no row
    let xp = match query!("SELECT xp FROM levels WHERE id = ?", user.to_string())
        .fetch_one(&state.db)
        .await
    {
        Ok(val) => val.xp,
        Err(e) => match e {
            sqlx::Error::RowNotFound => 0,
            _ => Err(e)?,
        },
    };
    let content: String;
    if let Some(invoker) = invoker {
        if invoker == user {
            if xp == 0 {
                content =
                    "You aren't ranked yet, because you haven't sent any messages!".to_string();
            } else {
                content = format!("You have {xp} xp.");
            };
        } else if xp == 0 {
            content =
                "This user isn't ranked yet, because they haven't sent any messages!".to_string();
        } else {
            content = format!("This user has {xp} xp.");
        }
    } else if xp == 0 {
        content = "This user isn't ranked yet, because they haven't sent any messages!".to_string();
    } else {
        content = format!("This user has {xp} xp.");
    }
    Ok(InteractionResponseDataBuilder::new()
        .flags(MessageFlags::EPHEMERAL)
        .content(content)
        .build())
}

#[derive(Debug, thiserror::Error)]
pub enum CommandProcessorError {
    #[error("Discord sent a command that is not known!")]
    UnrecognizedCommand,
    #[error("Discord did not send a user ID for the command invoker when it was required!")]
    NoInvokerId,
    #[error("Discord did not send part of the Resolved Data!")]
    NoResolvedData,
    #[error("Discord did not send target ID for message!")]
    NoMessageTargetId,
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[allow(clippy::unnecessary_wraps)]
fn err(msg: impl Display) -> Result<InteractionResponseData, CommandProcessorError> {
    Ok(InteractionResponseDataBuilder::new()
        .content(format!("Oops! {msg}"))
        .flags(MessageFlags::EPHEMERAL)
        .build())
}
