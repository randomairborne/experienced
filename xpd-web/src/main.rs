#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod error;
mod files;

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
    routing::get,
};
use error::HttpError;
use redis::AsyncCommands;
use sqlx::PgPool;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

pub use error::Error;
use xpd_common::{RedisGuild, RedisUser};

#[macro_use]
extern crate tracing;

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
    info!("Connecting to database {database_url}");
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
        ("404.html", include_str!("resources/404.html")),
        ("5xx.html", include_str!("resources/5xx.html")),
        (
            "leaderboard.html",
            include_str!("resources/leaderboard.html"),
        ),
        ("terms.html", include_str!("resources/terms.html")),
        ("privacy.html", include_str!("resources/privacy.html")),
        ("sitemap.txt", include_str!("resources/sitemap.txt")),
        ("robots.txt", include_str!("resources/robots.txt")),
    ])
    .expect("Failed to add templates");
    let tera = Arc::new(tera);
    let route = axum::Router::new()
        .route("/", get(files::serve_index))
        .route("/privacy/", get(files::serve_privacy))
        .route("/terms/", get(files::serve_terms))
        .route("/privacy", get(|| async { Redirect::to("/privacy/") }))
        .route("/terms", get(|| async { Redirect::to("/terms/") }))
        .route("/main.css", get(files::serve_css))
        .route("/MontserratAlt1.woff2", get(files::serve_font))
        .route("/favicon.png", get(files::serve_icon))
        .route("/logo.png", get(files::serve_logo))
        .route("/robots.txt", get(files::serve_robots))
        .route("/sitemap.txt", get(files::serve_sitemap))
        .route("/leaderboard/:id", get(fetch_stats))
        .fallback(files::serve_404)
        .with_state(AppState {
            db,
            redis,
            tera,
            root_url,
        });
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

#[derive(serde::Serialize, Debug)]
pub struct User {
    id: u64,
    level: u64,
    name: Option<String>,
    discriminator: Option<u16>,
}

const PAGE_SIZE: i64 = 50;

#[derive(serde::Deserialize)]
pub struct FetchQuery {
    #[serde(default = "get_0")]
    page: i64,
}

const fn get_0() -> i64 {
    0
}

async fn fetch_stats(
    weird_guild_id: Option<Path<Id<GuildMarker>>>,
    State(state): State<AppState>,
    Query(query): Query<FetchQuery>,
) -> Result<Html<String>, HttpError> {
    let Some(Path(guild_id)) = weird_guild_id else {
        return Err(HttpError::new(Error::NoLeveling, state));
    };
    let offset = query.page * PAGE_SIZE;
    let guild = get_redis_guild(&state, guild_id)
        .await
        .map_err(|e| HttpError::new(e, state.clone()))?;
    #[allow(clippy::cast_possible_wrap)]
    let user_rows = sqlx::query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT $2 OFFSET $3",
        guild_id.get() as i64,
        PAGE_SIZE + 1,
        offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| HttpError::new(e.into(), state.clone()))?;
    if user_rows.is_empty() {
        return Err(HttpError::new(Error::NoLeveling, state));
    }
    #[allow(clippy::cast_possible_truncation)]
    let has_next_page = user_rows.len() > PAGE_SIZE as usize;
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
            .await
            .map_err(|e| HttpError::new(e.into(), state.clone()))?
            .mget(
                users
                    .iter()
                    .map(|v| format!("cache-user-{}", v.id))
                    .collect::<Vec<String>>(),
            )
            .await
            .map_err(|e| HttpError::new(e.into(), state.clone()))?
    };
    // if we have 51 users, the 51st user is the first user on the next page
    if has_next_page {
        users.pop();
    }
    for user_string in user_strings.into_iter().flatten() {
        let user: RedisUser = match serde_json::from_str(&user_string) {
            Ok(v) => v,
            Err(source) => {
                error!(?source, "Failed to deserialize user from redis");
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
    context.insert("page", &query.page);
    context.insert("guild", &guild);
    context.insert("root_url", &state.root_url);
    context.insert("has_next_page", &has_next_page);
    let rendered = state
        .tera
        .render("leaderboard.html", &context)
        .map_err(|e| HttpError::new(e.into(), state))?;
    Ok(Html(rendered))
}

async fn get_redis_guild(state: &AppState, guild: Id<GuildMarker>) -> Result<RedisGuild, Error> {
    let maybe_guild_string: Option<String> = state
        .redis
        .get()
        .await?
        .get(format!("cache-guild-{guild}"))
        .await?;
    Ok(if let Some(guild_string) = maybe_guild_string {
        serde_json::from_str(&guild_string)?
    } else {
        RedisGuild {
            id: guild,
            name: "(name not in cache)".to_string(),
            banner_hash: None,
            icon_hash: None,
        }
    })
}
