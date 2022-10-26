#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
use std::sync::Arc;

use axum::routing::post;
use sqlx::MySqlPool;

mod cmd_defs;
mod discord_sig_validation;
mod handler;
mod processor;

pub type AppState = Arc<UnderlyingAppState>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected environment variable DISCORD_TOKEN");
    let pubkey =
        std::env::var("DISCORD_PUBKEY").expect("Expected environment variable DISCORD_PUBKEY");
    println!("Connecting to the database..");
    let db = MySqlPool::connect(
        &std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL"),
    )
    .await
    .expect("Failed to connect to the database!");
    let client = twilight_http::Client::new(token);
    println!("Creating commands...");
    cmd_defs::register(
        client.interaction(
            client
                .current_user_application()
                .exec()
                .await
                .expect("Failed to get own app ID!")
                .model()
                .await
                .expect("Failed to convert own app ID!")
                .id,
        ),
    )
    .await;
    let state = Arc::new(UnderlyingAppState { db, pubkey });
    let route = axum::Router::with_state(state).route("/", post(handler::handle));
    println!("Server listening on https://0.0.0.0:8080!");
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
        .serve(route.into_make_service())
        .await
        .expect("failed to run server!");
    Ok(())
}

pub struct UnderlyingAppState {
    pub db: MySqlPool,
    pub pubkey: String,
}
