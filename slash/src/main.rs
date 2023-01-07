#![deny(clippy::all, clippy::cargo, clippy::pedantic, clippy::nursery)]
use std::sync::Arc;

use axum::routing::post;
use sqlx::MySqlPool;
use twilight_model::id::{marker::ApplicationMarker, Id};

mod cmd_defs;
mod discord_sig_validation;
mod handler;
mod manager;
mod processor;
mod render_card;

pub type AppState = Arc<UnderlyingAppState>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected environment variable DISCORD_TOKEN");
    let pubkey =
        std::env::var("DISCORD_PUBKEY").expect("Expected environment variable DISCORD_PUBKEY");
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    let mut fonts = resvg::usvg_text_layout::fontdb::Database::new();
    fonts.load_font_data(include_bytes!("resources/OpenSans.ttf").to_vec());
    let mut tera = tera::Tera::default();
    tera.add_raw_template("svg", include_str!("resources/card.svg"))?;
    let svg = SvgState { fonts, tera };
    println!("Connecting to database {database_url}");
    let db = MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    let client = twilight_http::Client::new(token);
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

    let state = Arc::new(UnderlyingAppState {
        db,
        pubkey,
        client,
        my_id,
        svg,
    });
    let route = axum::Router::new()
        .route("/", post(handler::handle))
        .with_state(state);
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
    pub client: twilight_http::Client,
    pub my_id: Id<ApplicationMarker>,
    pub svg: SvgState,
}

pub struct SvgState {
    fonts: resvg::usvg_text_layout::fontdb::Database,
    tera: tera::Tera,
}
