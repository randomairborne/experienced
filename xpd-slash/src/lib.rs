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

use std::{collections::VecDeque, sync::Arc};

pub use error::Error;
use parking_lot::Mutex;
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
        redis: deadpool_redis::Pool,
        root_url: String,
        control_guild: Id<GuildMarker>,
        owners: Vec<Id<UserMarker>>,
    ) -> Self {
        let svg = SvgState::new();
        let import_queue = ImportQueue::new();
        let state = SlashState {
            db,
            client,
            my_id: id,
            svg,
            http,
            redis,
            import_queue,
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
        if let Err(error) = self
            .client()
            .interaction(self.id())
            .create_response(
                interaction_create.id,
                &interaction_create.token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await
        {
            error!(?error, "Failed to ack discord gateway message");
        };
        let response = self.run(interaction_create.0).await;
        if let Err(error) = self.send_followup(response, &interaction_token).await {
            error!(?error, "Failed to send real response");
        };
    }

    async fn run(&self, interaction: Interaction) -> XpdSlashResponse {
        Box::pin(dispatch::process(interaction, self.state.clone()))
            .await
            .unwrap_or_else(|error| {
                error!(?error, "got error");
                XpdSlashResponse::new().content(error.to_string())
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
    pub redis: deadpool_redis::Pool,
    pub import_queue: ImportQueue,
    pub root_url: Arc<str>,
    pub owners: Arc<[Id<UserMarker>]>,
    pub control_guild: Id<GuildMarker>,
}

pub type ImportQueueMember = (Id<GuildMarker>, String);
#[derive(Clone, Default)]
pub struct ImportQueue {
    pub mee6: Arc<Mutex<VecDeque<ImportQueueMember>>>,
}

impl ImportQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
