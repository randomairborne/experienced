#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cmd_defs;
mod colors;
mod error;
mod help;
mod levels;
mod manage_card;
mod manager;
mod mee6_worker;
mod processor;

pub use error::Error;
use twilight_util::builder::InteractionResponseDataBuilder;

use parking_lot::Mutex;
use sqlx::PgPool;
use std::{collections::VecDeque, sync::Arc};
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, GuildMarker},
        Id,
    },
};
use xpd_rank_card::SvgState;

#[derive(Clone)]
pub struct Slash {
    state: SlashState,
}

impl Slash {
    pub async fn new(
        http: reqwest::Client,
        client: Arc<twilight_http::Client>,
        id: Id<ApplicationMarker>,
        db: PgPool,
        #[cfg(feature = "ratelimiting")] redis: deadpool_redis::Pool,
    ) -> Self {
        let svg = SvgState::new();
        let import_queue = ImportQueue::new();
        let state = SlashState {
            db,
            client,
            my_id: id,
            svg,
            http,
            #[cfg(feature = "ratelimiting")]
            redis,
            import_queue,
        };
        debug!("Creating commands...");
        cmd_defs::register(state.client.interaction(state.my_id)).await;
        tokio::spawn(mee6_worker::do_fetches(state.clone()));
        Self { state }
    }
    pub async fn run(self, interaction: Interaction) {
        let interaction_token = interaction.token.clone();
        let interaction_id = interaction.id;
        let response =
            match Box::pin(crate::processor::process(interaction, self.state.clone())).await {
                Ok(val) => val,
                Err(e) => {
                    error!("{e}");
                    InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(
                            InteractionResponseDataBuilder::new()
                                .flags(MessageFlags::EPHEMERAL)
                                .content(e.to_string())
                                .build(),
                        ),
                    }
                }
            };
        if let Err(e) = self
            .state
            .client
            .interaction(self.state.my_id)
            .create_response(interaction_id, &interaction_token, &response)
            .await
        {
            error!("Error responding to interaction: {e}");
        }
    }
}

#[macro_use]
extern crate tracing;

const THEME_COLOR: u32 = 0x33_33_66;

#[derive(Clone)]
pub struct SlashState {
    pub db: PgPool,
    pub client: Arc<twilight_http::Client>,
    pub my_id: Id<ApplicationMarker>,
    pub svg: SvgState,
    pub http: reqwest::Client,
    #[cfg(feature = "ratelimiting")]
    pub redis: deadpool_redis::Pool,
    pub import_queue: ImportQueue,
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
