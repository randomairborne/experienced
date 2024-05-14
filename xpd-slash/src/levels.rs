use std::sync::Arc;

use base64::Engine;
use tokio::try_join;
use twilight_model::{
    channel::message::MessageFlags,
    http::attachment::Attachment,
    id::{
        marker::{GenericMarker, GuildMarker, UserMarker},
        Id,
    },
    user::User,
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::{id_to_db, Tag};
use xpd_rank_card::{
    cards::Card,
    customizations::{Color, Customizations},
    Font, Toy,
};

use crate::{Error, SlashState, XpdSlashResponse};

pub async fn get_level(
    guild_id: Id<GuildMarker>,
    user: User,
    invoker: Id<UserMarker>,
    showoff: Option<bool>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let rankstats = state.get_user_stats(invoker, guild_id).await?;
    let flags = if showoff.is_some_and(|v| v) {
        MessageFlags::empty()
    } else {
        MessageFlags::EPHEMERAL
    };

    let level_info = mee6::LevelInfo::new(u64::try_from(rankstats.xp).unwrap_or(0));
    let content = if user.bot {
        "Bots aren't ranked, that would be silly!".to_string()
    } else if invoker == user.id {
        if rankstats.xp == 0 {
            "You aren't ranked yet, because you haven't sent any messages!".to_string()
        } else {
            return generate_level_response(&state, user, level_info, rankstats.rank, flags).await;
        }
    } else if rankstats.xp == 0 {
        format!(
            "{} isn't ranked yet, because they haven't sent any messages!",
            user.tag()
        )
    } else {
        return generate_level_response(&state, user, level_info, rankstats.rank, flags).await;
    };
    let embed = EmbedBuilder::new().description(content).build();
    Ok(XpdSlashResponse::new().embeds([embed]).flags(flags))
}

async fn generate_level_response(
    state: &SlashState,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
    flags: MessageFlags,
) -> Result<XpdSlashResponse, Error> {
    let card = gen_card(state.clone(), Arc::new(user), level_info, rank).await?;
    Ok(XpdSlashResponse::new().attachments([card]).flags(flags))
}

pub async fn gen_card(
    state: SlashState,
    user: Arc<User>,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<Attachment, Error> {
    let customizations_future =
        async { get_customizations(state.clone(), &[user.id.cast()]).await };
    let avatar_future = get_avatar(state.clone(), user.clone());
    let (customizations, avatar) = try_join!(customizations_future, avatar_future)?;
    let discriminator = if user.discriminator == 0 {
        None
    } else {
        Some(user.discriminator().to_string())
    };
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    let percentage = (level_info.percentage() * 100.0).round() as u64;
    let png = state
        .svg
        .render(xpd_rank_card::Context {
            level: level_info.level(),
            rank,
            name: user.name.clone(),
            discriminator,
            percentage,
            current: level_info.xp(),
            needed: mee6::xp_needed_for_level(level_info.level() + 1),
            customizations,
            avatar,
        })
        .await?;
    Ok(Attachment {
        description: Some(format!(
            "{} is level {} (rank #{}), and is {}% of the way to level {}.",
            user.tag(),
            level_info.level(),
            rank,
            (level_info.percentage() * 100.0).round(),
            level_info.level() + 1
        )),
        file: png,
        filename: "card.png".to_string(),
        id: 0,
    })
}

pub async fn get_customizations(
    state: SlashState,
    ids: &[Id<GenericMarker>],
) -> Result<Customizations, Error> {
    let mut customizations = None;
    for id in ids {
        if let Some(custom_params) =
            query!("SELECT * FROM custom_card WHERE id = $1", id_to_db(*id))
                .fetch_optional(&state.db)
                .await?
        {
            customizations = Some(custom_params);
            break;
        }
    }
    let Some(customizations) = customizations else {
        return Ok(Card::default().default_customizations());
    };
    let card = Card::from_name(&customizations.card_layout).ok_or(Error::InvalidCard)?;
    let defaults = card.default_customizations();
    Ok(Customizations {
        username: color_or_default(&customizations.username, defaults.username)?,
        rank: color_or_default(&customizations.rank, defaults.rank)?,
        level: color_or_default(&customizations.level, defaults.level)?,
        border: color_or_default(&customizations.border, defaults.border)?,
        background: color_or_default(&customizations.background, defaults.background)?,
        progress_foreground: color_or_default(
            &customizations.progress_foreground,
            defaults.progress_foreground,
        )?,
        progress_background: color_or_default(
            &customizations.progress_background,
            defaults.progress_background,
        )?,
        background_xp_count: color_or_default(
            &customizations.background_xp_count,
            defaults.background_xp_count,
        )?,
        foreground_xp_count: color_or_default(
            &customizations.foreground_xp_count,
            defaults.foreground_xp_count,
        )?,
        font: font_or_default(&customizations.font, defaults.font).ok_or(Error::InvalidFont)?,
        toy: toy_or_none(&customizations.toy_image),
        card: Card::from_name(&customizations.card_layout).ok_or(Error::InvalidCard)?,
    })
}

fn color_or_default(color: &Option<String>, default: Color) -> Result<Color, Error> {
    if let Some(color) = &color {
        Ok(Color::from_hex(color)?)
    } else {
        Ok(default)
    }
}

fn font_or_default(font: &Option<String>, default: Font) -> Option<Font> {
    if let Some(font) = font {
        Some(Font::from_name(font)?)
    } else {
        Some(default)
    }
}

fn toy_or_none(toy: &Option<String>) -> Option<Toy> {
    if let Some(toy) = toy {
        Some(Toy::from_filename(toy)?)
    } else {
        None
    }
}

async fn get_avatar(state: SlashState, user: Arc<User>) -> Result<String, Error> {
    let url = user.avatar.map_or_else(
        || {
            format!(
                "https://cdn.discordapp.com/embed/avatars/{}.png",
                (user.id.get() >> 22) % 6
            )
        },
        |hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user.id, hash
            )
        },
    );
    debug!(url, "Downloading avatar");
    let png = state.http.get(url).send().await?.bytes().await?;
    debug!("Encoding avatar");
    let data = "data:image/png;base64,".to_string() + &BASE64_ENGINE.encode(png);
    debug!("Encoded avatar");
    Ok(data)
}

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::general_purpose::NO_PAD,
);
