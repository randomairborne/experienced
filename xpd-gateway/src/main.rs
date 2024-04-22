#![deny(clippy::all)]

#[macro_use]
extern crate tracing;

use std::sync::Arc;

use ahash::AHashSet;
use parking_lot::Mutex;
use sqlx::PgPool;
use tokio::{sync::watch::Receiver, task::JoinSet, time::MissedTickBehavior};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_gateway::{CloseFrame, Config, Event, Intents, MessageSender, Shard};
use twilight_http::Client as DiscordClient;
use twilight_model::{
    gateway::ShardId,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use xpd_common::id_to_db;
use xpd_listener::XpdListener;
use xpd_slash::XpdSlash;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let token = xpd_common::get_var("DISCORD_TOKEN");
    let redis_url = xpd_common::get_var("REDIS_URL");
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
    let root_url = xpd_common::get_var("ROOT_URL")
        .trim_end_matches('/')
        .to_string();
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
    redis.get().await.expect("Failed to connect to redis");
    let client = twilight_http::Client::new(token.clone());
    let intents = Intents::GUILD_MESSAGES | Intents::GUILD_MEMBERS | Intents::GUILDS;
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
    let senders: Vec<MessageSender> = shards.iter().map(Shard::sender).collect();
    let client = Arc::new(twilight_http::Client::new(token));
    info!("Connecting to discord");
    let http = reqwest::Client::new();
    let listener = XpdListener::new(db.clone(), redis.clone(), client.clone());

    let slash = XpdSlash::new(
        http,
        client.clone(),
        my_id,
        db.clone(),
        redis,
        root_url,
        control_guild,
        owners,
    )
    .await;

    let (shutdown_trigger, should_shutdown) = tokio::sync::watch::channel::<()>(());
    let mut set = JoinSet::new();
    for shard in shards {
        let guilds: GuildList = Arc::new(Mutex::new(AHashSet::with_capacity(3000)));
        set.spawn(cache_refresh_loop(
            shard.sender(),
            guilds.clone(),
            should_shutdown.clone(),
        ));
        let client = client.clone();
        set.spawn(event_loop(
            shard,
            client,
            guilds,
            should_shutdown.clone(),
            listener.clone(),
            slash.clone(),
            db.clone(),
        ));
    }

    vss::shutdown_signal().await;
    warn!("Shutting down..");

    // Let the shards know not to reconnect
    shutdown_trigger.send(()).unwrap();

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
    http: Arc<DiscordClient>,
    guilds: GuildList,
    mut should_shutdown: Receiver<()>,
    listener: XpdListener,
    slash: XpdSlash,
    db: PgPool,
) {
    loop {
        let next_event = tokio::select! {
            event = shard.next_event() => event,
            _ = should_shutdown.changed() => break,
        };
        trace!(?next_event, "event");
        match next_event {
            Ok(event) => {
                let listener = listener.clone();
                let http = http.clone();
                let slash = slash.clone();
                let sender = shard.sender();
                let db = db.clone();
                let guilds = guilds.clone();
                tokio::spawn(async move {
                    if let Err(error) =
                        handle_event(event, http, guilds, listener, slash, sender, db).await
                    {
                        // this includes even user caused errors. User beware. Don't set up automatic emails or anything.
                        error!(?error, "Handler error");
                    }
                });
            }
            Err(error) => error!(?error, "Shard loop error"),
        }
    }
}

async fn handle_event(
    event: Event,
    http: Arc<DiscordClient>,
    guilds: GuildList,
    listener: XpdListener,
    slash: XpdSlash,
    shard: MessageSender,
    db: PgPool,
) -> Result<(), Error> {
    match event {
        Event::Ready(ready) => {
            info!(
                shard_id = ?ready.shard.unwrap_or(ShardId::ONE),
                name = ready.user.name,
                id = ready.user.id.get(),
                "shard got ready",
            );
            let mut guilds = guilds.lock();
            for guild in ready.guilds {
                guilds.insert(guild.id);
            }
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
            guilds.lock().insert(guild_add.id);
            shard.command(
                &twilight_model::gateway::payload::outgoing::RequestGuildMembers::builder(
                    guild_add.id,
                )
                .query("", None),
            )?;
            listener.set_guild(guild_add.0).await?;
        }
        Event::GuildDelete(guild_delete) => {
            guilds.lock().remove(&guild_delete.id);
        }
        Event::MemberAdd(member_add) => listener.set_user(member_add.member.user).await?,
        Event::MemberUpdate(member_update) => listener.set_user(member_update.user).await?,
        Event::MemberChunk(member_chunk) => listener.set_chunk(member_chunk.members).await?,
        Event::InteractionCreate(interaction_create) => slash.execute(*interaction_create).await,
        _ => {}
    };
    Ok(())
}

type GuildList = Arc<Mutex<AHashSet<Id<GuildMarker>>>>;

async fn cache_refresh_loop(
    shard: MessageSender,
    guilds: GuildList,
    mut should_shutdown: Receiver<()>,
) {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(3600));
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = should_shutdown.changed() => break,
            _ = timer.tick() => {}
        }
        let refresh_result = tokio::select! {
            result = refresh_cache(
                shard.clone(),
                guilds.lock().clone()
            ) => result,
            _ = should_shutdown.changed() => break,
        };
        if let Err(source) = refresh_result {
            error!(?source, "Failed to update cache");
        }
    }
}

async fn refresh_cache(
    shard: MessageSender,
    guilds: AHashSet<Id<GuildMarker>>,
) -> Result<(), Error> {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(1));
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    for guild in &guilds {
        debug!(guild_id = guild.get(), "Requesting users for guild");
        shard.command(
            &twilight_model::gateway::payload::outgoing::RequestGuildMembers::builder(*guild)
                .query("", None),
        )?;
        timer.tick().await;
    }
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
    #[error("Twilight-Http error: {0}")]
    Api(#[from] twilight_http::Error),
    #[error("Twilight-Http deserialization error: {0}")]
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    #[error("Postgres error: {0}")]
    Postgres(#[from] sqlx::Error),
}
