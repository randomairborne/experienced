use crate::{processor::CommandProcessorError, AppState};
use sqlx::query;
use twilight_model::{
    channel::message::{
        component::{Button, ButtonStyle},
        Component, MessageFlags, ReactionType,
    },
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
        let toy = query!(
            "SELECT toy FROM card_toy WHERE id = $1",
            user.id.get() as i64
        )
        .fetch_optional(&state.db)
        .await
        .map_or(None, |v| v.map(|r| r.toy));
        #[allow(clippy::cast_precision_loss)]
        match crate::render_card::render(
            state.clone(),
            crate::render_card::Context {
                level: level_info.level(),
                rank,
                name: user.name.clone(),
                discriminator: user.discriminator().to_string(),
                percentage: get_percentage_bar_as_pixels(level_info.percentage()),
                current: level_info.xp(),
                needed: mee6::xp_needed_for_level(level_info.level() + 1),
                toy,
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
                            (level_info.percentage() * 100.0).round(),
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

pub async fn leaderboard(
    guild_id: Id<GuildMarker>,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    #[allow(clippy::cast_possible_wrap)]
    let users = query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp LIMIT 10",
        guild_id.get() as i64
    )
    .fetch_all(&state.db)
    .await?;
    let mut description = String::with_capacity(users.len() * 128);
    #[allow(clippy::cast_sign_loss)]
    for user in users {
        description += &format!("<@{}>\n", user.id as u64);
    }
    let embed = EmbedBuilder::new()
        .description(description)
        .color(crate::THEME_COLOR)
        .build();
    let back_button = Component::Button(Button {
        custom_id: Some("back_button".to_string()),
        disabled: true,
        emoji: Some(ReactionType::Unicode {
            name: "arrow_backward".to_string(),
        }),
        label: Some("Previous".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let forward_button = Component::Button(Button {
        custom_id: Some("forward_button".to_string()),
        disabled: true,
        emoji: Some(ReactionType::Unicode {
            name: "arrow_forward".to_string(),
        }),
        label: Some("Next".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let data = InteractionResponseDataBuilder::new()
        .embeds([embed])
        .components([back_button, forward_button])
        .build();
    Ok(InteractionResponse {
        data: Some(data),
        kind: InteractionResponseType::ChannelMessageWithSource,
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn get_percentage_bar_as_pixels(percentage: f64) -> u64 {
    percentage.mul_add(700.0, 40.0) as u64
}
