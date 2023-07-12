#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod discord_sig_validation;
mod handler;

use sqlx::PgPool;
use std::sync::Arc;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

use xpd_slash::XpdSlash;

#[macro_use]
extern crate tracing;

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
    let root_url = std::env::var("ROOT_URL")
        .expect("Expected environment variable REDIS_URL")
        .trim_end_matches('/')
        .to_string();
    println!("Connecting to redis {redis_url}");
    let redis_cfg = deadpool_redis::Config::from_url(redis_url);
    let redis = redis_cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to connect to redis");
    println!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let client = Arc::new(twilight_http::Client::new(token));
    let my_id = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!")
        .id;
    let http = reqwest::Client::new();
    let state = AppState {
        pubkey,
        bot: XpdSlash::new(http, client, my_id, db, redis, Some(root_url)).await,
    };
    let route = axum::Router::new()
        .route("/", axum::routing::get(|| async {}).post(handler::handle))
        .with_state(state);
    println!("Server listening on https://0.0.0.0:8080!");
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
        .serve(route.into_make_service())
        .with_graceful_shutdown(async {
            xpd_common::wait_for_shutdown().await;
            println!("Shutting down...");
        })
        .await
        .expect("failed to run server!");
    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub pubkey: Arc<String>,
    pub bot: XpdSlash,
}
