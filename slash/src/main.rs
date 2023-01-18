#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
use std::sync::Arc;

use sqlx::PgPool;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_model::id::{marker::ApplicationMarker, Id};

mod cmd_defs;
#[macro_use]
mod colors;
mod discord_sig_validation;
mod handler;
mod help;
mod levels;
mod manage_card;
mod manager;
mod processor;
mod render_card;

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
    let pubkey =
        std::env::var("DISCORD_PUBKEY").expect("Expected environment variable DISCORD_PUBKEY");
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    let redis_url = std::env::var("REDIS_URL").expect("Expected environment variable REDIS_URL");
    let mut fonts = resvg::usvg_text_layout::fontdb::Database::new();
    fonts.load_font_data(include_bytes!("resources/Mojang.ttf").to_vec());
    fonts.load_font_data(include_bytes!("resources/Roboto.ttf").to_vec());
    fonts.load_font_data(include_bytes!("resources/JetBrainsMono.ttf").to_vec());
    fonts.load_font_data(include_bytes!("resources/MontserratAlt1.ttf").to_vec());
    let mut tera = tera::Tera::default();
    tera.add_raw_template("svg", include_str!("resources/card.svg"))?;
    let svg = SvgState { fonts, tera };
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
    let http = reqwest::Client::new();
    let state = AppState {
        db,
        pubkey: pubkey.into(),
        client: client.into(),
        my_id,
        svg: svg.into(),
        http,
        redis,
    };
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
    pub svg: Arc<SvgState>,
    pub http: reqwest::Client,
    pub redis: redis::aio::ConnectionManager,
}

pub struct SvgState {
    fonts: resvg::usvg_text_layout::fontdb::Database,
    tera: tera::Tera,
}
