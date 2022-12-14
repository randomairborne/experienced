use crate::{processor::CommandProcessorError, AppState};
use sqlx::query;
use twilight_model::{
    channel::message::MessageFlags,
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseType},
    },
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub async fn get_level(
    user: User,
    invoker: User,
    token: String,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    // Select current XP from the database, return 0 if there is no row
    #[allow(clippy::cast_possible_wrap)]
    let xp = match query!("SELECT xp FROM levels WHERE id = $1", user.id.get() as i64)
        .fetch_one(&state.db)
        .await
    {
        Ok(val) => val.xp,
        Err(e) => match e {
            sqlx::Error::RowNotFound => 0,
            _ => Err(e)?,
        },
    };
    let rank = query!("SELECT COUNT(*) as count FROM levels WHERE xp > $1", xp)
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
                .content(content)
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
        let chosen_font = query!(
            "SELECT font FROM custom_card WHERE id = $1",
            user.id.get() as i64
        )
        .fetch_one(&state.db)
        .await;
        #[allow(clippy::cast_precision_loss)]
        let next_level = (level_info.level() + 1) as f64;
        match crate::render_card::render(
            state.clone(),
            crate::render_card::Context {
                level: level_info.level(),
                rank,
                name: user.name.clone(),
                discriminator: user.discriminator().to_string(),
                width: 40 + (u64::from(level_info.percentage()) * 7),
                current: level_info.xp(),
                needed: mee6::LevelInfo::xp_to_level(next_level),
                colors: crate::colors::Colors::for_user(&state.db, user.id).await,
                font: chosen_font.map_or_else(
                    |_| "Roboto".to_string(),
                    |v| v.font.unwrap_or_else(|| "Roboto".to_string()),
                ),
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
