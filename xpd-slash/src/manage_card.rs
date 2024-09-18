use mee6::LevelInfo;
use twilight_model::id::{
    marker::{GenericMarker, GuildMarker},
    Id,
};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};
use xpd_common::MemberDisplayInfo;
use xpd_database::{CardUpdate, Database};
use xpd_rank_card::ConfigItem;

use crate::{
    cmd_defs::{
        card::{CardCommandEdit, ColorOption},
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
    let (contents, target) = match command {
        CardCommand::Reset(_reset) => (process_reset(state, invoker.id.cast()).await?, invoker),
        CardCommand::Fetch(fetch) => {
            let target = fetch
                .user
                .map_or(invoker, |v| MemberDisplayInfo::from(v.resolved));
            let contents = if let Some(guild_id) = guild_id {
                process_fetch(state, &[target.id.cast(), guild_id.cast()]).await
            } else {
                process_fetch(state, &[target.id.cast()]).await
            }?;
            (contents, target)
        }
        CardCommand::Edit(edit) => (process_edit(edit, state, invoker.id.cast()).await?, invoker),
    };
    let user_stats = if let Some(id) = guild_id {
        state.get_user_stats(target.id, id).await?
    } else {
        // I am so mature.
        UserStats { xp: 420, rank: 69 }
    };
    let level_info = LevelInfo::new(u64::try_from(user_stats.xp).unwrap_or(0));
    let card =
        crate::levels::gen_card(state.clone(), target, guild_id, level_info, user_stats.rank)
            .await?;
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
    let card = crate::levels::gen_card(
        state.clone(),
        referenced_user,
        Some(guild_id),
        level_info,
        127,
    )
    .await?;
    let embed = EmbedBuilder::new()
        .description(contents)
        .image(ImageSource::attachment("card.png")?)
        .build();
    Ok(XpdSlashResponse::new()
        .ephemeral(true)
        .attachments([card])
        .embeds([embed]))
}

fn process_edit_helper(
    items: &[ConfigItem],
    field: Option<String>,
    error: Error,
) -> Result<Option<String>, Error> {
    field
        .map(|chosen| {
            items
                .iter()
                .find_map(|ci| matches_config_item(ci, &chosen))
                .ok_or(error)
        })
        .transpose()
        .map(|v| match v.as_deref() {
            Some(CUSTOM_CARD_NULL_SENTINEL) | None => None,
            Some(_) => v,
        })
}

pub const CUSTOM_CARD_NULL_SENTINEL: &str = "NULL";

async fn process_edit(
    edit: CardCommandEdit,
    state: &SlashState,
    id: Id<GenericMarker>,
) -> Result<String, Error> {
    let items = state.svg.config();
    let toy_image = process_edit_helper(&items.toys, edit.toy_image, Error::UnknownToy)?;
    let card_layout = process_edit_helper(&items.cards, edit.card_layout, Error::UnknownCard)?;
    let font = process_edit_helper(&items.fonts, edit.font, Error::UnknownFont)?;

    let update = CardUpdate {
        username: edit.username.map(ColorOption::string),
        rank: edit.rank.map(ColorOption::string),
        level: edit.level.map(ColorOption::string),
        border: edit.border.map(ColorOption::string),
        background: edit.background.map(ColorOption::string),
        progress_background: edit.progress_background.map(ColorOption::string),
        progress_foreground: edit.progress_foreground.map(ColorOption::string),
        foreground_xp_count: edit.foreground_xp_count.map(ColorOption::string),
        background_xp_count: edit.background_xp_count.map(ColorOption::string),
        font,
        toy_image,
        card_layout,
        card_layout_default: "classic.svg".to_string(),
    };

    xpd_database::update_card(&state.db, id, &update).await?;

    Ok("Updated card!".to_string())
}

fn matches_config_item(ci: &ConfigItem, choice: &str) -> Option<String> {
    if ci.internal_name == choice {
        Some(ci.internal_name.clone())
    } else {
        None
    }
}

async fn process_reset(state: &SlashState, id: Id<GenericMarker>) -> Result<String, Error> {
    xpd_database::delete_card_customizations(&state.db, id).await?;
    Ok("Card settings cleared!".to_string())
}

async fn process_fetch(state: &SlashState, ids: &[Id<GenericMarker>]) -> Result<String, Error> {
    Ok(crate::levels::get_customizations(state, ids)
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
