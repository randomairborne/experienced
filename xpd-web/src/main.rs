#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
};
use redis::AsyncCommands;
use sqlx::PgPool;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_env("LOG"))
        .init();
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    let redis_url = std::env::var("REDIS_URL").expect("Expected environment variable REDIS_URL");
    println!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    let cfg = deadpool_redis::Config::from_url(redis_url);
    let redis = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let mut tera = tera::Tera::default();
    tera.add_raw_templates([
        ("base.html", include_str!("resources/base.html")),
        ("index.html", include_str!("resources/index.html")),
        (
            "leaderboard.html",
            include_str!("resources/leaderboard.html"),
        ),
        ("terms.html", include_str!("resources/terms.html")),
        ("privacy.html", include_str!("resources/privacy.html")),
    ])
    .expect("Failed to add templates");
    let tera = Arc::new(tera);
    let route = axum::Router::new()
        .route("/", axum::routing::get(serve_index))
        .route("/privacy/", axum::routing::get(serve_privacy))
        .route("/terms/", axum::routing::get(serve_terms))
        .route(
            "/privacy",
            axum::routing::get(|| async { Redirect::to("/privacy/") }),
        )
        .route(
            "/terms",
            axum::routing::get(|| async { Redirect::to("/terms/") }),
        )
        .route(
            "/main.css",
            axum::routing::get(|| async {
                (
                    [("Content-Type", "text/css")],
                    include_bytes!("resources/main.css").as_slice(),
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
                    include_bytes!("resources/MontserratAlt1.woff").as_slice(),
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
                    include_bytes!("resources/MontserratAlt1.woff2").as_slice(),
                )
            }),
        )
        .route(
            "/favicon.png",
            axum::routing::get(|| async {
                (
                    [("Content-Type", "image/png")],
                    include_bytes!("resources/favicon.png").as_slice(),
                )
            }),
        )
        .route(
            "/robots.txt",
            axum::routing::get(|| async { "User-Agent: *\nAllow: /$\nDisallow: /" }),
        )
        .route("/:id", axum::routing::get(fetch_stats))
        .with_state(AppState { db, redis, tera });
    println!("Server listening on https://0.0.0.0:8000!");
    axum::Server::bind(&([0, 0, 0, 0], 8000).into())
        .serve(route.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            println!("Shutting down...");
        })
        .await
        .expect("failed to run server!");
}

#[derive(Clone)]
struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub tera: Arc<tera::Tera>,
}

#[derive(serde::Serialize, Debug)]
struct User {
    id: u64,
    level: u64,
    name: Option<String>,
    discriminator: Option<u16>,
}

#[derive(serde::Deserialize)]
pub struct FetchQuery {
    offset: Option<i64>,
}

async fn fetch_stats(
    Path(guild_id): Path<u64>,
    State(state): State<AppState>,
    Query(query): Query<FetchQuery>,
) -> Result<Html<String>, Error> {
    let offset = query.offset.unwrap_or(0);
    #[allow(clippy::cast_possible_wrap)]
    let user_rows = sqlx::query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT 100 OFFSET $2",
        guild_id as i64,
        offset * 100
    )
    .fetch_all(&state.db)
    .await?;
    let mut ids_to_indices: HashMap<u64, usize> = HashMap::with_capacity(user_rows.len());
    let mut users: Vec<User> = user_rows
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            #[allow(clippy::cast_sign_loss)]
            ids_to_indices.insert(v.id as u64, i);
            #[allow(clippy::cast_sign_loss)]
            User {
                id: v.id as u64,
                level: mee6::LevelInfo::new(v.xp as u64).level(),
                name: None,
                discriminator: None,
            }
        })
        .collect();
    let user_strings: Vec<Option<String>> = if users.is_empty() {
        Vec::new()
    } else {
        state
            .redis
            .get()
            .await?
            .mget(
                users
                    .iter()
                    .map(|v| format!("cache-user-{}", v.id))
                    .collect::<Vec<String>>(),
            )
            .await?
    };
    for user_string in user_strings.into_iter().flatten() {
        let user: xpd_common::RedisUser = match serde_json::from_str(&user_string) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        if let Some(i) = ids_to_indices.get(&user.id) {
            users[*i].discriminator = user.discriminator;
            users[*i].name = user.username;
        }
    }
    let mut context = tera::Context::new();
    context.insert("users", &users);
    context.insert("offset", &offset);
    context.insert("guild", &guild_id);
    let rendered = state.tera.render("leaderboard.html", &context)?;
    Ok(Html(rendered))
}

#[allow(clippy::unused_async)]
async fn serve_index(State(state): State<AppState>) -> Result<Html<String>, Error> {
    Ok(Html(
        state.tera.render("index.html", &tera::Context::new())?,
    ))
}

#[allow(clippy::unused_async)]
async fn serve_privacy(State(state): State<AppState>) -> Result<Html<String>, Error> {
    Ok(Html(
        state.tera.render("privacy.html", &tera::Context::new())?,
    ))
}

#[allow(clippy::unused_async)]
async fn serve_terms(State(state): State<AppState>) -> Result<Html<String>, Error> {
    Ok(Html(
        state.tera.render("terms.html", &tera::Context::new())?,
    ))
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Templating error: {0}")]
    Tera(#[from] tera::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Redis pool error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),
    #[error("Non-integer value where integer expected: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        self.to_string().into_response()
    }
}
