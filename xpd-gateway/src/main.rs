#![deny(clippy::all)]

#[macro_use]
extern crate tracing;

use std::{
    env::VarError,
    process::{ExitCode, Termination},
    str::FromStr,
    sync::Arc,
};

use sqlx::PgPool;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::error;
use tracing_subscriber::EnvFilter;
use twilight_cache_inmemory::{InMemoryCache, InMemoryCacheBuilder};
use twilight_gateway::{
    CloseFrame, Config, Event, EventTypeFlags, Intents, MessageSender, Shard, StreamExt,
    error::ReceiveMessageError,
};
use twilight_http::Client as DiscordClient;
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::ShardId,
    id::{Id, marker::GuildMarker},
};
use xpd_common::RequiredDiscordResources;
use xpd_listener::XpdListener;
use xpd_slash::XpdSlash;
use xpd_util::LogError;

#[tokio::main]
async fn main() -> Result<(), SetupError> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
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
    let intents = XpdListener::required_intents()
        | XpdSlash::required_intents()
        | Intents::GUILD_MEMBERS
        | Intents::GUILDS;

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
            listener.clone(),
            slash.clone(),
            cache.clone(),
            db.clone(),
            shutdown.clone(),
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
        // We send and detect a specific frame to know if we should
        // shut down permanently
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

    info!("Done, see ya!");
    Ok(())
}

async fn next_event(
    shard: &mut Shard,
    flags: EventTypeFlags,
    shutdown: &CancellationToken,
) -> Option<Result<Event, ReceiveMessageError>> {
    tokio::select! {
        biased;
        _ = shutdown.cancelled() => None,
        v = shard.next_event(flags) => v,
    }
}

#[allow(clippy::too_many_arguments)]
async fn event_loop(
    mut shard: Shard,
    http: Arc<DiscordClient>,
    task_tracker: TaskTracker,
    listener: XpdListener,
    slash: XpdSlash,
    cache: Arc<InMemoryCache>,
    db: PgPool,
    shutdown: CancellationToken,
) {
    let event_flags = XpdListener::required_events()
        | XpdSlash::required_events()
        | EventTypeFlags::READY
        | EventTypeFlags::MEMBER_UPDATE
        | EventTypeFlags::MEMBER_REMOVE
        | EventTypeFlags::GUILD_DELETE
        | EventTypeFlags::GUILD_CREATE;
    while let Some(next) = next_event(&mut shard, event_flags, &shutdown).await {
        trace!(?next, "got new event");
        let event = match next {
            Ok(event) => event,
            Err(source) => {
                error!(?source, "error receiving event");
                continue;
            }
        };
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
            if xpd_database::is_guild_banned(&db, guild_add.id()).await? {
                debug!(
                    id = guild_add.id().get(),
                    "Leaving guild because it is banned"
                );
                http.leave_guild(guild_add.id()).await?;
                return Ok(());
            }
            xpd_database::delete_guild_cleanup(&db, guild_add.id()).await?;
        }
        Event::GuildDelete(del) => {
            xpd_database::add_guild_cleanup(&db, del.id).await?;
        }
        Event::MemberRemove(mr) => {
            xpd_database::add_user_guild_cleanup(&db, mr.guild_id, mr.user.id).await?;
        }
        Event::MemberAdd(ma) => {
            xpd_database::delete_user_guild_cleanup(&db, ma.guild_id, ma.user.id).await?;
        }
        Event::GuildAuditLogEntryCreate(gae) => xpd_listener::audit_log(&db, *gae).await?,
        Event::InteractionCreate(interaction_create) => slash.execute(*interaction_create).await,
        _ => {}
    };
    Ok(())
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
    #[error("No bot returned from discord- Probably ratelimited")]
    NoBot,
    #[error("No team members or bot owner returned from discord")]
    NoTeamOrOwner,
    #[error("Could not parse environment variable {0} as UTF-8")]
    UnparsableEnv(String),
    #[error("Environment variable {0} is required!")]
    MissingEnv(String),
    #[error("Environment variable {0} is required when the variable {1} is set!")]
    ReliantEnv(String, String),
    #[error("Could not parse environment variable {0}: {1}")]
    FromStr(String, Box<dyn std::error::Error>),
    #[error("Failed to build database client: {0}")]
    DatabaseConnect(#[from] sqlx::Error),
    #[error("Failed to run database migrations: {0}")]
    DatabaseMigrate(#[from] sqlx::migrate::MigrateError),
    #[error("Failed to build reqwest client: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to request to Discord API: {0}")]
    Twilight(Box<twilight_http::Error>),
    #[error("Failed to deserialize from Discord API: {0}")]
    TwilightBody(#[from] twilight_http::response::DeserializeBodyError),
    #[error("Failed to start shard connections to Discord API: {0}")]
    TwilightGateway(#[from] twilight_gateway::error::StartRecommendedError),
}

impl From<twilight_http::Error> for SetupError {
    fn from(value: twilight_http::Error) -> Self {
        Self::Twilight(Box::new(value))
    }
}

impl Termination for SetupError {
    fn report(self) -> std::process::ExitCode {
        std::thread::sleep(std::time::Duration::from_secs(2));
        eprintln!("{self}");
        ExitCode::FAILURE
    }
}
