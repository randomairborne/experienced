#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::sync::{atomic::AtomicBool, Arc};
use tokio::task::JoinSet;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_gateway::{CloseFrame, Config, Event, Intents, Shard};
use twilight_model::http::interaction::InteractionResponse;
use xpd_listener::XpdListener;
use xpd_slash::XpdSlash;

#[macro_use]
extern crate tracing;

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
    let intents = Intents::GUILD_MESSAGES | Intents::GUILDS;
    let my_id = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!")
        .id;
    let config = Config::new(token.clone(), intents);
    let shards: Vec<Shard> =
        twilight_gateway::stream::create_recommended(&client, config, |_, builder| builder.build())
            .await
            .expect("Failed to create reccomended shard count")
            .collect();
    let senders: Vec<twilight_gateway::MessageSender> =
        shards.iter().map(twilight_gateway::Shard::sender).collect();
    let client = Arc::new(twilight_http::Client::new(token));
    println!("Connecting to discord");
    let http = reqwest::Client::new();
    let listener = XpdListener::new(db.clone(), redis.clone(), client.clone());
    let slash = XpdSlash::new(http, client.clone(), my_id, db, redis, None).await;
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let mut set = JoinSet::new();

    for shard in shards {
        let client = client.clone();
        set.spawn(event_loop(
            shard,
            client,
            should_shutdown.clone(),
            listener.clone(),
            slash.clone(),
        ));
    }

    xpd_common::wait_for_shutdown().await;

    warn!("Shutting down..");

    // Let the shards know not to reconnect
    should_shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

    // Tell the shards to shut down
    for sender in senders {
        sender.close(CloseFrame::NORMAL).ok();
    }

    // Await all tasks to complete.
    while set.join_next().await.is_some() {}
    info!("Done, see ya!");
}

async fn event_loop(
    mut shard: Shard,
    http: Arc<twilight_http::Client>,
    should_shutdown: Arc<AtomicBool>,
    listener: XpdListener,
    slash: XpdSlash,
) {
    loop {
        match shard.next_event().await {
            Ok(event) => {
                let listener = listener.clone();
                let http = http.clone();
                let slash = slash.clone();
                tokio::spawn(async move {
                    if let Err(error) = handle_event(event, http, listener, slash).await {
                        // this includes even user caused errors. User beware. Don't set up automatic emails or anything.
                        error!(?error, "Handler error");
                    }
                });
            }
            Err(error) => error!(?error, "Shard loop error"),
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
    slash: XpdSlash,
) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => listener.save(*msg).await?,
        Event::ThreadCreate(thread) => {
            let _ = http.join_thread(thread.id).await;
        }
        Event::InteractionCreate(interaction_create) => {
            let interaction_token = interaction_create.token.clone();
            if let Err(error) = slash.client().interaction(slash.id()).create_response(interaction_create.id, &interaction_create.token, &InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::DeferredChannelMessageWithSource,
                data: None
            }).await {
                error!(?error, "Failed to ack discord gateway message");
            };
            let response = slash.clone().run(interaction_create.0).await;
            if let Err(error) = slash.send_followup(response, &interaction_token).await {
                error!(?error, "Failed to send real response");
            };
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
    #[error("Twilight-Validate error: {0}")]
    Validate(#[from] twilight_validate::message::MessageValidationError),
}
