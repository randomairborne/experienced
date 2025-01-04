#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

mod admin;
mod audit;
mod autocomplete;
mod config;
mod dispatch;
mod error;
mod experience;
mod gdpr;
mod help;
mod leaderboard;
mod levels;
mod manage_card;
mod manager;
mod response;
mod rewards;

use std::{future::Future, sync::Arc, time::Instant};

pub use error::Error;
pub use response::XpdInteractionData;
use response::XpdInteractionResponse;
use sqlx::PgPool;
use tokio::{runtime::Handle, sync::mpsc::Sender, task::JoinHandle};
use tokio_util::task::TaskTracker;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    gateway::{payload::incoming::InteractionCreate, Intents},
    http::interaction::InteractionResponseType,
    id::{
        marker::{ApplicationMarker, GuildMarker, UserMarker},
        Id,
    },
};
use xpd_common::{EventBusMessage, GuildConfig, RequiredDiscordResources};
use xpd_rank_card::SvgState;
use xpd_util::LogError;

#[macro_use]
extern crate tracing;

#[derive(Clone)]
pub struct XpdSlash {
    state: SlashState,
}

pub type EventBus = Sender<EventBusMessage>;

impl XpdSlash {
    /// Creates a new xpd slash, which can be passed around
    /// Make sure to trim your ``root_url`` trailing slash.
    ///
    /// # Panics
    /// If loading resources or connecting to a database fails, this function will panic.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        http: reqwest::Client,
        client: Arc<twilight_http::Client>,
        app_id: Id<ApplicationMarker>,
        bot_id: Id<UserMarker>,
        db: PgPool,
        cache: Arc<InMemoryCache>,
        task_tracker: TaskTracker,
        control_guild: Id<GuildMarker>,
        owners: Vec<Id<UserMarker>>,
        event_bus: EventBus,
    ) -> Self {
        let svg = SvgState::new("xpd-card-resources").expect("Failed to initialize card renderer");
        let rt = Handle::current();
        let state = SlashState {
            db,
            client,
            app_id,
            bot_id,
            svg,
            task_tracker,
            http,
            rt,
            cache,
            control_guild,
            owners: owners.into(),
            event_bus,
        };
        Self { state }
    }

    pub async fn execute(&self, interaction_create: InteractionCreate) {
        let interaction_token = interaction_create.token.clone();
        let ic_id = interaction_create.id;
        let process_start = Instant::now();
        let response = self.run(interaction_create.0).await;
        let total_time = process_start.elapsed();
        info!(?total_time, "processed interaction in time");
        if !response.inhibit {
            let response = response.into();
            self.client()
                .interaction(self.state.app_id)
                .create_response(ic_id, &interaction_token, &response)
                .await
                .log_error("Failed to ack discord gateway message");
        }
    }

    async fn run(&self, interaction: Interaction) -> XpdInteractionResponse {
        Box::pin(dispatch::process(interaction, self.state.clone()))
            .await
            .unwrap_or_else(|error| {
                error!(?error, "got error");
                XpdInteractionData::new()
                    .ephemeral(true)
                    .content(error.to_string())
                    .into_interaction_response(InteractionResponseType::ChannelMessageWithSource)
            })
    }

    #[must_use]
    pub fn client(&self) -> Arc<twilight_http::Client> {
        self.state.client.clone()
    }
}

impl RequiredDiscordResources for XpdSlash {
    fn required_intents() -> Intents {
        Intents::empty()
    }

    fn required_events() -> EventTypeFlags {
        EventTypeFlags::INTERACTION_CREATE
    }

    fn required_cache_types() -> ResourceType {
        ResourceType::ROLE
            | ResourceType::CHANNEL
            | ResourceType::USER_CURRENT
            | ResourceType::GUILD
            | ResourceType::MEMBER
    }
}

#[derive(Clone)]
pub struct SlashState {
    pub db: PgPool,
    pub client: Arc<twilight_http::Client>,
    pub app_id: Id<ApplicationMarker>,
    pub task_tracker: TaskTracker,
    pub bot_id: Id<UserMarker>,
    pub cache: Arc<InMemoryCache>,
    pub svg: SvgState,
    pub rt: Handle,
    pub http: reqwest::Client,
    pub owners: Arc<[Id<UserMarker>]>,
    pub control_guild: Id<GuildMarker>,
    pub event_bus: EventBus,
}

impl SlashState {
    pub async fn update_config(&self, guild: Id<GuildMarker>, config: GuildConfig) {
        let _ = self
            .event_bus
            .send(EventBusMessage::UpdateConfig(guild, config))
            .await;
    }

    pub async fn invalidate_rewards(&self, guild: Id<GuildMarker>) {
        let _ = self
            .event_bus
            .send(EventBusMessage::InvalidateRewards(guild))
            .await;
    }
}

#[derive(Copy, Clone)]
pub struct UserStats {
    xp: i64,
    rank: i64,
}

impl SlashState {
    /// Get public-facing statistics for a user
    /// # Errors
    /// This function can error when sqlx fails to get the right datatype.
    /// # Panics
    /// This can panic if sqlx is unable to convert the rows to the proper types.
    pub async fn get_user_stats(
        &self,
        id: Id<UserMarker>,
        guild_id: Id<GuildMarker>,
    ) -> Result<UserStats, Error> {
        let xp = xpd_database::user_xp(&self.db, guild_id, id)
            .await?
            .unwrap_or(0);
        let rank = xpd_database::count_with_higher_xp(&self.db, guild_id, xp)
            .await?
            .unwrap_or(0)
            + 1;
        Ok(UserStats { xp, rank })
    }

    /// # Errors
    /// This function reports an error INTERNALLY, but not at the callsite.
    /// Its failures are generally not recoverable to that task, though.
    pub async fn send_followup(&self, response: XpdInteractionData, token: &str) {
        trace!(?response, "sending followup message");
        self.client
            .interaction(self.app_id)
            .create_followup(token)
            .allowed_mentions(response.allowed_mentions.as_ref())
            .attachments(&response.attachments.unwrap_or_default())
            .components(&response.components.unwrap_or_default())
            .content(&response.content.unwrap_or_default())
            .embeds(&response.embeds.unwrap_or_default())
            .tts(response.tts.unwrap_or(false))
            .flags(response.flags.unwrap_or(MessageFlags::empty()))
            .await
            .log_error("Failed to respond to interaction");
    }

    pub fn spawn<F>(&self, item: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.task_tracker.spawn_on(item, &self.rt)
    }
}
