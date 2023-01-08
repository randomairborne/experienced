use sqlx::query;
use std::collections::HashMap;
use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{colors::Color, manager::Error, AppState};

pub async fn process_colors(
    data: CommandData,
    invoker: &User,
    state: AppState,
) -> Result<InteractionResponseData, Error> {
    for maybe_cmd in data.options {
        if let CommandOptionValue::SubCommand(opts) = maybe_cmd.value {
            let args: HashMap<String, CommandOptionValue> =
                opts.into_iter().map(|val| (val.name, val.value)).collect();
            let contents = match maybe_cmd.name.as_str() {
                "edit" => process_edit(args, state, invoker).await,
                "reset" => process_reset(state, invoker).await,
                "fetch" => process_fetch(state, invoker).await,
                _ => return Err(Error::UnknownSubcommand),
            }?;
            return Ok(InteractionResponseDataBuilder::new()
                .content(contents)
                .flags(MessageFlags::EPHEMERAL)
                .build());
        }
    }
    Err(Error::InvalidSubcommand)
}

async fn process_edit(
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
    user: &User,
) -> Result<String, Error> {
    let mut opts = HashMap::with_capacity(options.len());
    for (key, value) in options {
        let val = match value {
            CommandOptionValue::String(s) => Color::from_hex(&s)?.to_string(),
            _ => Color::new(0, 0, 0).to_string(),
        };
        opts.insert(key, val);
    }
    let important = opts.get("important");
    let secondary = opts.get("secondary");
    let rank = opts.get("rank");
    let level = opts.get("level");
    let border = opts.get("border");
    let background = opts.get("background");
    let progress_foreground = opts.get("progress_foreground");
    let progress_background = opts.get("progress_background");
    query!(
        "INSERT INTO custom_colors (
            important,
            secondary,
            `rank`,
            level,
            border,
            background,
            progress_foreground,
            progress_background,
            id
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?, ?, ?
        ) ON DUPLICATE KEY UPDATE
            important = COALESCE(?, important),
            secondary = COALESCE(?, secondary),
            rank = COALESCE(?, rank),
            level = COALESCE(?, level),
            border = COALESCE(?, border),
            background = COALESCE(?, background),
            progress_foreground = COALESCE(?, progress_foreground),
            progress_background = COALESCE(?, progress_background)",
        important,
        secondary,
        rank,
        level,
        border,
        background,
        progress_foreground,
        progress_background,
        user.id.get(),
        important,
        secondary,
        rank,
        level,
        border,
        background,
        progress_foreground,
        progress_background
    )
    .execute(&state.db)
    .await?;
    Ok("Updated".to_string())
}
async fn process_reset(state: AppState, user: &User) -> Result<String, Error> {
    query!("DELETE FROM custom_colors WHERE id = ?", user.id.get())
        .execute(&state.db)
        .await?;
    Ok("Card settings cleared!".to_string())
}
async fn process_fetch(state: AppState, user: &User) -> Result<String, Error> {
    Ok(crate::colors::Colors::for_user(&state.db, user.id)
        .await
        .to_string())
}
