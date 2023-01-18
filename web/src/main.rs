use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
};
use sqlx::PgPool;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_env("LOG"))
        .init();
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    let mut tera = tera::Tera::default();
    tera.add_raw_template("leaderboard.html", include_str!("leaderboard.html"))
        .expect("Failed to add template");
    println!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let route = axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async { Html(include_bytes!("homepage.html").as_slice()) }),
        )
        .route(
            "/homepage.css",
            axum::routing::get(|| async {
                (
                    [("Content-Type", "text/css")],
                    include_bytes!("homepage.css").as_slice(),
                )
            }),
        )
        .route(
            "/leaderboard.css",
            axum::routing::get(|| async {
                (
                    [("Content-Type", "text/css")],
                    include_bytes!("leaderboard.css").as_slice(),
                )
            }),
        )
        .route(
            "/MontserratAlt1.woff",
            axum::routing::get(|| async {
                (
                    [
                        ("Content-Type", "font/woff"),
                        ("Cache-Control", "max-age=31536000"),
                    ],
                    include_bytes!("MontserratAlt1.woff").as_slice(),
                )
            }),
        )
        .route(
            "/MontserratAlt1.woff2",
            axum::routing::get(|| async {
                (
                    [
                        ("Content-Type", "font/woff2"),
                        ("Cache-Control", "max-age=31536000"),
                    ],
                    include_bytes!("MontserratAlt1.woff2").as_slice(),
                )
            }),
        )
        .route(
            "/favicon.png",
            axum::routing::get(|| async {
                (
                    [("Content-Type", "image/png")],
                    include_bytes!("favicon.png").as_slice(),
                )
            }),
        )
        .route(
            "/robots.txt",
            axum::routing::get(|| async { "User-Agent: *\nAllow: /\nDisallow: /*" }),
        )
        .route("/:id", axum::routing::get(fetch_stats))
        .with_state(AppState {
            db,
            tera: Arc::new(tera),
        });
    println!("Server listening on https://0.0.0.0:8080!");
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
        .serve(route.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            println!("Shutting down...");
        })
        .await
        .expect("failed to run server!");
}

#[derive(serde::Serialize)]
struct User {
    id: i64,
    level: u64,
}

async fn fetch_stats(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, Error> {
    let user_rows = sqlx::query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT 100",
        i64::from_str_radix(&id, 10)?
    )
    .fetch_all(&state.db)
    .await?;
    let users: Vec<User> = user_rows
        .into_iter()
        .map(|v| User {
            id: v.id,
            level: mee6::LevelInfo::new(v.xp as u64).level(),
        })
        .collect();
    let mut context = tera::Context::new();
    context.insert("users", &users);
    Ok(Html(state.tera.render("leaderboard.html", &context)?))
}

#[derive(Clone)]
struct AppState {
    pub db: PgPool,
    pub tera: Arc<tera::Tera>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Templating error: {0}")]
    Tera(#[from] tera::Error),
    #[error("Non-integer value where integer expected: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        self.to_string().into_response()
    }
}
