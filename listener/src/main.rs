#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use futures::stream::StreamExt;
use sqlx::PgPool;
use std::{env, sync::Arc};
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
};
mod message;
mod user_cache;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let token =
        env::var("DISCORD_TOKEN").expect("Failed to get DISCORD_TOKEN environment variable");
    let scheme = ShardScheme::Range {
        from: env::var("SHARDS_START")
            .expect("Failed to get SHARDS_START environment variable")
            .parse()
            .expect("Failed to parse SHARDS_START as u64"),
        to: env::var("SHARDS_END")
            .expect("Failed to get SHARDS_END environment variable")
            .parse()
            .expect("Failed to parse SHARDS_END as u64"),
        total: env::var("TOTAL_SHARDS")
            .expect("Failed to get TOTAL_SHARDS environment variable")
            .parse()
            .expect("Failed to parse TOTAL_SHARDS as u64"),
    };
    let redis_url = env::var("REDIS_URL").expect("Failed to get REDIS_URL environment variable");
    let pg = env::var("DATABASE_URL").expect("Failed to get DATABASE_URL environment variable");
    println!("Connecting to database {pg}");
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&pg)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");

    let redis = redis::aio::ConnectionManager::new(
        redis::Client::open(redis_url).expect("Failed to connect to redis"),
    )
    .await
    .expect("Failed to create connection manager");

    let client = Arc::new(twilight_http::Client::new(token.clone()));

    let intents = Intents::GUILD_MESSAGES | Intents::GUILD_MEMBERS | Intents::GUILD_PRESENCES;

    let (cluster, mut events) = Cluster::builder(token.clone(), intents)
        .shard_scheme(scheme)
        .build()
        .await
        .expect("Failed to create discord cluster");

    let cluster = Arc::new(cluster);

    let cluster_spawn = cluster.clone();
    println!("Connecting to discord");
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen to ctrl-c");
        println!("Shutting down...");
        cluster.down();
    });

    let mut has_connected = false;
    while let Some((_shard_id, event)) = events.next().await {
        if !has_connected {
            has_connected = true;
            println!("Connected to discord!");
        }
        let redis = redis.clone();
        let client = client.clone();
        let db = db.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_event(event, db, redis, client).await {
                eprintln!("Error: {e}")
            }
        });
    }
    println!("Done, see ya!");
}

async fn handle_event(
    event: Event,
    db: PgPool,
    mut redis: redis::aio::ConnectionManager,
    http: Arc<twilight_http::Client>,
) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => Ok(message::save(*msg, db, redis, http).await),
        Event::GuildCreate(guild_add) => {
            user_cache::set_chunk(&mut redis, guild_add.0.members).await
        }
        Event::MemberAdd(member_add) => user_cache::set_user(&mut redis, member_add.0.user).await,
        Event::MemberUpdate(member_update) => {
            user_cache::set_user(&mut redis, member_update.user).await
        }
        Event::MemberChunk(member_chunk) => {
            user_cache::set_chunk(&mut redis, member_chunk.members).await
        }
        Event::ThreadCreate(thread) => http.join_thread(thread.id).await.map(|_| Ok(()))?,
        _ => Ok(()),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Discord error: {0}")]
    Twilight(#[from] twilight_http::Error),
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}
