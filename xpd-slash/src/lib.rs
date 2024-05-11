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
pub use response::XpdSlashResponse;
use sqlx::PgPool;
use tokio::sync::oneshot::{
    channel as oneshot_channel, Receiver as OneshotReceiver, Sender as OneshotSender,
};
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

    async fn run(&self, interaction: Interaction, pr: PreResponder) -> InteractionResponse {
        Box::pin(dispatch::process(interaction, self.state.clone(), pr))
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
