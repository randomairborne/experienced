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
    let mut font: Option<String> = None;
    for (key, value) in options {
        if key == "font" {
            font = if let CommandOptionValue::String(fon) = value {
                Some(fon)
            } else {
                None
            };
            continue;
        }
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
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "INSERT INTO custom_card (
            important,
            secondary,
            rank,
            level,
            border,
            background,
            progress_foreground,
            progress_background,
            font,
            id
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
        ) ON CONFLICT (id) DO UPDATE SET
            important = COALESCE(excluded.important, custom_card.important),
            secondary = COALESCE(excluded.secondary, custom_card.secondary),
            rank = COALESCE(excluded.rank, custom_card.rank),
            level = COALESCE(excluded.level, custom_card.level),
            border = COALESCE(excluded.border, custom_card.border),
            background = COALESCE(excluded.background, custom_card.background),
            progress_foreground = COALESCE(excluded.progress_foreground, custom_card.progress_foreground),
            progress_background = COALESCE(excluded.progress_background, custom_card.progress_background),
            font = COALESCE(excluded.font, custom_card.font)",
        important,
        secondary,
        rank,
        level,
        border,
        background,
        progress_foreground,
        progress_background,
        font,
        user.id.get() as i64,
    )
    .execute(&state.db)
    .await?;
    Ok("Updated card!".to_string())
}
async fn process_reset(state: AppState, user: &User) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "DELETE FROM custom_card WHERE id = $1",
        user.id.get() as i64
    )
    .execute(&state.db)
    .await?;
    Ok("Card settings cleared!".to_string())
}
async fn process_fetch(state: AppState, user: &User) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let chosen_font = query!(
        "SELECT font FROM custom_card WHERE id = $1",
        user.id.get() as i64
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(crate::colors::Colors::for_user(&state.db, user.id)
        .await
        .to_string()
        + &chosen_font.map_or_else(
            || "Roboto (default)\n".to_string(),
            |v| v.font.map_or("Roboto (default)\n".to_string(), |v| v),
        ))
}
