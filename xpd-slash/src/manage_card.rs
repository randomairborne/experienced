use std::sync::Arc;

use twilight_model::{
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::{
    cmd_defs::{card::CardCommandEdit, CardCommand},
    Error, SlashState, XpdSlashResponse,
};

pub async fn card_update<'a>(
    command: CardCommand,
    invoker: User,
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    let (contents, referenced_user) = match command {
        CardCommand::Reset(_reset) => (process_reset(state, &invoker).await?, Arc::new(invoker)),
        CardCommand::Fetch(fetch) => {
            let fetch_user = Arc::new(fetch.user.map_or(invoker, |user| user.resolved));
            (process_fetch(state, fetch_user.clone()).await?, fetch_user)
        }
        CardCommand::Edit(edit) => (
            process_edit(edit, state, &invoker).await?,
            Arc::new(invoker),
        ),
    };
    #[allow(clippy::cast_possible_wrap)]
    let guild_id = guild_id.get() as i64;
    #[allow(clippy::cast_possible_wrap)]
    let referenced_user_id = referenced_user.id.get() as i64;
    // Select current XP from the database, return 0 if there is no row
    let xp = query!(
        "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
        referenced_user_id,
        guild_id
    )
    .fetch_optional(&state.db)
    .await?
    .map_or(0, |v| v.xp);
    let rank = query!(
        "SELECT COUNT(*) as count FROM levels WHERE xp > $1 AND guild = $2",
        xp,
        guild_id
    )
    .fetch_one(&state.db)
    .await?
    .count
    .unwrap_or(0)
        + 1;
    #[allow(clippy::cast_sign_loss)]
    let level_info = mee6::LevelInfo::new(xp as u64);
    let card = crate::levels::gen_card(state.clone(), referenced_user, level_info, rank).await?;
    let embed = EmbedBuilder::new()
        .description(contents)
        .image(ImageSource::attachment("card.png")?)
        .build();
    Ok(XpdSlashResponse::new().attachments([card]).embeds([embed]))
}

async fn process_edit(
    edit: CardCommandEdit,
    state: &SlashState,
    user: &User,
) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "INSERT INTO custom_card (
            username,
            rank,
            level,
            border,
            background,
            progress_foreground,
            progress_background,
            foreground_xp_count,
            background_xp_count,
            font,
            toy_image,
            card_layout,
            id
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, COALESCE($12, 'classic.svg'), $13
        ) ON CONFLICT (id) DO UPDATE SET
            username = COALESCE($1, custom_card.username),
            rank = COALESCE($2, custom_card.rank),
            level = COALESCE($3, custom_card.level),
            border = COALESCE($4, custom_card.border),
            background = COALESCE($5, custom_card.background),
            progress_foreground = COALESCE($6, custom_card.progress_foreground),
            progress_background = COALESCE($7, custom_card.progress_background),
            foreground_xp_count = COALESCE($8, custom_card.foreground_xp_count),
            background_xp_count = COALESCE($9, custom_card.background_xp_count),
            font = COALESCE($10, custom_card.font),
            toy_image = COALESCE($11, custom_card.toy_image),
            card_layout = COALESCE($12, custom_card.card_layout)",
        edit.username.map(|v| v.to_string()),
        edit.rank.map(|v| v.to_string()),
        edit.level.map(|v| v.to_string()),
        edit.border.map(|v| v.to_string()),
        edit.background.map(|v| v.to_string()),
        edit.progress_foreground.map(|v| v.to_string()),
        edit.progress_background.map(|v| v.to_string()),
        edit.foreground_xp_count.map(|v| v.to_string()),
        edit.background_xp_count.map(|v| v.to_string()),
        edit.font.map(|v| v.value()),
        edit.toy_image.map(|v| v.value()),
        edit.card_layout.map(|v| v.value()),
        user.id.get() as i64,
    )
    .execute(&state.db)
    .await?;

    Ok("Updated card!".to_string())
}

async fn process_reset(state: &SlashState, user: &User) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "DELETE FROM custom_card WHERE id = $1",
        user.id.get() as i64
    )
    .execute(&state.db)
    .await?;
    Ok("Card settings cleared!".to_string())
}

async fn process_fetch(state: &SlashState, user: Arc<User>) -> Result<String, Error> {
    Ok(crate::levels::get_customizations(state.clone(), user)
        .await?
        .to_string())
}
