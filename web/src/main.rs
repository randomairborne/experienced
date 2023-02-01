use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
};
use redis::{aio::ConnectionManager, AsyncCommands};
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
    let redis_url = std::env::var("REDIS_URL").expect("Expected environment variable REDIS_URL");
    println!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    let redis = redis::aio::ConnectionManager::new(
        redis::Client::open(redis_url).expect("Failed to connect to redis"),
    )
    .await
    .expect("Redis connection manager creation failed");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let mut tera = tera::Tera::default();
    tera.add_raw_template("leaderboard.html", include_str!("leaderboard.html"))
        .expect("Failed to add leaderboard");
    let tera = Arc::new(tera);
    let route = axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async { Html(include_bytes!("index.html").as_slice()) }),
        )
        .route(
            "/privacy",
            axum::routing::get(|| async { Redirect::to("/privacy/") }),
        )
        .route(
            "/privacy/",
            axum::routing::get(|| async { Html(include_bytes!("privacy.html").as_slice()) }),
        )
        .route(
            "/terms",
            axum::routing::get(|| async { Redirect::to("/terms/") }),
        )
        .route(
            "/terms/",
            axum::routing::get(|| async { Html(include_bytes!("terms.html").as_slice()) }),
        )
        .route(
            "/main.css",
            axum::routing::get(|| async {
                (
                    [("Content-Type", "text/css")],
                    include_bytes!("main.css").as_slice(),
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
            axum::routing::get(|| async { "User-Agent: *\nAllow: /$\nDisallow: /" }),
        )
        .route("/:id", axum::routing::get(fetch_stats))
        .with_state(AppState { db, tera, redis });
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

#[derive(Clone)]
struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub tera: Arc<tera::Tera>,
}

#[derive(serde::Serialize)]
struct User {
    id: u64,
    level: u64,
    name: Option<String>,
    discriminator: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct FetchQuery {
    offset: Option<i64>,
}

async fn fetch_stats(
    Path(guild_id): Path<u64>,
    State(mut state): State<AppState>,
    Query(query): Query<FetchQuery>,
) -> Result<Html<String>, Error> {
    let offset = query.offset.unwrap_or(0);
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
            ids_to_indices.insert(v.id as u64, i);
            User {
                id: v.id as u64,
                level: mee6::LevelInfo::new(v.xp as u64).level(),
                name: None,
                discriminator: None,
            }
        })
        .collect();
    let maybe_user_strings: Option<Vec<String>> = state
        .redis
        .get(users.iter().map(|v| v.id).collect::<Vec<u64>>())
        .await?;
    if let Some(user_strings) = maybe_user_strings {
        for user_string in user_strings {
            let user: twilight_model::user::User = match serde_json::from_str(&user_string) {
                Ok(v) => v,
                Err(_e) => continue,
            };
            if let Some(i) = ids_to_indices.get(&user.id.get()) {
                users[*i].discriminator = Some(format!("{}", user.discriminator()));
                users[*i].name = Some(user.name);
            }
        }
    }
    let mut context = tera::Context::new();
    context.insert("users", &users);
    context.insert("offset", &offset);
    let rendered = state.tera.render("leaderboard.html", &context)?;
    Ok(Html(rendered))
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Templating error: {0}")]
    Tera(#[from] tera::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Non-integer value where integer expected: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        self.to_string().into_response()
    }
}
