use std::sync::Arc;

use axum::routing::post;
use sqlx::{Connection, MySqlConnection};

mod cmd_defs;
mod discord_sig_validation;
mod handler;
pub type AppState = Arc<UnderlyingAppState>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let pubkey = std::env::var("DISCORD_PUBKEY")?;
    let db = MySqlConnection::connect(&std::env::var("DATABASE_URL")?)
        .await
        .expect("Failed to connect to the database!");
    let state = Arc::new(UnderlyingAppState {
        db,
        client: twilight_http::Client::new(token),
        pubkey,
    });
    let route = axum::Router::with_state(state).route("/", post(handler::handle));
    axum::Server::bind(&([0, 0, 0, 0], 8080).into()).serve(route.into_make_service());
    Ok(())
}

// mee6 algorithm: 5 * (lvl ^ 2) + (50 * lvl) + 100 - xp;
pub struct UnderlyingAppState {
    pub db: MySqlConnection,
    pub pubkey: String,
    pub client: twilight_http::Client,
}
