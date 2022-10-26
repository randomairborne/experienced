use std::collections::HashMap;
use twilight_model::{
    application::interaction::application_command::{
        CommandData, CommandDataOption, CommandOptionValue,
    },
    channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{processor::CommandProcessorError, AppState};

pub async fn process_anvil<'a>(
    data: &'a CommandData,
    _invoker: &'a User,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    for maybe_group in &data.options {
        if let CommandOptionValue::SubCommandGroup(group) = maybe_group.value {
            match maybe_group.name.as_str() {
                "rewards" => return process_rewards(group, state).await,
                _ => return Err(CommandProcessorError::UnknownSubcommand),
            }
        }
    }
    Err(CommandProcessorError::InvalidSubcommand)
}

async fn process_rewards<'a>(
    options: Vec<CommandDataOption>,
    state: AppState,
) -> Result<InteractionResponseData, CommandProcessorError> {
    for maybe_cmd in options {
        let cmd_name = maybe_cmd.name.clone();
        if let CommandOptionValue::SubCommand(opts) = maybe_cmd.value {
            let args: HashMap<String, CommandOptionValue> =
                opts.iter().map(|val| (val.name, val.value)).collect();
            let contents = match cmd_name.as_str() {
                "add" => process_rewards_add(data, args, state).await,
                "remove" => process_rewards_rm(data, args, state).await,
                "list" => process_rewards_list(data, args, state).await,
                _ => return Err(CommandProcessorError::UnknownSubcommand),
            }?;
            return Ok(InteractionResponseDataBuilder::new()
                .content(contents)
                .flags(MessageFlags::EPHEMERAL)
                .build());
        }
    }
    Err(CommandProcessorError::InvalidSubcommand)
}

async fn process_rewards_add<'a>(
    data: &'a CommandData,
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
) -> Result<String, CommandProcessorError> {
    Ok("".to_string())
}
async fn process_rewards_rm<'a>(
    data: &'a CommandData,
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
) -> Result<String, CommandProcessorError> {
    Ok("".to_string())
}
async fn process_rewards_list<'a>(
    data: &'a CommandData,
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
) -> Result<String, CommandProcessorError> {
    Ok("".to_string())
}
