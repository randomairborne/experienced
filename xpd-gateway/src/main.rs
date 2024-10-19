#![deny(clippy::all)]

#[macro_use]
extern crate tracing;

use std::{
    collections::HashMap,
    env::VarError,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use base64::{
    engine::{GeneralPurpose as Base64Engine, GeneralPurposeConfig as Base64Config},
    Engine,
};
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{logs::LoggerProvider, Resource};
use sqlx::PgPool;
use tokio_util::task::TaskTracker;
use tracing::{error, Level, Metadata};
use tracing_subscriber::{
    layer::{Context, Filter, SubscriberExt},
    util::SubscriberInitExt,
    Layer, Registry,
};
use twilight_cache_inmemory::{InMemoryCache, InMemoryCacheBuilder};
use twilight_gateway::{
    error::ReceiveMessageErrorType, CloseFrame, Config, Event, EventTypeFlags, Intents,
    MessageSender, Shard, StreamExt,
};
use twilight_http::Client as DiscordClient;
use twilight_model::{
    gateway::ShardId,
    id::{marker::GuildMarker, Id},
};
use xpd_common::RequiredDiscordResources;
use xpd_listener::XpdListener;
use xpd_slash::{InvalidateCache, UpdateChannels, XpdSlash};

#[tokio::main]
async fn main() {
    let tracer_shutdown = init_tracing();

    let token = xpd_common::get_var("DISCORD_TOKEN");
    let pg = xpd_common::get_var("DATABASE_URL");
    let control_guild: Id<GuildMarker> = xpd_common::parse_var("CONTROL_GUILD");
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

    let current_app = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!");
    let app_id = current_app.id;
    let bot_id = current_app.bot.expect("There has to be a bot here").id;
    let owners = if let Some(owner) = current_app.owner {
        vec![owner.id]
    } else {
        current_app
            .team
            .expect("No team or owner for app")
            .members
            .iter()
            .map(|v| v.user.id)
            .collect()
    };

    let http = reqwest::Client::builder()
        .user_agent("randomairborne/experienced")
        .https_only(true)
        .build()
        .unwrap();

    let cache_resource_types =
        XpdListener::required_cache_types() | XpdSlash::required_cache_types();
    let cache = Arc::new(
        InMemoryCacheBuilder::new()
            .resource_types(cache_resource_types)
            .build(),
    );

    let task_tracker = TaskTracker::new();

    let (config_tx, mut config_rx) = tokio::sync::mpsc::channel(10);
    let (rewards_tx, mut rewards_rx) = tokio::sync::mpsc::channel(10);

    let listener = XpdListener::new(
        db.clone(),
        client.clone(),
        cache.clone(),
        task_tracker.clone(),
        bot_id,
    );

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
        app_id,
        bot_id,
        db.clone(),
        cache.clone(),
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
            cache.clone(),
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

    if let Some(tracer) = tracer_shutdown {
        tracer.shutdown().expect("Failed to shut down tracer");
    }

    info!("Done, see ya!");
}

#[allow(clippy::too_many_arguments)]
async fn event_loop(
    mut shard: Shard,
    http: Arc<DiscordClient>,
    task_tracker: TaskTracker,
    shutdown: Arc<AtomicBool>,
    listener: XpdListener,
    slash: XpdSlash,
    cache: Arc<InMemoryCache>,
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
        let cache = cache.clone();
        task_tracker.spawn(async move {
            if let Err(error) = handle_event(event, http, listener, slash, cache, db).await {
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
    cache: Arc<InMemoryCache>,
    db: PgPool,
) -> Result<(), Error> {
    cache.update(&event);
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
            if xpd_database::is_guild_banned(&db, guild_add.id).await? {
                debug!(
                    id = guild_add.id.get(),
                    "Leaving guild because it is banned"
                );
                http.leave_guild(guild_add.id).await?;
                return Ok(());
            }
        }
        Event::GuildDelete(_del) => {}
        Event::InteractionCreate(interaction_create) => slash.execute(*interaction_create).await,
        _ => {}
    };
    Ok(())
}

struct PrefixFilter;

impl<S> Filter<S> for PrefixFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        *meta.level() >= Level::INFO || meta.module_path().is_some_and(|mp| mp.starts_with("xpd"))
    }
}

#[must_use]
fn init_tracing() -> Option<LoggerProvider> {
    let logger = std::env::var("OTLP_ENDPOINT").ok().map(|v| make_otlp(&v));

    let layer = logger
        .as_ref()
        .map(OpenTelemetryTracingBridge::new)
        .map(|v| v.with_filter(PrefixFilter));
    let fmt = tracing_subscriber::fmt::layer();

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    Registry::default().with(fmt).with(layer).init();
    logger
}

#[must_use]
fn make_otlp(endpoint: &str) -> LoggerProvider {
    let svc_name = Resource::new(vec![KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        env!("CARGO_PKG_NAME"),
    )]);

    let headers = make_otlp_headers();

    // Create a new OpenTelemetry trace pipeline that prints to stdout
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_resource(svc_name.clone())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(endpoint)
                .with_headers(headers)
                .with_http_client(reqwest::Client::new()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap()
}

fn make_otlp_headers() -> HashMap<String, String> {
    let username = std::env::var("OTLP_BASIC_USERNAME");
    let username = match username {
        Ok(name) => name,
        Err(VarError::NotPresent) => return HashMap::new(),
        Err(VarError::NotUnicode(_)) => panic!("Failed to parse OTLP_BASIC_USERNAME"),
    };
    let password = std::env::var("OTLP_BASIC_PASSWORD")
        .expect("OTLP_BASIC_USERNAME was set, but OTLP_BASIC_PASSWORD was not!");

    const B64_ENGINE: Base64Engine =
        Base64Engine::new(&base64::alphabet::URL_SAFE, Base64Config::new());

    let basic_string = B64_ENGINE.encode(format!("{username}:{password}"));
    let mut out_map = HashMap::new();
    out_map.insert("Authorization".to_string(), format!("Basic {basic_string}"));
    out_map
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
    Postgres(#[from] xpd_database::Error),
}
