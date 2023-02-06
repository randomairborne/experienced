use sqlx::query;
use twilight_model::{
    channel::message::MessageFlags, http::interaction::InteractionResponseData, user::User,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::{
    cmd_defs::{card::CardCommandEdit, CardCommand},
    manager::Error,
    AppState,
};

pub async fn process_colors(
    data: CardCommand,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponseData, Error> {
    let contents = match data {
        CardCommand::Reset(_reset) => process_reset(state, &invoker).await,
        CardCommand::Fetch(fetch) => {
            process_fetch(state, &fetch.user.map_or_else(|| invoker, |v| v.resolved)).await
        }
        CardCommand::Edit(edit) => process_edit(edit, state, &invoker).await,
    }?;
    Ok(InteractionResponseDataBuilder::new()
        .flags(MessageFlags::EPHEMERAL)
        .embeds([EmbedBuilder::new().description(contents).build()])
        .build())
}

async fn process_edit(
    edit: CardCommandEdit,
    state: AppState,
    user: &User,
) -> Result<String, Error> {
    let toy_image = edit.toy_image.and_then(|v| match v {
        crate::cmd_defs::card::CardCommandEditToy::None => None,
        _ => Some(v.value()),
    });
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
            toy_image,
            id
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
        ) ON CONFLICT (id) DO UPDATE SET
            important = COALESCE(excluded.important, custom_card.important),
            secondary = COALESCE(excluded.secondary, custom_card.secondary),
            rank = COALESCE(excluded.rank, custom_card.rank),
            level = COALESCE(excluded.level, custom_card.level),
            border = COALESCE(excluded.border, custom_card.border),
            background = COALESCE(excluded.background, custom_card.background),
            progress_foreground = COALESCE(excluded.progress_foreground, custom_card.progress_foreground),
            progress_background = COALESCE(excluded.progress_background, custom_card.progress_background),
            font = COALESCE(excluded.font, custom_card.font),
            toy_image = COALESCE(excluded.toy_image, custom_card.toy_image)",
        edit.important.map(|v| v.to_string()),
        edit.secondary.map(|v| v.to_string()),
        edit.rank.map(|v| v.to_string()),
        edit.level.map(|v| v.to_string()),
        edit.border.map(|v| v.to_string()),
        edit.background.map(|v| v.to_string()),
        edit.progress_foreground.map(|v| v.to_string()),
        edit.progress_background.map(|v| v.to_string()),
        edit.font.map(|v| v.value()),
        toy_image,
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
        + "Font: "
        + &chosen_font.map_or_else(
            || "`Roboto` (default)\n".to_string(),
            |v| v.font.map_or("`Roboto` (default)\n".to_string(), |v| v),
        ))
}
