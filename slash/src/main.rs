#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cmd_defs;
mod colors;
mod discord_sig_validation;
mod error;
mod handler;
mod help;
mod levels;
mod manage_card;
mod manager;
mod mee6_worker;
mod processor;

pub use error::Error;

use parking_lot::Mutex;
use sqlx::PgPool;
use std::{collections::VecDeque, sync::Arc};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_model::id::{
    marker::{ApplicationMarker, GuildMarker},
    Id,
};
use xpd_rank_card::SvgState;

#[macro_use]
extern crate tracing;

const THEME_COLOR: u32 = 0x33_33_66;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_env("LOG"))
        .init();
    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected environment variable DISCORD_TOKEN");
    let pubkey = Arc::new(
        std::env::var("DISCORD_PUBKEY").expect("Expected environment variable DISCORD_PUBKEY"),
    );
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    let redis_url = std::env::var("REDIS_URL").expect("Expected environment variable REDIS_URL");
    println!("Connecting to redis {redis_url}");
    let redis = redis::aio::ConnectionManager::new(
        redis::Client::open(redis_url).expect("Failed to connect to redis"),
    )
    .await
    .expect("Failed to connect to redis!");
    println!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let client = Arc::new(twilight_http::Client::new(token));
    println!("Creating commands...");
    let my_id = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!")
        .id;
    cmd_defs::register(client.interaction(my_id)).await;
    let http = reqwest::Client::new();
    let svg = SvgState::new();
    let import_queue = ImportQueue::new();
    let state = AppState {
        db,
        pubkey,
        client,
        my_id,
        svg,
        http,
        redis,
        import_queue,
    };
    tokio::spawn(mee6_worker::do_fetches(state.clone()));
    let route = axum::Router::new()
        .route("/", axum::routing::get(|| async {}).post(handler::handle))
        .with_state(state);
    println!("Server listening on https://0.0.0.0:8080!");
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
        .serve(route.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            println!("Shutting down...");
        })
        .await
        .expect("failed to run server!");
    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub pubkey: Arc<String>,
    pub client: Arc<twilight_http::Client>,
    pub my_id: Id<ApplicationMarker>,
    pub svg: SvgState,
    pub http: reqwest::Client,
    pub redis: redis::aio::ConnectionManager,
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
