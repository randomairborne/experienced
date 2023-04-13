use crate::{AppState, Error};
use base64::Engine;
use sqlx::query;
use twilight_model::{
    channel::message::MessageFlags,
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseType},
    },
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};
use xpd_rank_card::{Font, Toy};

pub async fn get_level(
    guild_id: Id<GuildMarker>,
    user: User,
    invoker: User,
    state: AppState,
    interaction_token: String,
) -> Result<InteractionResponse, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let guild_id = guild_id.get() as i64;
    // Select current XP from the database, return 0 if there is no row
    #[allow(clippy::cast_possible_wrap)]
    let xp = query!(
        "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
        user.id.get() as i64,
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
    let content = if user.bot {
        "Bots aren't ranked, that would be silly!".to_string()
    } else if invoker == user {
        if xp == 0 {
            "You aren't ranked yet, because you haven't sent any messages!".to_string()
        } else {
            return generate_level_response(state, user, level_info, rank, interaction_token).await;
        }
    } else if xp == 0 {
        format!(
            "{}#{} isn't ranked yet, because they haven't sent any messages!",
            user.name,
            user.discriminator()
        )
    } else {
        return generate_level_response(state, user, level_info, rank, interaction_token).await;
    };
    Ok(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .embeds([EmbedBuilder::new().description(content).build()])
                .build(),
        ),
    })
}

#[allow(clippy::unused_async)]
async fn generate_level_response(
    state: AppState,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
    interaction_token: String,
) -> Result<InteractionResponse, Error> {
    tokio::spawn(update_interaction_with_card(
        state,
        user,
        level_info,
        rank,
        interaction_token,
    ));
    Ok(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

async fn update_interaction_with_card(
    state: AppState,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
    interaction_token: String,
) {
    if let Err(e) = update_interaction_with_card_actual(
        &state,
        user,
        level_info,
        rank,
        interaction_token.clone(),
    )
    .await
    {
        warn!("{e:?}");
        if let Ok(followup) = state
            .client
            .interaction(state.my_id)
            .create_followup(&interaction_token)
            .content(&format!("{e:#?}"))
        {
            if let Err(e) = followup.await {
                warn!("{e:?}");
            }
        };
    }
}

async fn update_interaction_with_card_actual(
    state: &AppState,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
    interaction_token: String,
) -> Result<(), Error> {
    let card = gen_card(state, &user, level_info, rank).await?;
    state
        .client
        .interaction(state.my_id)
        .create_followup(&interaction_token)
        .attachments(&[card])?
        .await?;
    Ok(())
}

pub async fn gen_card(
    state: &AppState,
    user: &User,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<Attachment, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let non_color_customizations = query!(
        "SELECT font, toy_image FROM custom_card WHERE id = $1",
        user.id.get() as i64
    )
    .fetch_optional(&state.db)
    .await?;
    let (font, toy) = if let Some(customizations) = non_color_customizations {
        let font = {
            if let Some(strfont) = customizations.font {
                Font::from_name(&strfont).ok_or(Error::InvalidFont)?
            } else {
                Font::Roboto
            }
        };
        let toy = customizations
            .toy_image
            .and_then(|v| Toy::from_filename(&v));
        (font, toy)
    } else {
        (Font::Roboto, None)
    };
    let avatar = get_avatar(state, user).await?;
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    let png = state
        .svg
        .render(xpd_rank_card::Context {
            level: level_info.level(),
            rank,
            name: user.name.clone(),
            discriminator: user.discriminator().to_string(),
            percentage: (level_info.percentage() * 100.0).round() as u64,
            current: level_info.xp(),
            needed: mee6::xp_needed_for_level(level_info.level() + 1),
            colors: crate::colors::for_user(&state.db, user.id).await,
            font,
            toy,
            avatar,
        })
        .await?;
    Ok(Attachment {
        description: Some(format!(
            "{}#{} is level {} (rank #{}), and is {}% of the way to level {}.",
            user.name,
            user.discriminator(),
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

pub fn leaderboard(guild_id: Id<GuildMarker>) -> InteractionResponse {
    let guild_link = format!("https://xp.valk.sh/{guild_id}");
    let embed = EmbedBuilder::new()
        .description(format!("[Click to view the leaderboard!]({guild_link})"))
        .color(crate::THEME_COLOR)
        .build();
    let data = InteractionResponseDataBuilder::new()
        .embeds([embed])
        .build();
    InteractionResponse {
        data: Some(data),
        kind: InteractionResponseType::ChannelMessageWithSource,
    }
}

async fn get_avatar(state: &AppState, user: &User) -> Result<String, Error> {
    let url = user.avatar.map_or_else(
        || {
            format!(
                "https://cdn.discordapp.com/embed/avatars/{}/{}.png",
                user.id,
                user.discriminator % 5
            )
        },
        |hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user.id, hash
            )
        },
    );
    let png = state.http.get(url).send().await?.bytes().await?;
    let data = format!("data:image/png;base64,{}", BASE64_ENGINE.encode(png));
    Ok(data)
}

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::general_purpose::NO_PAD,
);
