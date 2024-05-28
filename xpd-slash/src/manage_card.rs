use mee6::LevelInfo;
use twilight_model::id::{
    marker::{GenericMarker, GuildMarker},
    Id,
};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};
use xpd_common::{id_to_db, MemberDisplayInfo};

use crate::{
    cmd_defs::{
        card::{CardCommandEdit, CardCommandEditFont, ColorOption},
        CardCommand, GuildCardCommand,
    },
    Error, SlashState, UserStats, XpdSlashResponse,
};

pub async fn user_card_update(
    command: CardCommand,
    invoker: MemberDisplayInfo,
    state: &SlashState,
    guild_id: Option<Id<GuildMarker>>,
) -> Result<XpdSlashResponse, Error> {
    let contents = match command {
        CardCommand::Reset(_reset) => process_reset(state, invoker.id.cast()).await?,
        CardCommand::Fetch(_fetch) => {
            if let Some(guild_id) = guild_id {
                process_fetch(state, &[invoker.id.cast(), guild_id.cast()]).await
            } else {
                process_fetch(state, &[invoker.id.cast()]).await
            }?
        }
        CardCommand::Edit(edit) => process_edit(edit, state, invoker.id.cast()).await?,
    };
    let user_stats = if let Some(id) = guild_id {
        state.get_user_stats(invoker.id, id).await?
    } else {
        // I am so mature.
        UserStats { xp: 420, rank: 69 }
    };
    let level_info = LevelInfo::new(u64::try_from(user_stats.xp).unwrap_or(0));
    let card = crate::levels::gen_card(state.clone(), invoker, level_info, user_stats.rank).await?;
    let embed = EmbedBuilder::new()
        .description(contents)
        .image(ImageSource::attachment("card.png")?)
        .build();
    Ok(XpdSlashResponse::new()
        .attachments([card])
        .ephemeral(true)
        .embeds([embed]))
}

pub async fn guild_card_update(
    command: GuildCardCommand,
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    let contents = match command {
        GuildCardCommand::Reset(_reset) => process_reset(state, guild_id.cast()).await?,
        GuildCardCommand::Fetch(_fetch) => process_fetch(state, &[guild_id.cast()]).await?,
        GuildCardCommand::Edit(edit) => process_edit(edit, state, guild_id.cast()).await?,
    };
    let referenced_user = fake_user(guild_id.cast());
    let level_info = LevelInfo::new(40);
    let card = crate::levels::gen_card(state.clone(), referenced_user, level_info, 127).await?;
    let embed = EmbedBuilder::new()
        .description(contents)
        .image(ImageSource::attachment("card.png")?)
        .build();
    Ok(XpdSlashResponse::new()
        .ephemeral(true)
        .attachments([card])
        .embeds([embed]))
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

fn fake_user(id: Id<GenericMarker>) -> MemberDisplayInfo {
    MemberDisplayInfo {
        id: id.cast(),
        name: "Preview".to_string(),
        global_name: None,
        nick: None,
        avatar: None,
        local_avatar: None,
        bot: false,
    }
}
