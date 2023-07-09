#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cmd_defs;
mod error;
mod help;
mod levels;
mod manage_card;
mod manager;
mod mee6_worker;
mod processor;
mod response;

pub use error::Error;
pub use response::XpdSlashResponse;

use parking_lot::Mutex;
use sqlx::PgPool;
use std::{collections::VecDeque, sync::Arc};
use twilight_model::{
    application::interaction::Interaction,
    id::{
        marker::{ApplicationMarker, GuildMarker},
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
    pub async fn new(
        http: reqwest::Client,
        client: Arc<twilight_http::Client>,
        id: Id<ApplicationMarker>,
        db: PgPool,
        redis: deadpool_redis::Pool,
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
        };
        info!("Creating commands...");
        cmd_defs::register(state.client.interaction(state.my_id)).await;
        tokio::spawn(mee6_worker::do_fetches(state.clone()));
        Self { state }
    }
    pub async fn run(self, interaction: Interaction) -> XpdSlashResponse {
        match Box::pin(crate::processor::process(interaction, self.state.clone())).await {
            Ok(val) => val,
            Err(e) => {
                error!("{e}");
                XpdSlashResponse::new().content(e.to_string())
            }
        }
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
