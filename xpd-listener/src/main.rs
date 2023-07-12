#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::sync::{atomic::AtomicBool, Arc};
use tokio::task::JoinSet;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_gateway::{CloseFrame, Config, Event, Intents, MessageSender, Shard};
use xpd_listener::XpdListener;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_env("LOG"))
        .init();
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
    let state = XpdListener::new(db, redis, http.clone());
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let mut set = JoinSet::new();

    for shard in shards {
        let http = http.clone();
        set.spawn(event_loop(
            shard,
            http,
            should_shutdown.clone(),
            state.clone(),
        ));
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

async fn event_loop(
    mut shard: Shard,
    http: Arc<twilight_http::Client>,
    should_shutdown: Arc<AtomicBool>,
    listener: XpdListener,
) {
    let sender = shard.sender();
    loop {
        let sender = sender.clone();
        match shard.next_event().await {
            Ok(event) => {
                let listener = listener.clone();
                let http = http.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_event(event, http, listener, sender).await {
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

async fn handle_event(
    event: Event,
    http: Arc<twilight_http::Client>,
    listener: XpdListener,
    shard: MessageSender,
) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => listener.save(*msg).await?,
        Event::GuildCreate(guild_add) => {
            shard.command(
                &twilight_model::gateway::payload::outgoing::RequestGuildMembers::builder(
                    guild_add.id,
                )
                .query("", None),
            )?;
            listener.set_guild(guild_add.0).await?;
        }
        Event::MemberAdd(member_add) => listener.set_user(&member_add.user).await?,
        Event::MemberUpdate(member_update) => listener.set_user(&member_update.user).await?,
        Event::MemberChunk(member_chunk) => listener.set_chunk(member_chunk.members).await?,
        Event::ThreadCreate(thread) => {
            let _ = http.join_thread(thread.id).await;
        }
        _ => {}
    };
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("listener-library error: {0}")]
    Listener(#[from] xpd_listener::Error),
    #[error("Twilight-Gateway error: {0}")]
    Send(#[from] twilight_gateway::error::SendError),
}
