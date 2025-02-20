use base64::Engine;
use tokio::try_join;
use twilight_model::{
    channel::message::MessageFlags,
    http::{attachment::Attachment, interaction::InteractionResponseType},
    id::{
        Id,
        marker::{GenericMarker, GuildMarker, UserMarker},
    },
    util::ImageHash,
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::{DisplayName, MemberDisplayInfo};
use xpd_rank_card::customizations::{Color, Customizations};

use crate::{Error, SlashState, XpdInteractionData, response::XpdInteractionResponse};

pub async fn get_level(
    guild_id: Id<GuildMarker>,
    target: MemberDisplayInfo,
    invoker: Id<UserMarker>,
    showoff: Option<bool>,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    let rank_stats = state.get_user_stats(target.id, guild_id);
    let guild_config = xpd_database::guild_config(&state.db, guild_id);
    let (rank_stats, guild_config) = try_join!(rank_stats, guild_config)?;
    let flags = if showoff
        .or_else(|| guild_config.map(|cfg| cfg.guild_card_default_show_off))
        .is_some_and(|v| v)
    {
        MessageFlags::empty()
    } else {
        MessageFlags::EPHEMERAL
    };

    let level_info = mee6::LevelInfo::new(u64::try_from(rank_stats.xp).unwrap_or(0));
    let content = if target.bot {
        "Bots aren't ranked, that would be silly!".to_string()
    } else if invoker == target.id {
        if rank_stats.xp == 0 {
            "You aren't ranked yet, because you haven't sent any messages!".to_string()
        } else {
            return generate_level_response(
                &state,
                target,
                guild_id,
                level_info,
                rank_stats.rank,
                flags,
            )
            .await;
        }
    } else if rank_stats.xp == 0 {
        format!(
            "{} isn't ranked yet, because they haven't sent any messages!",
            target.display_name()
        )
    } else {
        return generate_level_response(
            &state,
            target,
            guild_id,
            level_info,
            rank_stats.rank,
            flags,
        )
        .await;
    };
    let embed = EmbedBuilder::new().description(content).build();
    Ok(XpdInteractionData::new()
        .embeds([embed])
        .flags(flags)
        .into_interaction_response(InteractionResponseType::ChannelMessageWithSource))
}

async fn generate_level_response(
    state: &SlashState,
    user: MemberDisplayInfo,
    guild_id: Id<GuildMarker>,
    level_info: mee6::LevelInfo,
    rank: i64,
    flags: MessageFlags,
) -> Result<XpdInteractionResponse, Error> {
    let card = gen_card(state.clone(), user, Some(guild_id), level_info, rank).await?;
    Ok(XpdInteractionData::new()
        .attachments([card])
        .flags(flags)
        .into_interaction_response(InteractionResponseType::ChannelMessageWithSource))
}

async fn get_customizations_fields(
    state: SlashState,
    user_id: Id<UserMarker>,
    guild_id: Option<Id<GuildMarker>>,
) -> Result<Customizations, Error> {
    if let Some(guild_id) = guild_id {
        get_customizations(&state, &[user_id.cast(), guild_id.cast()]).await
    } else {
        get_customizations(&state, &[user_id.cast()]).await
    }
}

pub async fn gen_card(
    state: SlashState,
    user: MemberDisplayInfo,
    guild_id: Option<Id<GuildMarker>>,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<Attachment, Error> {
    let customizations_future = get_customizations_fields(state.clone(), user.id, guild_id);
    let avatar_ref = AvatarReference::new(user.id, user.avatar, guild_id, user.local_avatar);
    let avatar_future = get_avatar(&state.http, avatar_ref);
    let (customizations, avatar) = try_join!(customizations_future, avatar_future)?;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let percentage = (level_info.percentage() * 100.0).round() as u64;
    let png = state
        .svg
        .render(xpd_rank_card::Context {
            level: level_info.level(),
            rank,
            name: user.display_name().to_string(),
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
            user.display_name(),
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
    state: &SlashState,
    ids: &[Id<GenericMarker>],
) -> Result<Customizations, Error> {
    let Some(customizations) = xpd_database::card_customizations(&state.db, ids).await? else {
        return Ok(state.svg.default_customizations().clone());
    };
    let defaults = state
        .svg
        .customizations_for(&customizations.card_layout)
        .ok_or(Error::UnknownCard)?;

    Ok(Customizations {
        username: color_or_default(customizations.username.as_deref(), defaults.username)?,
        rank: color_or_default(customizations.rank.as_deref(), defaults.rank)?,
        level: color_or_default(customizations.level.as_deref(), defaults.level)?,
        border: color_or_default(customizations.border.as_deref(), defaults.border)?,
        background: color_or_default(customizations.background.as_deref(), defaults.background)?,
        progress_foreground: color_or_default(
            customizations.progress_foreground.as_deref(),
            defaults.progress_foreground,
        )?,
        progress_background: color_or_default(
            customizations.progress_background.as_deref(),
            defaults.progress_background,
        )?,
        background_xp_count: color_or_default(
            customizations.background_xp_count.as_deref(),
            defaults.background_xp_count,
        )?,
        foreground_xp_count: color_or_default(
            customizations.foreground_xp_count.as_deref(),
            defaults.foreground_xp_count,
        )?,
        font: customizations.font.unwrap_or_else(|| defaults.font.clone()),
        toy: customizations.toy_image,
        internal_name: customizations.card_layout,
    })
}

fn color_or_default(color: Option<&str>, default: Color) -> Result<Color, Error> {
    if let Some(color) = &color {
        Ok(Color::from_hex(color)?)
    } else {
        Ok(default)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AvatarReference {
    id: Id<UserMarker>,
    kind: Option<AvatarReferenceKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AvatarReferenceKind {
    Guild(Id<GuildMarker>, ImageHash),
    User(ImageHash),
}

impl AvatarReference {
    pub fn new(
        id: Id<UserMarker>,
        user_hash: Option<ImageHash>,
        guild_id: Option<Id<GuildMarker>>,
        guild_hash: Option<ImageHash>,
    ) -> Self {
        let kind = guild_hash
            .and_then(|hash| Some(AvatarReferenceKind::Guild(guild_id?, hash)))
            .or_else(|| user_hash.map(AvatarReferenceKind::User));
        Self { id, kind }
    }

    pub fn to_url(self) -> String {
        let user_id = self.id;
        match self.kind {
            Some(AvatarReferenceKind::Guild(guild_id, avatar_hash)) => format!(
                "https://cdn.discordapp.com/guilds/{guild_id}/users/{user_id}/avatars/{avatar_hash}.png",
            ),
            Some(AvatarReferenceKind::User(avatar_hash)) => {
                format!("https://cdn.discordapp.com/avatars/{user_id}/{avatar_hash}.png",)
            }
            None => format!(
                "https://cdn.discordapp.com/embed/avatars/{}.png",
                (user_id.get() >> 22) % 6
            ),
        }
    }
}

#[tracing::instrument(skip(client))]
async fn get_avatar(client: &reqwest::Client, avatar: AvatarReference) -> Result<String, Error> {
    let url = avatar.to_url();
    debug!(url, "Downloading avatar");
    let png = client.get(url).send().await?.bytes().await?;
    debug!("Encoding avatar");
    let data = "data:image/png;base64,".to_string() + &BASE64_ENGINE.encode(png);
    debug!("Encoded avatar");
    Ok(data)
}

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::general_purpose::NO_PAD,
);
