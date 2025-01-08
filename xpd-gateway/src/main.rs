#![deny(clippy::all)]

#[macro_use]
extern crate tracing;

use std::{
    collections::HashMap,
    env::VarError,
    process::{ExitCode, Termination},
    str::FromStr,
    sync::Arc,
};

use base64::{
    engine::{GeneralPurpose as Base64Engine, GeneralPurposeConfig as Base64Config},
    Engine,
};
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{logs::LoggerProvider, Resource};
use sqlx::PgPool;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
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
    channel::message::AllowedMentions,
    gateway::ShardId,
    id::{marker::GuildMarker, Id},
};
use xpd_common::RequiredDiscordResources;
use xpd_listener::XpdListener;
use xpd_slash::XpdSlash;
use xpd_util::LogError;

#[tokio::main]
async fn main() -> Result<(), SetupError> {
    let tracer_shutdown = init_tracing()?;
    info!(
        version = xpd_common::CURRENT_GIT_SHA,
        "Starting experienced!"
    );

    let token = get_var("DISCORD_TOKEN")?;
    let pg = get_var("DATABASE_URL")?;
    let control_guild: Id<GuildMarker> = parse_var("CONTROL_GUILD")?;

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&pg)
        .await?;
    sqlx::migrate!("../migrations").run(&db).await?;

    let client = Arc::new(
        DiscordClient::builder()
            .default_allowed_mentions(AllowedMentions::default())
            .token(token.clone())
            .build(),
    );
    let intents = XpdListener::required_intents() | XpdSlash::required_intents() | Intents::GUILDS;

    let current_app = client.current_user_application().await?.model().await?;
    let app_id = current_app.id;
    let bot_id = current_app.bot.ok_or(SetupError::NoBot)?.id;
    let owners = if let Some(team) = current_app.team {
        team.members.iter().map(|v| v.user.id).collect()
    } else {
        vec![current_app.owner.ok_or(SetupError::NoTeamOrOwner)?.id]
    };

    info!(?owners, "Got list of owners");

    let http = reqwest::Client::builder()
        .user_agent("randomairborne/experienced")
        .https_only(true)
        .build()?;

    let cache_resource_types =
        XpdListener::required_cache_types() | XpdSlash::required_cache_types();
    let cache = Arc::new(
        InMemoryCacheBuilder::new()
            .resource_types(cache_resource_types)
            .build(),
    );

    let task_tracker = TaskTracker::new();

    let (event_bus_tx, mut event_bus_rx) = tokio::sync::mpsc::channel(10);

    let listener = XpdListener::new(
        db.clone(),
        client.clone(),
        cache.clone(),
        task_tracker.clone(),
        bot_id,
    );

    let shutdown = CancellationToken::new();

    let updating_listener = listener.clone();
    let updating_task_tracker = task_tracker.clone();
    let updating_shutdown = shutdown.clone();
    let config_update = tokio::spawn(async move {
        while let Some(event) = updating_shutdown
            .run_until_cancelled(event_bus_rx.recv())
            .await
            .flatten()
        {
            let listener = updating_listener.clone();
            updating_task_tracker.spawn(async move {
                listener.bus(event).await;
            });
        }
    });

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
        event_bus_tx,
    );
    let config = Config::new(token.clone(), intents);
    let shards: Vec<Shard> =
        twilight_gateway::create_recommended(&client, config, |_, builder| builder.build())
            .await?
            .collect();
    let senders: Vec<MessageSender> = shards.iter().map(Shard::sender).collect();
    info!("Connecting to discord");

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
    shutdown.cancel();

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
    config_update
        .await
        .log_error("Could not shut down config updater");

    if let Some(tracer) = tracer_shutdown {
        tracer.shutdown()?;
    }

    info!("Done, see ya!");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn event_loop(
    mut shard: Shard,
    http: Arc<DiscordClient>,
    task_tracker: TaskTracker,
    shutdown: CancellationToken,
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
                if shutdown.is_cancelled()
                    && matches!(source.kind(), ReceiveMessageErrorType::WebSocket)
                {
                    break;
                }
                error!(?source, "error receiving event");
                continue;
            }
        };
        if matches!(event, Event::GatewayClose(_)) && shutdown.is_cancelled() {
            break;
        }
        trace!(?event, "got event");
        let listener = listener.clone();
        let http = http.clone();
        let slash = slash.clone();
        let db = db.clone();
        let cache = cache.clone();
        task_tracker.spawn(async move {
            handle_event(event, http, listener, slash, cache, db)
                .await
                .log_error("Handler error");
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
            xpd_database::delete_guild_cleanup(&db, guild_add.id).await?;
        }
        Event::GuildDelete(del) => {
            xpd_database::add_guild_cleanup(&db, del.id).await?;
        }
        Event::InteractionCreate(interaction_create) => slash.execute(*interaction_create).await,
        Event::BanAdd(ban) => {
            // These values are "best effort", as we can only report the errors to the devs- but unless
            // we actually delete the levels, we don't want the audit logs deleted
            xpd_database::delete_levels_user_guild(&db, ban.user.id, ban.guild_id).await?;
            xpd_database::delete_audit_log_events_user_guild(&db, ban.user.id, ban.guild_id)
                .await?;
        }
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

fn init_tracing() -> Result<Option<LoggerProvider>, SetupError> {
    let logger = get_var_opt("OTLP_ENDPOINT")?
        .map(|v| make_otlp(&v))
        .transpose()?;

    let layer = logger
        .as_ref()
        .map(OpenTelemetryTracingBridge::new)
        .map(|v| v.with_filter(PrefixFilter));
    let fmt = tracing_subscriber::fmt::layer();

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    Registry::default().with(fmt).with(layer).init();
    Ok(logger)
}

fn make_otlp(endpoint: &str) -> Result<LoggerProvider, SetupError> {
    let svc_name = Resource::new(vec![KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        env!("CARGO_PKG_NAME"),
    )]);

    let headers = make_otlp_headers()?;

    let exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .with_headers(headers)
        .with_http_client(reqwest::Client::new())
        .build()?;

    // Create a new OpenTelemetry trace pipeline that prints to stdout
    Ok(LoggerProvider::builder()
        .with_resource(svc_name.clone())
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build())
}

fn make_otlp_headers() -> Result<HashMap<String, String>, SetupError> {
    let Some(username) = get_var_opt("OTLP_BASIC_USERNAME")? else {
        return Ok(HashMap::new());
    };

    let password = get_var_opt("OTLP_BASIC_PASSWORD")?.ok_or_else(|| {
        SetupError::ReliantEnv(
            "OTLP_BASIC_PASSWORD".to_owned(),
            "OTLP_BASIC_USERNAME".to_owned(),
        )
    })?;

    const B64_ENGINE: Base64Engine =
        Base64Engine::new(&base64::alphabet::URL_SAFE, Base64Config::new());

    let basic_string = B64_ENGINE.encode(format!("{username}:{password}"));
    let mut out_map = HashMap::new();
    out_map.insert("Authorization".to_string(), format!("Basic {basic_string}"));
    Ok(out_map)
}

fn get_var_opt(name: &str) -> Result<Option<String>, SetupError> {
    let value = std::env::var(name);
    match value {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(SetupError::UnparsableEnv(name.to_owned())),
    }
}

fn get_var(name: &str) -> Result<String, SetupError> {
    get_var_opt(name)?.ok_or_else(|| SetupError::MissingEnv(name.to_owned()))
}

fn parse_var<T>(name: &str) -> Result<T, SetupError>
where
    T: FromStr,
    T::Err: std::error::Error + 'static,
{
    get_var(name)?
        .parse()
        .map_err(|e| SetupError::FromStr(name.to_string(), Box::new(e)))
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

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("No bot returned from discord")]
    NoBot,
    #[error("No team members or bot owner returned from discord")]
    NoTeamOrOwner,
    #[error("Could not parse environment variable {0} as UTF-8")]
    UnparsableEnv(String),
    #[error("Environment variable {0} is required!")]
    MissingEnv(String),
    #[error("Environment variable {0} is required when {1} is set!")]
    ReliantEnv(String, String),
    #[error("Could not parse environment variable {0}: {1}")]
    FromStr(String, Box<dyn std::error::Error>),
    #[error("Failed to build logger: {0}")]
    Otel(#[from] opentelemetry_sdk::logs::LogError),
    #[error("Failed to build database client: {0}")]
    DatabaseConnect(#[from] sqlx::Error),
    #[error("Failed to run database migrations: {0}")]
    DatabaseMigrate(#[from] sqlx::migrate::MigrateError),
    #[error("Failed to build reqwest client: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to request to Discord API: {0}")]
    Twilight(#[from] twilight_http::Error),
    #[error("Failed to deserialize from Discord API: {0}")]
    TwilightBody(#[from] twilight_http::response::DeserializeBodyError),
    #[error("Failed to start shard connections to Discord API: {0}")]
    TwilightGateway(#[from] twilight_gateway::error::StartRecommendedError),
}

impl Termination for SetupError {
    fn report(self) -> std::process::ExitCode {
        eprintln!("{self}");
        ExitCode::FAILURE
    }
}
