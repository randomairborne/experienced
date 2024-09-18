use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, RwLock},
};

use ahash::AHashMap;
use tokio_util::task::TaskTracker;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::{event::Event, Intents},
    id::{
        marker::{ApplicationMarker, GuildMarker, UserMarker},
        Id,
    },
};
use xpd_common::{GuildConfig, RequiredDiscordResources, RoleReward};
use xpd_database::PgPool;

mod message;

#[macro_use]
extern crate tracing;

// TODO: Maybe we can improve the locking on this. Have one task per guild or something.
// Go philosophy is good, we want to share memory by communicating.
// We use i64 here, because it makes some database stuff easier and
// it's impossible for a discord snowflake timestamp to exceed i64
type SentMessages = AHashMap<(Id<GuildMarker>, Id<UserMarker>), i64>;
type LockingMap<K, V> = RwLock<HashMap<K, V>>;

#[derive(Clone)]
pub struct XpdListener(Arc<XpdListenerInner>);

impl XpdListener {
    pub fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        cache: Arc<InMemoryCache>,
        tasks: TaskTracker,
        me: Id<ApplicationMarker>,
    ) -> Self {
        Self(Arc::new(XpdListenerInner::new(db, http, cache, tasks, me)))
    }
}

impl Deref for XpdListener {
    type Target = XpdListenerInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl RequiredDiscordResources for XpdListener {
    fn required_intents() -> Intents {
        XpdListenerInner::required_intents()
    }

    fn required_events() -> EventTypeFlags {
        XpdListenerInner::required_events()
    }

    fn required_cache_types() -> ResourceType {
        XpdListenerInner::required_cache_types()
    }
}

pub struct XpdListenerInner {
    db: PgPool,
    messages: RwLock<SentMessages>,
    http: Arc<twilight_http::Client>,
    cache: Arc<InMemoryCache>,
    #[allow(unused)]
    task_tracker: TaskTracker,
    configs: LockingMap<Id<GuildMarker>, Arc<GuildConfig>>,
    rewards: LockingMap<Id<GuildMarker>, Arc<Vec<RoleReward>>>,
    current_application_id: Id<ApplicationMarker>,
}

impl XpdListenerInner {
    pub(crate) fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        cache: Arc<InMemoryCache>,
        task_tracker: TaskTracker,
        current_application_id: Id<ApplicationMarker>,
    ) -> Self {
        let messages = RwLock::new(SentMessages::new());
        let configs = RwLock::new(HashMap::new());
        let rewards = RwLock::new(HashMap::new());

        Self {
            db,
            messages,
            http,
            configs,
            rewards,
            cache,
            task_tracker,
            current_application_id,
        }
    }

    pub fn update_cache(&self, uc: &Event) {
        self.cache.update(uc);
    }

    pub fn update_config(&self, guild: Id<GuildMarker>, config: GuildConfig) -> Result<(), Error> {
        self.configs.write()?.insert(guild, Arc::new(config));
        Ok(())
    }

    pub async fn get_guild_config(
        &self,
        guild: Id<GuildMarker>,
    ) -> Result<Arc<GuildConfig>, Error> {
        if let Some(guild_config) = self.configs.read()?.get(&guild) {
            return Ok(guild_config.clone());
        }
        let config = xpd_database::guild_config(&self.db, guild)
            .await?
            .unwrap_or_default();
        let config = Arc::new(config);
        self.configs.write()?.insert(guild, config.clone());
        Ok(config)
    }

    pub async fn invalidate_rewards(&self, guild: Id<GuildMarker>) -> Result<(), Error> {
        let mut new_rewards = xpd_database::guild_rewards(&self.db, guild).await?;
        new_rewards.sort_by(xpd_common::sort_rewards);
        self.rewards.write()?.insert(guild, Arc::new(new_rewards));
        Ok(())
    }

    pub async fn get_guild_rewards(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Arc<Vec<RoleReward>>, Error> {
        if let Some(rewards) = self.rewards.read()?.get(&guild_id) {
            return Ok(rewards.clone());
        }
        let mut rewards = xpd_database::guild_rewards(&self.db, guild_id).await?;
        rewards.sort_by(xpd_common::sort_rewards);

        let new_copy = Arc::new(rewards);
        self.rewards.write()?.insert(guild_id, new_copy.clone());
        Ok(new_copy)
    }
}

impl RequiredDiscordResources for XpdListenerInner {
    fn required_intents() -> Intents {
        Intents::GUILDS | Intents::GUILD_MESSAGES
    }

    fn required_events() -> EventTypeFlags {
        EventTypeFlags::GUILD_CREATE
            | EventTypeFlags::GUILD_UPDATE
            | EventTypeFlags::GUILD_DELETE
            | EventTypeFlags::CHANNEL_CREATE
            | EventTypeFlags::CHANNEL_UPDATE
            | EventTypeFlags::CHANNEL_DELETE
            | EventTypeFlags::ROLE_DELETE
            | EventTypeFlags::ROLE_UPDATE
            | EventTypeFlags::ROLE_CREATE
            | EventTypeFlags::THREAD_CREATE
            | EventTypeFlags::THREAD_UPDATE
            | EventTypeFlags::THREAD_LIST_SYNC
            | EventTypeFlags::THREAD_DELETE
            | EventTypeFlags::MESSAGE_CREATE
    }

    fn required_cache_types() -> ResourceType {
        ResourceType::USER_CURRENT
            | ResourceType::ROLE
            | ResourceType::GUILD
            | ResourceType::CHANNEL
            | ResourceType::MEMBER
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Discord error")]
    Twilight(#[from] twilight_http::Error),
    #[error("database fetch fail: {0}")]
    DatabaseAbstraction(#[from] xpd_database::Error),
    #[error("simpleinterpolation failed")]
    CouldNotInterpolate(#[from] simpleinterpolation::Error),
    #[error("Unknown permissions for role")]
    UnknownPermissionsForRole(#[from] twilight_cache_inmemory::permission::RootError),
    #[error("Unknown permissions for role")]
    UnknownPermissionsForMessage(#[from] twilight_cache_inmemory::permission::ChannelError),
    #[error("Failed to check permissions: {0}")]
    PermissionsCalculator(#[from] xpd_util::PermissionCheckError),
    #[error("RwLock Poisioned, please report: https://valk.sh/discord")]
    LockPoisoned,
    #[error("Discord did not send a member where they MUST send a member")]
    NoMember,
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned
    }
}
