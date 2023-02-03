use crate::{processor::CommandProcessorError, AppState};
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

pub async fn get_level(
    guild_id: Id<GuildMarker>,
    user: User,
    invoker: User,
    token: String,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
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
            return generate_level_response(state, token, user, level_info, rank).await;
        }
    } else if xp == 0 {
        format!(
            "{}#{} isn't ranked yet, because they haven't sent any messages!",
            user.name,
            user.discriminator()
        )
    } else {
        return generate_level_response(state, token, user, level_info, rank).await;
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

async fn generate_level_response(
    state: AppState,
    token: String,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<InteractionResponse, CommandProcessorError> {
    tokio::task::spawn(async move {
        let interaction_client = state.client.interaction(state.my_id);
        #[allow(clippy::cast_possible_wrap)]
        let non_color_customizations = query!(
            "SELECT font, toy_image FROM custom_card WHERE id = $1",
            user.id.get() as i64
        )
        .fetch_one(&state.db)
        .await;
        let (font, icon) = non_color_customizations.map_or_else(
            |_| ("Roboto".to_string(), None),
            |v| (v.font.unwrap_or_else(|| "Roboto".to_string()), v.toy_image),
        );
        #[allow(clippy::cast_precision_loss)]
        match crate::render_card::render(
            state.clone(),
            crate::render_card::Context {
                level: level_info.level(),
                rank,
                name: user.name.clone(),
                discriminator: user.discriminator().to_string(),
                width: get_percentage_bar_as_pixels(level_info.percentage()),
                current: level_info.xp(),
                needed: mee6::xp_needed_for_level(level_info.level() + 1),
                colors: crate::colors::Colors::for_user(&state.db, user.id).await,
                font,
                icon,
            },
        )
        .await
        {
            Ok(png) => {
                match interaction_client
                    .create_followup(&token)
                    .attachments(&[Attachment {
                        description: Some(format!(
                            "{}#{} is level {} (rank #{}), and is {}% of the way to level {}.",
                            user.name,
                            user.discriminator(),
                            level_info.level(),
                            rank,
                            level_info.percentage(),
                            level_info.level() + 1
                        )),
                        file: png,
                        filename: "card.png".to_string(),
                        id: 0,
                    }]) {
                    Ok(followup) => followup.await,
                    Err(e) => {
                        warn!("{e}");
                        interaction_client
                            .create_followup(&token)
                            .content("Invalid upload, please contact bot administrators")
                            .unwrap()
                            .await
                    }
                }
            }
            Err(err) => {
                match interaction_client
                    .create_followup(&token)
                    .content(&format!("Rendering card failed: {err}"))
                {
                    Ok(awaitable) => awaitable.await,
                    Err(e) => {
                        warn!("{e}");
                        interaction_client
                            .create_followup(&token)
                            .content("Error too long, please contact bot administrators")
                            .unwrap()
                            .await
                    }
                }
            }
        }
    });
    Ok(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn get_percentage_bar_as_pixels(percentage: f64) -> u64 {
    percentage.mul_add(700.0, 40.0) as u64
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
