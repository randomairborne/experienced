#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod error;
mod leaderboard;

use std::{net::SocketAddr, sync::Arc};

use axum::{handler::Handler, routing::get};
use axum_extra::routing::RouterExt;
use error::HttpError;
use sqlx::PgPool;
use tokio::net::TcpListener;
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
    let database_url = xpd_common::get_var("DATABASE_URL");
    let redis_url = xpd_common::get_var("REDIS_URL");
    let asset_dir = xpd_common::get_var("ASSET_DIR");
    let template_dir = {
        let template_dir = xpd_common::get_var("TEMPLATE_DIR");
        template_dir.trim_end_matches('/').to_owned()
    };
    let root_url = {
        let raw_root_url = xpd_common::get_var("ROOT_URL");
        Arc::new(raw_root_url.trim_end_matches('/').to_string())
    };
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
    let tera = Arc::new(
        tera::Tera::new(&format!("{template_dir}/**/*")).expect("Failed to build templates"),
    );
    let state = AppState {
        db,
        redis,
        tera,
        root_url,
    };
    let serve_dir = tower_http::services::ServeDir::new(asset_dir)
        .append_index_html_on_directories(false)
        .not_found_service(crate::basic_handler!("404.html").with_state(state.clone()));
    let app = axum::Router::new()
        .route("/", get(crate::basic_handler!("index.html")))
        .route_with_tsr("/privacy/", get(crate::basic_handler!("privacy.html")))
        .route_with_tsr("/terms/", get(crate::basic_handler!("terms.html")))
        .route_with_tsr("/leaderboard/:id", get(leaderboard::fetch_stats))
        .route("/robots.txt", get(crate::basic_handler!("robots.txt")))
        .route("/sitemap.txt", get(crate::basic_handler!("sitemap.txt")))
        .fallback_service(serve_dir)
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(state);
    info!("Server listening on http://0.0.0.0:8080!");
    let bind_address = SocketAddr::from(([0, 0, 0, 0], 8080));
    let tcp = TcpListener::bind(bind_address).await.unwrap();
    axum::serve(tcp, app)
        .with_graceful_shutdown(vss::shutdown_signal())
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
            ::axum::extract::State(state): ::axum::extract::State<$crate::AppState>,
        ) -> ::axum::response::Response {
            use ::axum::response::IntoResponse;
            let mut context = ::tera::Context::new();
            context.insert("root_url", &state.root_url);
            let rendered = match state.tera.render($template, &context) {
                Ok(v) => v,
                Err(e) => return $crate::HttpError::new(e.into(), state).into_response(),
            };
            if $template.ends_with(".html") {
                ::axum::response::Html(rendered).into_response()
            } else {
                rendered.into_response()
            }
        }
        __basic_generated_handler
    }};
}
