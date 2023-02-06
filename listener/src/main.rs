#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod message;
mod user_cache;

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

    let redis = redis::aio::ConnectionManager::new(
        redis::Client::open(redis_url).expect("Failed to connect to redis"),
    )
    .await
    .expect("Failed to create connection manager");

    let client = twilight_http::Client::new(token.clone());
    let intents = Intents::GUILD_MESSAGES | Intents::GUILD_MEMBERS | Intents::GUILDS;
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
    while let Some((shard, event_result)) = events.next().await {
        match event_result {
            Ok(event) => {
                let redis = redis.clone();
                let client = client.clone();
                let db = db.clone();
                let shard = shard.sender();
                tokio::spawn(async move {
                    if let Err(e) = handle_event(event, db, redis, client, shard).await {
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
    mut redis: redis::aio::ConnectionManager,
    http: Arc<twilight_http::Client>,
    shard: MessageSender,
) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => message::save(*msg, db, redis, http).await,
        Event::GuildCreate(guild_add) => {
            shard.command(
                &twilight_model::gateway::payload::outgoing::RequestGuildMembers::builder(
                    guild_add.id,
                )
                .query("", None),
            )?;
            user_cache::set_chunk(&mut redis, guild_add.0.members).await
        }
        Event::MemberAdd(member_add) => user_cache::set_user(&mut redis, &member_add.user).await,
        Event::MemberUpdate(member_update) => {
            user_cache::set_user(&mut redis, &member_update.user).await
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
    #[error("Send error: {0}")]
    TwilightCommand(#[from] twilight_gateway::error::SendError),
}
