use std::{ops::Deref, sync::Arc};

use dashmap::DashMap;
use tokio_util::task::TaskTracker;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::Intents,
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};
use xpd_common::{EventBusMessage, GuildConfig, RequiredDiscordResources, RoleReward};
use xpd_database::PgPool;

mod audit_log;
mod message;

pub use audit_log::audit_log;

#[macro_use]
extern crate tracing;

#[derive(Clone)]
pub struct XpdListener(Arc<XpdListenerInner>);

impl XpdListener {
    pub fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        cache: Arc<InMemoryCache>,
        tasks: TaskTracker,
        me: Id<UserMarker>,
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
    http: Arc<twilight_http::Client>,
    cache: Arc<InMemoryCache>,
    #[allow(unused)]
    task_tracker: TaskTracker,
    configs: DashMap<Id<GuildMarker>, Arc<GuildConfig>>,
    rewards: DashMap<Id<GuildMarker>, Arc<[RoleReward]>>,
    bot_id: Id<UserMarker>,
}

impl XpdListenerInner {
    pub(crate) fn new(
        db: PgPool,
        http: Arc<twilight_http::Client>,
        cache: Arc<InMemoryCache>,
        task_tracker: TaskTracker,
        bot_id: Id<UserMarker>,
    ) -> Self {
        let configs = DashMap::new();
        let rewards = DashMap::new();

        Self {
            db,
            http,
            configs,
            rewards,
            cache,
            task_tracker,
            bot_id,
        }
    }

    pub async fn bus(&self, msg: EventBusMessage) {
        let res = match msg {
            EventBusMessage::InvalidateRewards(id) => self.invalidate_rewards(id).await,
            EventBusMessage::UpdateConfig(id, guild_config) => self.update_config(id, guild_config),
        };
        match res {
            Ok(()) => {}
            Err(err) => error!(message = %err, ?err, "Error processing on event bus"),
        }
    }

    pub fn update_config(&self, guild: Id<GuildMarker>, config: GuildConfig) -> Result<(), Error> {
        self.configs.insert(guild, Arc::new(config));
        Ok(())
    }

    pub async fn get_guild_config(
        &self,
        guild: Id<GuildMarker>,
    ) -> Result<Arc<GuildConfig>, Error> {
        if let Some(guild_config) = self.configs.get(&guild) {
            return Ok(Arc::clone(&guild_config));
        }
        let config = xpd_database::guild_config(&self.db, guild)
            .await?
            .unwrap_or_default();
        let config = Arc::new(config);
        self.configs.insert(guild, config.clone());
        Ok(config)
    }

    pub async fn invalidate_rewards(&self, guild: Id<GuildMarker>) -> Result<(), Error> {
        let mut new_rewards = xpd_database::guild_rewards(&self.db, guild).await?;
        new_rewards.sort_by(xpd_common::compare_rewards_requirement);
        self.rewards.insert(guild, new_rewards.into());
        Ok(())
    }

    pub async fn get_guild_rewards(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Arc<[RoleReward]>, Error> {
        if let Some(rewards) = self.rewards.get(&guild_id) {
            return Ok(rewards.clone());
        }
        let mut rewards = xpd_database::guild_rewards(&self.db, guild_id).await?;
        rewards.sort_by(xpd_common::compare_rewards_requirement);

        let new_copy: Arc<[RoleReward]> = rewards.into();
        self.rewards.insert(guild_id, new_copy.clone());
        Ok(new_copy)
    }
}

impl RequiredDiscordResources for XpdListenerInner {
    fn required_intents() -> Intents {
        Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::GUILD_MODERATION
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
            | EventTypeFlags::GUILD_AUDIT_LOG_ENTRY_CREATE
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
    CouldNotInterpolate(#[from] simpleinterpolation::ParseError),
    #[error("Unknown permissions for role")]
    UnknownPermissionsForRole(#[from] twilight_cache_inmemory::permission::RootError),
    #[error("Unknown permissions for role")]
    UnknownPermissionsForMessage(#[from] twilight_cache_inmemory::permission::ChannelError),
    #[error("Failed to check permissions: {0}")]
    PermissionsCalculator(#[from] xpd_util::PermissionCheckError),
    #[error("{0}")]
    AuditLogError(#[from] audit_log::AuditLogError),
    #[error("Discord did not send a member where they MUST send a member")]
    NoMember,
}
