#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod message;
mod user_cache;

use sqlx::PgPool;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::task::JoinSet;
use twilight_gateway::{CloseFrame, Config, Event, Intents, MessageSender, Shard};

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
    let intents = Intents::GUILD_MESSAGES | Intents::GUILDS | Intents::GUILD_MEMBERS;
    let config = Config::new(token.clone(), intents);

    let shards: Vec<Shard> =
        twilight_gateway::stream::create_recommended(&client, config, |_, builder| builder.build())
            .await
            .expect("Failed to create reccomended shard count")
            .collect();
    let senders: Vec<twilight_gateway::MessageSender> =
        shards.iter().map(twilight_gateway::Shard::sender).collect();
    let http = Arc::new(twilight_http::Client::new(token));
    println!("Connecting to discord");
    let state = AppState { db, redis, http };
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let mut set = JoinSet::new();

    for shard in shards {
        set.spawn_local(event_loop(shard, should_shutdown.clone(), state.clone()));
    }

    tokio::signal::ctrl_c().await.unwrap();

    eprintln!("Shutting down..");

    // Let the shards know not to reconnect
    should_shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

    // Tell the shards to shut down
    for sender in senders {
        sender.close(CloseFrame::NORMAL).ok();
    }

    // Await all tasks to complete.
    while set.join_next().await.is_some() {}
    println!("Done, see ya!");
}

async fn event_loop(mut shard: Shard, should_shutdown: Arc<AtomicBool>, state: AppState) {
    let sender = shard.sender();
    loop {
        let sender = sender.clone();
        match shard.next_event().await {
            Ok(event) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_event(event, state, sender).await {
                        // this includes even user caused errors. User beware. Don't set up automatic emails or anything.
                        eprintln!("Handler error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Shard loop error: {e}"),
        }
        if should_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
    }
}

async fn handle_event(event: Event, state: AppState, shard: MessageSender) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => message::save(*msg, state).await,
        Event::GuildCreate(guild_add) => {
            shard.command(
                &twilight_model::gateway::payload::outgoing::RequestGuildMembers::builder(
                    guild_add.id,
                )
                .query("", None),
            )?;
            user_cache::set_chunk(state.redis, guild_add.0.members).await
        }
        Event::MemberAdd(member_add) => user_cache::set_user(state.redis, &member_add.user).await,
        Event::MemberUpdate(member_update) => {
            user_cache::set_user(state.redis, &member_update.user).await
        }
        Event::MemberChunk(member_chunk) => {
            user_cache::set_chunk(state.redis, member_chunk.members).await
        }
        Event::ThreadCreate(thread) => state.http.join_thread(thread.id).await.map(|_| Ok(()))?,
        _ => Ok(()),
    }
}

#[derive(Clone)]
pub struct AppState {
    db: PgPool,
    redis: deadpool_redis::Pool,
    http: Arc<twilight_http::Client>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_redis::PoolError),
    #[error("Discord error: {0}")]
    Twilight(#[from] twilight_http::Error),
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Send error: {0}")]
    TwilightCommand(#[from] twilight_gateway::error::SendError),
}
