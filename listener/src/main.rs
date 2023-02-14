#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod message;

use futures::StreamExt;
use sqlx::PgPool;
use std::sync::Arc;
use twilight_gateway::{
    stream::ShardEventStream, CloseFrame, Config, Event, Intents, MessageSender, Shard,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let token =
        std::env::var("DISCORD_TOKEN").expect("Failed to get DISCORD_TOKEN environment variable");
    let redis_url =
        std::env::var("REDIS_URL").expect("Failed to get REDIS_URL environment variable");
    let pg =
        std::env::var("DATABASE_URL").expect("Failed to get DATABASE_URL environment variable");
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
    let redis_cfg = deadpool_redis::Config::from_url(redis_url);
    let redis = redis_cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to connect to redis");

    let client = twilight_http::Client::new(token.clone());
    let intents = Intents::GUILD_MESSAGES;
    let config = Config::new(token, intents);

    let mut shards: Vec<Shard> =
        twilight_gateway::stream::create_recommended(&client, config, |_, builder| builder.build())
            .await
            .expect("Failed to create reccomended shard count")
            .collect();
    let shard_closers: Vec<MessageSender> = shards.iter().map(Shard::sender).collect();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen to ctrl-c");
        println!("Shutting down...");
        for shard in shard_closers {
            shard.close(CloseFrame::NORMAL).ok();
        }
    });
    let mut events = ShardEventStream::new(shards.iter_mut());
    println!("Connecting to discord");
    let client = Arc::new(client);
    while let Some((_shard, event_result)) = events.next().await {
        match event_result {
            Ok(event) => {
                let redis = match redis.get().await {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("ERROR: Fatal redis error: {e}");
                        return;
                    }
                };
                let client = client.clone();
                let db = db.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_event(event, db, redis, client).await {
                        eprintln!("Handler error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Shard loop error: {e}"),
        }
    }
    println!("Done, see ya!");
}

async fn handle_event(
    event: Event,
    db: PgPool,
    redis: deadpool_redis::Connection,
    http: Arc<twilight_http::Client>,
) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => message::save(*msg, db, redis, http).await,
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
    #[error("Send error: {0}")]
    TwilightCommand(#[from] twilight_gateway::error::SendError),
}
