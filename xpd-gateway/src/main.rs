#![deny(clippy::all)]

#[macro_use]
extern crate tracing;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use sqlx::PgPool;
use tokio_util::task::TaskTracker;
use tracing::Level;
use twilight_gateway::{
    error::ReceiveMessageErrorType, CloseFrame, Config, Event, EventTypeFlags, Intents,
    MessageSender, Shard, StreamExt,
};
use twilight_http::Client as DiscordClient;
use twilight_model::{
    gateway::ShardId,
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use xpd_common::{id_to_db, RequiredEvents};
use xpd_listener::XpdListener;
use xpd_slash::{InvalidateCache, UpdateChannels, XpdSlash};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .json()
        .init();
    let token = xpd_common::get_var("DISCORD_TOKEN");
    let pg = xpd_common::get_var("DATABASE_URL");
    let control_guild: Id<GuildMarker> = xpd_common::parse_var("CONTROL_GUILD");
    let owners: Vec<Id<UserMarker>> = xpd_common::get_var("OWNERS")
        .split(',')
        .map(|v| {
            v.trim()
                .parse::<Id<UserMarker>>()
                .expect("One of the values in OWNERS was not a valid ID!")
        })
        .collect();
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&pg)
        .await
        .expect("Failed to connect to database");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations!");

    let client = Arc::new(DiscordClient::new(token.clone()));
    let intents = XpdListener::required_intents() | XpdSlash::required_intents() | Intents::GUILDS;
    let my_id = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!")
        .id;

    let http = reqwest::Client::builder()
        .user_agent("randomairborne/experienced")
        .https_only(true)
        .build()
        .unwrap();

    let task_tracker = TaskTracker::new();

    let (config_tx, mut config_rx) = tokio::sync::mpsc::channel(10);
    let (rewards_tx, mut rewards_rx) = tokio::sync::mpsc::channel(10);

    let listener = XpdListener::new(db.clone(), client.clone(), task_tracker.clone(), my_id);

    let updating_listener = listener.clone();
    let config_update = tokio::spawn(async move {
        while let Some((guild, config)) = config_rx.recv().await {
            if let Err(source) = updating_listener.update_config(guild, config) {
                error!(?guild, ?source, "Unable to update config for guild");
            }
        }
    });

    let updating_listener = listener.clone();
    let rewards_update = tokio::spawn(async move {
        while let Some(InvalidateCache(guild)) = rewards_rx.recv().await {
            let updating_listener = updating_listener.clone();
            tokio::spawn(async move {
                if let Err(source) = updating_listener.invalidate_rewards(guild).await {
                    error!(?guild, ?source, "Unable to invalidate rewards for guild");
                }
            });
        }
    });

    let update_channels = UpdateChannels {
        config: config_tx,
        rewards: rewards_tx,
    };

    let slash = XpdSlash::new(
        http,
        client.clone(),
        my_id,
        db.clone(),
        task_tracker.clone(),
        control_guild,
        owners,
        update_channels,
    )
    .await;
    let config = Config::new(token.clone(), intents);
    let shards: Vec<Shard> =
        twilight_gateway::create_recommended(&client, config, |_, builder| builder.build())
            .await
            .expect("Failed to create recommended shard count")
            .collect();
    let senders: Vec<MessageSender> = shards.iter().map(Shard::sender).collect();
    info!("Connecting to discord");

    let shutdown = Arc::new(AtomicBool::new(false));
    for shard in shards {
        let client = client.clone();
        task_tracker.clone().spawn(event_loop(
            shard,
            client,
            task_tracker.clone(),
            shutdown.clone(),
            listener.clone(),
            slash.clone(),
            db.clone(),
        ));
    }

    vss::shutdown_signal().await;
    warn!("Shutting down..");
    debug!("Informing shards of shutdown");
    // Let the shards know not to reconnect
    shutdown.store(true, Ordering::Release);

    debug!("Informing discord of shutdown");
    // Tell the shards to shut down
    for sender in senders {
        sender.close(CloseFrame::NORMAL).ok();
    }

    debug!("Waiting for background tasks to complete");
    // Await all tasks to complete.
    task_tracker.close();
    task_tracker.wait().await;

    drop(slash); // Must be dropped before awaiting config shutdown, to allow the recv loop to end
    debug!("Waiting for listener updater to close");
    if let Err(source) = config_update.await {
        error!(?source, "Could not shut down config updater");
    }
    if let Err(source) = rewards_update.await {
        error!(?source, "Could not shut down config updater");
    }

    info!("Done, see ya!");
}

async fn event_loop(
    mut shard: Shard,
    http: Arc<DiscordClient>,
    task_tracker: TaskTracker,
    shutdown: Arc<AtomicBool>,
    listener: XpdListener,
    slash: XpdSlash,
    db: PgPool,
) {
    let event_flags = XpdListener::required_events()
        | XpdSlash::required_events()
        | EventTypeFlags::READY
        | EventTypeFlags::GUILD_CREATE;
    while let Some(next) = shard.next_event(event_flags).await {
        trace!(?next, "got new event");
        let event = match next {
            Ok(event) => event,
            Err(source) => {
                if shutdown.load(Ordering::Acquire)
                    && matches!(source.kind(), ReceiveMessageErrorType::WebSocket)
                {
                    break;
                }
                error!(?source, "error receiving event");
                continue;
            }
        };
        if matches!(event, Event::GatewayClose(_)) && shutdown.load(Ordering::Acquire) {
            break;
        }
        trace!(?event, "got event");
        let listener = listener.clone();
        let http = http.clone();
        let slash = slash.clone();
        let db = db.clone();
        task_tracker.spawn(async move {
            if let Err(error) = handle_event(event, http, listener, slash, db).await {
                // this includes even user caused errors. User beware. Don't set up automatic emails or anything.
                error!(?error, "Handler error");
            }
        });
    }
}

async fn handle_event(
    event: Event,
    http: Arc<DiscordClient>,
    listener: XpdListener,
    slash: XpdSlash,
    db: PgPool,
) -> Result<(), Error> {
    listener.update_cache(&event);
    match event {
        Event::Ready(ready) => {
            info!(
                shard_id = ?ready.shard.unwrap_or(ShardId::ONE),
                name = ready.user.name,
                id = ready.user.id.get(),
                "shard got ready",
            );
        }
        Event::MessageCreate(msg) => listener.save(*msg).await?,
        Event::GuildCreate(guild_add) => {
            if !sqlx::query!(
                "SELECT id FROM guild_bans WHERE
                ((expires > NOW()) OR (expires IS NULL))
                AND id = $1",
                id_to_db(guild_add.id)
            )
            .fetch_all(&db)
            .await?
            .is_empty()
            {
                debug!(
                    id = guild_add.id.get(),
                    "Leaving guild because it is banned"
                );
                http.leave_guild(guild_add.id).await?;
                return Ok(());
            }
        }
        Event::InteractionCreate(interaction_create) => slash.execute(*interaction_create).await,
        _ => {}
    };
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("listener-library error: {0}")]
    Listener(#[from] xpd_listener::Error),
    #[error("Twilight-Validate error: {0}")]
    Validate(#[from] twilight_validate::message::MessageValidationError),
    #[error("Twilight-Http error: {0}")]
    Api(#[from] twilight_http::Error),
    #[error("Twilight-Http deserialization error: {0}")]
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    #[error("Postgres error: {0}")]
    Postgres(#[from] sqlx::Error),
}
