#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

mod admin;
mod autocomplete;
mod cmd_defs;
mod config;
mod dispatch;
mod error;
mod gdpr;
mod help;
mod leaderboard;
mod levels;
mod manage_card;
mod manager;
mod response;

use std::{future::Future, sync::Arc, time::Instant};

pub use error::Error;
pub use response::XpdSlashResponse;
use sqlx::PgPool;
use tokio::{runtime::Handle, sync::mpsc::Sender, task::JoinHandle};
use tokio_util::task::TaskTracker;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    gateway::{payload::incoming::InteractionCreate, Intents},
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;
use xpd_common::{id_to_db, GuildConfig, RequiredDiscordResources};
use xpd_database::Database;
use xpd_rank_card::SvgState;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate sqlx;

pub type UpdateSender<T> = Sender<(Id<GuildMarker>, T)>;

#[derive(Clone)]
pub struct XpdSlash {
    state: SlashState,
}

pub struct InvalidateCache(pub Id<GuildMarker>);

#[derive(Clone)]
pub struct UpdateChannels {
    pub config: UpdateSender<GuildConfig>,
    pub rewards: Sender<InvalidateCache>,
}

impl XpdSlash {
    /// Creates a new xpd slash, which can be passed around
    /// Make sure to trim your ``root_url`` trailing slash.
    ///
    /// # Panics
    /// If loading resources or connecting to a database fails, this function will panic.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        http: reqwest::Client,
        client: Arc<twilight_http::Client>,
        id: Id<ApplicationMarker>,
        db: PgPool,
        cache: Arc<InMemoryCache>,
        task_tracker: TaskTracker,
        control_guild: Id<GuildMarker>,
        owners: Vec<Id<UserMarker>>,
        update_channels: UpdateChannels,
    ) -> Self {
        let svg = SvgState::new("xpd-card-resources").unwrap();
        let rt = Handle::current();
        let state = SlashState {
            db,
            client,
            my_id: id,
            svg,
            task_tracker,
            http,
            rt,
            cache,
            control_guild,
            owners: owners.into(),
            update_channels,
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
                            .flags(MessageFlags::EPHEMERAL)
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

const THEME_COLOR: u32 = 0x33_33_66;

#[derive(Clone)]
pub struct SlashState {
    pub db: PgPool,
    pub client: Arc<twilight_http::Client>,
    pub my_id: Id<ApplicationMarker>,
    pub task_tracker: TaskTracker,
    pub cache: Arc<InMemoryCache>,
    pub svg: SvgState,
    pub rt: Handle,
    pub http: reqwest::Client,
    pub owners: Arc<[Id<UserMarker>]>,
    pub control_guild: Id<GuildMarker>,
    pub update_channels: UpdateChannels,
}

impl Database for SlashState {
    fn db(&self) -> &PgPool {
        &self.db
    }
}

impl SlashState {
    pub async fn update_config(&self, guild: Id<GuildMarker>, config: GuildConfig) {
        let _ = self.update_channels.config.send((guild, config)).await;
    }

    pub async fn invalidate_rewards(&self, guild: Id<GuildMarker>) {
        let _ = self
            .update_channels
            .rewards
            .send(InvalidateCache(guild))
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
        let xp = self.query_user_xp(guild_id, id).await?.unwrap_or(0);
        let rank = self
            .query_count_with_higher_xp(guild_id, xp)
            .await?
            .unwrap_or(0)
            + 1;
        Ok(UserStats { xp, rank })
    }

    /// # Errors
    /// This function reports an error INTERNALLY, but not at the callsite.
    /// Its failures are generally not recoverable to that task, though.
    pub async fn send_followup(&self, response: XpdSlashResponse, token: &str) {
        trace!(?response, "sending followup message");
        if let Err(source) = self
            .client
            .interaction(self.my_id)
            .create_followup(token)
            .allowed_mentions(response.allowed_mentions.as_ref())
            .attachments(&response.attachments.unwrap_or_default())
            .components(&response.components.unwrap_or_default())
            .content(&response.content.unwrap_or_default())
            .embeds(&response.embeds.unwrap_or_default())
            .tts(response.tts.unwrap_or(false))
            .flags(response.flags.unwrap_or(MessageFlags::empty()))
            .await
        {
            error!(?source, "Failed to respond to interaction");
        }
    }

    pub fn spawn<F>(&self, item: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.task_tracker.spawn_on(item, &self.rt)
    }
}
