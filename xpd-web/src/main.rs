#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod error;
mod leaderboard;

use std::sync::Arc;

use axum::{handler::Handler, response::Html, routing::get};
use axum_extra::routing::RouterExt;
pub use error::Error;
use error::HttpError;
use sqlx::PgPool;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[macro_use]
extern crate tracing;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    let redis_url = std::env::var("REDIS_URL").expect("Expected environment variable REDIS_URL");
    let raw_root_url = std::env::var("ROOT_URL").expect("Expected environment variable ROOT_URL");
    let root_url = Arc::new(raw_root_url.trim_end_matches('/').to_string());
    info!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    let cfg = deadpool_redis::Config::from_url(redis_url);
    let redis = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();
    redis.get().await.expect("Failed to connect to redis");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let tera =
        Arc::new(tera::Tera::new("./templates/**/*.html").expect("Failed to build templates"));
    let state = AppState {
        db,
        redis,
        tera,
        root_url,
    };
    let serve_dir = tower_http::services::ServeDir::new("./static/")
        .append_index_html_on_directories(false)
        .not_found_service(crate::basic_handler!("404.html").with_state(state.clone()));
    let route = axum::Router::new()
        .route("/", get(crate::basic_handler!("index.html")))
        .route_with_tsr("/privacy/", get(crate::basic_handler!("privacy.html")))
        .route_with_tsr("/terms/", get(crate::basic_handler!("terms.html")))
        .route_with_tsr("/leaderboard/:id", get(leaderboard::fetch_stats))
        .route("/robots.txt", get(crate::basic_handler!("robots.txt")))
        .route("/sitemap.txt", get(crate::basic_handler!("sitemap.txt")))
        .fallback_service(serve_dir)
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(state);
    info!("Server listening on https://0.0.0.0:8080!");
    #[allow(clippy::redundant_pub_crate)]
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
        .serve(route.into_make_service())
        .with_graceful_shutdown(async {
            xpd_common::wait_for_shutdown().await;
            warn!("Shutting down...");
        })
        .await
        .expect("failed to run server!");
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub tera: Arc<tera::Tera>,
    pub root_url: Arc<String>,
}

#[macro_export]
macro_rules! basic_handler {
    ($template:expr) => {{
        #[allow(clippy::unused_async)]
        async fn __basic_generated_handler(
            ::axum::extract::State(state): ::axum::extract::State<AppState>,
        ) -> Result<Html<String>, HttpError> {
            let mut context = ::tera::Context::new();
            context.insert("root_url", &state.root_url);
            Ok(Html(state.tera.render($template, &context).map_err(
                |e| {
                    #[allow(clippy::crate_in_macro_def)]
                    crate::HttpError::new(e.into(), state)
                },
            )?))
        }
        __basic_generated_handler
    }};
}
