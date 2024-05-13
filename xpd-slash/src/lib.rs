#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

mod admin;
mod cmd_defs;
mod dispatch;
mod error;
mod gdpr;
mod help;
mod leaderboard;
mod levels;
mod manage_card;
mod manager;
mod response;

use std::{sync::Arc, time::Instant};

pub use error::Error;
pub use response::XpdSlashResponse;
use sqlx::PgPool;
use twilight_model::{
    application::interaction::Interaction,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;
use xpd_common::id_to_db;
use xpd_rank_card::SvgState;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate sqlx;

#[derive(Clone)]
pub struct XpdSlash {
    state: SlashState,
}

impl XpdSlash {
    /// Make sure to trim your ``root_url`` trailing slash.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        http: reqwest::Client,
        client: Arc<twilight_http::Client>,
        id: Id<ApplicationMarker>,
        db: PgPool,
        root_url: String,
        control_guild: Id<GuildMarker>,
        owners: Vec<Id<UserMarker>>,
    ) -> Self {
        let svg = SvgState::new();
        let state = SlashState {
            db,
            client,
            my_id: id,
            svg,
            http,
            root_url: root_url.into(),
            control_guild,
            owners: owners.into(),
        };
        info!("Creating commands...");
        state.register_slashes().await;
        Self { state }
    }

    pub async fn execute(&self, interaction_create: InteractionCreate) {
        let interaction_token = interaction_create.token.clone();
        let ic_id = interaction_create.id;
        let process_start = Instant::now();
        let response = self.run(interaction_create.0).await;
        let total_time = process_start.elapsed();
        info!(?total_time, "processed interaction in time");
        if let Err(error) = self
            .client()
            .interaction(self.id())
            .create_response(ic_id, &interaction_token, &response)
            .await
        {
            error!(?error, "Failed to ack discord gateway message");
        };
    }

    async fn run(&self, interaction: Interaction) -> InteractionResponse {
        Box::pin(dispatch::process(interaction, self.state.clone()))
            .await
            .unwrap_or_else(|error| {
                error!(?error, "got error");
                InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(
                        InteractionResponseDataBuilder::new()
                            .content(error.to_string())
                            .build(),
                    ),
                }
            })
    }

    #[must_use]
    pub fn client(&self) -> Arc<twilight_http::Client> {
        self.state.client.clone()
    }

    #[must_use]
    pub const fn id(&self) -> Id<ApplicationMarker> {
        self.state.my_id
    }

    /// # Errors
    /// Errors if the message could not be validated
    /// this is stupid
    /// [`twilight_http`] validation is supposed to be OPTIONAL
    pub async fn send_followup(
        &self,
        response: XpdSlashResponse,
        token: &str,
    ) -> Result<(), Error> {
        trace!(?response, "sending followup message");
        if let Err(source) = self
            .client()
            .interaction(self.id())
            .create_followup(token)
            .allowed_mentions(response.allowed_mentions.as_ref())
            .attachments(&response.attachments.unwrap_or_default())?
            .components(&response.components.unwrap_or_default())?
            .content(&response.content.unwrap_or_default())?
            .embeds(&response.embeds.unwrap_or_default())?
            .tts(response.tts.unwrap_or(false))
            .await
        {
            error!(?source, "Failed to respond to interaction");
        }
        Ok(())
    }
}

const THEME_COLOR: u32 = 0x33_33_66;

#[derive(Clone)]
pub struct SlashState {
    pub db: PgPool,
    pub client: Arc<twilight_http::Client>,
    pub my_id: Id<ApplicationMarker>,
    pub svg: SvgState,
    pub http: reqwest::Client,
    pub root_url: Arc<str>,
    pub owners: Arc<[Id<UserMarker>]>,
    pub control_guild: Id<GuildMarker>,
}

#[derive(Copy, Clone)]
pub struct UserStats {
    xp: i64,
    rank: i64,
}

impl SlashState {
    pub async fn get_user_stats(
        &self,
        id: Id<UserMarker>,
        guild_id: Id<GuildMarker>,
    ) -> Result<UserStats, Error> {
        // Select current XP from the database, return 0 if there is no row
        let xp = query!(
            "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
            id_to_db(id),
            id_to_db(guild_id)
        )
        .fetch_optional(&self.db)
        .await?
        .map_or(0, |v| v.xp);
        let rank = query!(
            "SELECT COUNT(*) as count FROM levels WHERE xp > $1 AND guild = $2",
            xp,
            id_to_db(guild_id)
        )
        .fetch_one(&self.db)
        .await?
        .count
        .unwrap_or(0)
            + 1;
        Ok(UserStats { xp, rank })
    }
}
