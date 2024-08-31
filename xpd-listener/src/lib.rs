use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, RwLock},
};

use ahash::AHashMap;
use sqlx::{query, query_as, PgPool};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::{event::Event, Intents},
    id::{
        marker::{ApplicationMarker, GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};
use xpd_common::{db_to_id, id_to_db, GuildConfig, RawGuildConfig, RequiredEvents, RoleReward};

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
    pub fn new(db: PgPool, http: Arc<twilight_http::Client>, me: Id<ApplicationMarker>) -> Self {
        Self(Arc::new(XpdListenerInner::new(db, http, me)))
    }
}

impl Deref for XpdListener {
    type Target = XpdListenerInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl RequiredEvents for XpdListener {
    fn required_intents() -> Intents {
        XpdListenerInner::required_intents()
    }

    fn required_events() -> EventTypeFlags {
        XpdListenerInner::required_events()
    }
}

pub struct XpdListenerInner {
    db: PgPool,
    messages: RwLock<SentMessages>,
    http: Arc<twilight_http::Client>,
    cache: InMemoryCache,
    configs: LockingMap<Id<GuildMarker>, Arc<GuildConfig>>,
    rewards: LockingMap<Id<GuildMarker>, Arc<Vec<RoleReward>>>,
    current_application_id: Id<ApplicationMarker>,
}

impl XpdListenerInner {
    pub(crate) fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        current_application_id: Id<ApplicationMarker>,
    ) -> Self {
        let messages = RwLock::new(SentMessages::new());
        let configs = RwLock::new(HashMap::new());
        let rewards = RwLock::new(HashMap::new());
        let resource_types = ResourceType::USER_CURRENT
            | ResourceType::ROLE
            | ResourceType::GUILD
            | ResourceType::CHANNEL
            | ResourceType::MEMBER;

        let cache = InMemoryCache::builder()
            .resource_types(resource_types)
            .build();

        Self {
            db,
            messages,
            http,
            configs,
            rewards,
            cache,
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
        let config = query_as!(
            RawGuildConfig,
            "SELECT one_at_a_time, level_up_message, level_up_channel, ping_on_level_up \
             FROM guild_configs WHERE id = $1",
            id_to_db(guild)
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or_else(RawGuildConfig::default);
        let config: GuildConfig = config.try_into()?;
        let config = Arc::new(config);
        self.configs.write()?.insert(guild, config.clone());
        Ok(config)
    }

    pub async fn invalidate_rewards(&self, guild: Id<GuildMarker>) -> Result<(), Error> {
        let mut new_rewards = self.get_guild_rewards_uncached(guild).await?;
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
        let mut rewards = self.get_guild_rewards_uncached(guild_id).await?;
        rewards.sort_by(xpd_common::sort_rewards);

        let new_copy = Arc::new(rewards);
        self.rewards.write()?.insert(guild_id, new_copy.clone());
        Ok(new_copy)
    }

    async fn get_guild_rewards_uncached(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Vec<RoleReward>, Error> {
        let rewards: Vec<RoleReward> = query!(
            "SELECT id, requirement FROM role_rewards WHERE guild = $1",
            id_to_db(guild_id),
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|row| RoleReward {
            id: db_to_id(row.id),
            requirement: row.requirement,
        })
        .collect();
        Ok(rewards)
    }
}

impl RequiredEvents for XpdListenerInner {
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
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Discord error")]
    Twilight(#[from] twilight_http::Error),
    #[error("simpleinterpolation failed")]
    CouldNotInterpolate(#[from] simpleinterpolation::Error),
    #[error("Unknown permissions for role")]
    UnknownPermissionsForRole(#[from] twilight_cache_inmemory::permission::RootError),
    #[error("Unknown permissions for role")]
    UnknownPermissionsForMessage(#[from] twilight_cache_inmemory::permission::ChannelError),
    #[error("RwLock Poisioned, please report: https://valk.sh/discord")]
    LockPoisoned,
    #[error("Discord did not send a member where they MUST send a member")]
    NoMember,
    #[error("Unknown role: <@&{0}>")]
    UnknownRole(Id<RoleMarker>),
    #[error("Highest known role for self was not found in cache!")]
    NoHighestRoleForSelf,
    #[error("Target role was not found in cache!")]
    NoTargetRoleInCache,
    #[error("Got unknown role for own highest role!")]
    UnknownPositionForOwnHighestRole,
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned
    }
}
