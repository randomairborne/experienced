#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
};
use redis::AsyncCommands;
use sqlx::PgPool;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

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
    let raw_root_url = std::env::var("ROOT_URL").expect("Expected environment variable ROOT_URL");
    let root_url = Arc::new(raw_root_url.trim_end_matches('/').to_string());
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
            axum::routing::get(|| async { "User-Agent: *\nAllow: /" }),
        )
        .route("/:id", axum::routing::get(fetch_stats))
        .with_state(AppState {
            db,
            redis,
            tera,
            root_url,
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

#[derive(Clone)]
struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub tera: Arc<tera::Tera>,
    pub root_url: Arc<String>,
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
    page: Option<i64>,
}

async fn fetch_stats(
    Path(guild_id): Path<Id<GuildMarker>>,
    State(state): State<AppState>,
    Query(query): Query<FetchQuery>,
) -> Result<Html<String>, Error> {
    const PAGE_SIZE: i64 = 50;
    let page = query.page.unwrap_or(0);
    let offset = page * PAGE_SIZE;
    #[allow(clippy::cast_possible_wrap)]
    let user_rows = sqlx::query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT 51 OFFSET $2",
        guild_id.get() as i64,
        offset
    )
    .fetch_all(&state.db)
    .await?;
    if user_rows.is_empty() {
        return Err(Error::NoLeveling);
    }
    let has_next_page = user_rows.len() >= 51;
    let maybe_guild_string: Option<String> = state
        .redis
        .get()
        .await?
        .get(format!("cache-guild-{guild_id}"))
        .await?;
    let guild: xpd_common::RedisGuild = if let Some(guild_string) = maybe_guild_string {
        serde_json::from_str(&guild_string)?
    } else {
        xpd_common::RedisGuild {
            id: guild_id,
            name: "(name not in cache)".to_string(),
            banner_hash: None,
            icon_hash: None,
        }
    };
    let mut ids_to_indices: HashMap<Id<UserMarker>, usize> =
        HashMap::with_capacity(user_rows.len());
    let mut users: Vec<User> = user_rows
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            #[allow(clippy::cast_sign_loss)]
            ids_to_indices.insert(Id::new(v.id as u64), i);
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
    // if we have 51 users, the 51st user is the first user on the next page
    if has_next_page {
        users.pop();
    }
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
    context.insert("page", &page);
    context.insert("guild", &guild);
    context.insert("root_url", &state.root_url);
    context.insert("has_next_page", &has_next_page);
    let rendered = state.tera.render("leaderboard.html", &context)?;
    Ok(Html(rendered))
}

#[allow(clippy::unused_async)]
async fn serve_index(State(state): State<AppState>) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(state.tera.render("index.html", &context)?))
}

#[allow(clippy::unused_async)]
async fn serve_privacy(State(state): State<AppState>) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(state.tera.render("privacy.html", &context)?))
}

#[allow(clippy::unused_async)]
async fn serve_terms(State(state): State<AppState>) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(state.tera.render("terms.html", &context)?))
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
    ParseInt(#[from] std::num::ParseIntError),
    #[error("JSON deserialization failed: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("This server does not have Experienced, or no users have leveled up.")]
    NoLeveling,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        eprintln!("{self:?}");
        self.to_string().into_response()
    }
}
