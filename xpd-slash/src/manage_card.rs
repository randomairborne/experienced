use std::sync::Arc;

use twilight_model::{
    id::{
        marker::{GenericMarker, GuildMarker},
        Id,
    },
    user::User,
};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};
use xpd_common::id_to_db;

use crate::{
    cmd_defs::{
        card::{CardCommandEdit, CardCommandEditFont, ColorOption},
        CardCommand,
    },
    Error, SlashState, XpdSlashResponse,
};

pub async fn card_update<'a>(
    command: CardCommand,
    invoker: Option<User>,
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    // if let else takes self
    #[allow(clippy::option_if_let_else)]
    let target_id = if let Some(invoker) = &invoker {
        invoker.id.cast()
    } else {
        guild_id.cast()
    };
    let contents = match command {
        CardCommand::Reset(_reset) => process_reset(state, target_id).await?,
        CardCommand::Fetch(_fetch) => process_fetch(state, &[target_id, guild_id.cast()]).await?,
        CardCommand::Edit(edit) => process_edit(edit, state, target_id).await?,
    };
    let referenced_user = Arc::new(invoker.unwrap_or_else(fake_user));
    // Select current XP from the database, return 0 if there is no row
    let xp = query!(
        "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
        id_to_db(referenced_user.id),
        id_to_db(guild_id)
    )
    .fetch_optional(&state.db)
    .await?
    .map_or(0, |v| v.xp);
    let rank = query!(
        "SELECT COUNT(*) as count FROM levels WHERE xp > $1 AND guild = $2",
        xp,
        id_to_db(guild_id)
    )
    .fetch_one(&state.db)
    .await?
    .count
    .unwrap_or(0)
        + 1;
    #[allow(clippy::cast_sign_loss)]
    let level_info = mee6::LevelInfo::new(u64::try_from(xp).unwrap_or(0));
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
    id: Id<GenericMarker>,
) -> Result<String, Error> {
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
        edit.username.map(ColorOption::string),
        edit.rank.map(ColorOption::string),
        edit.level.map(ColorOption::string),
        edit.border.map(ColorOption::string),
        edit.background.map(ColorOption::string),
        edit.progress_foreground.map(ColorOption::string),
        edit.progress_background.map(ColorOption::string),
        edit.foreground_xp_count.map(ColorOption::string),
        edit.background_xp_count.map(ColorOption::string),
        edit.font
            .unwrap_or(CardCommandEditFont::Roboto)
            .as_xpd_rank_card()
            .to_string(),
        edit.toy_image.map(|v| v.value()),
        edit.card_layout.map(|v| v.value()),
        id_to_db(id),
    )
    .execute(&state.db)
    .await?;

    Ok("Updated card!".to_string())
}

async fn process_reset(state: &SlashState, id: Id<GenericMarker>) -> Result<String, Error> {
    query!("DELETE FROM custom_card WHERE id = $1", id_to_db(id))
        .execute(&state.db)
        .await?;
    Ok("Card settings cleared!".to_string())
}

async fn process_fetch(state: &SlashState, ids: &[Id<GenericMarker>]) -> Result<String, Error> {
    Ok(crate::levels::get_customizations(state.clone(), ids)
        .await?
        .to_string())
}

fn fake_user() -> User {
    User {
        accent_color: None,
        avatar: None,
        avatar_decoration: None,
        banner: None,
        bot: false,
        discriminator: 0,
        email: None,
        flags: None,
        global_name: None,
        id: Id::new(1),
        locale: None,
        mfa_enabled: None,
        name: "Preview".to_string(),
        premium_type: None,
        public_flags: None,
        system: None,
        verified: None,
    }
}
