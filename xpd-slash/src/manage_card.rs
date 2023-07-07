use sqlx::query;
use twilight_model::{
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::{
    cmd_defs::{card::CardCommandEdit, CardCommand},
    Error, SlashState, XpdSlashResponse,
};

pub async fn card_update(
    data: CardCommand,
    user: User,
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let user_id = user.id.get() as i64;
    let contents = match data {
        CardCommand::Reset(_reset) => process_reset(state, &user).await?,
        CardCommand::Fetch(fetch) => {
            let user = fetch.user.as_ref().map_or(&user, |user| &user.resolved);
            process_fetch(state, user).await?
        }
        CardCommand::Edit(edit) => process_edit(edit, state, &user).await?,
    };
    #[allow(clippy::cast_possible_wrap)]
    let guild_id = guild_id.get() as i64;
    // Select current XP from the database, return 0 if there is no row
    let xp = query!(
        "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
        user_id,
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
    let card = crate::levels::gen_card(state, &user, level_info, rank).await?;
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
            id
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
        ) ON CONFLICT (id) DO UPDATE SET
            username = COALESCE(excluded.username, custom_card.username),
            rank = COALESCE(excluded.rank, custom_card.rank),
            level = COALESCE(excluded.level, custom_card.level),
            border = COALESCE(excluded.border, custom_card.border),
            background = COALESCE(excluded.background, custom_card.background),
            progress_foreground = COALESCE(excluded.progress_foreground, custom_card.progress_foreground),
            progress_background = COALESCE(excluded.progress_background, custom_card.progress_background),
            font = COALESCE(excluded.font, custom_card.font),
            toy_image = COALESCE(excluded.toy_image, custom_card.toy_image)",
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

async fn process_fetch(state: &SlashState, user: &User) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let chosen_font = query!(
        "SELECT font FROM custom_card WHERE id = $1",
        user.id.get() as i64
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(crate::colors::for_user(&state.db, user.id)
        .await
        .to_string()
        + "Font: "
        + &chosen_font.map_or_else(
            || "`Roboto` (default)\n".to_string(),
            |v| v.font.map_or("`Roboto` (default)\n".to_string(), |v| v),
        ))
}
