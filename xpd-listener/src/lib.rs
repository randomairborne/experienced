use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use expiringmap::ExpiringSet;
use sqlx::{query_as, PgPool};
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use xpd_common::{id_to_db, GuildConfig};

mod message;

#[macro_use]
extern crate tracing;

type SentMessages = ExpiringSet<(Id<GuildMarker>, Id<UserMarker>)>;

#[derive(Clone)]
pub struct XpdListener {
    db: PgPool,
    messages: Arc<RwLock<SentMessages>>,
    http: Arc<twilight_http::Client>,
    configs: Arc<RwLock<HashMap<Id<GuildMarker>, GuildConfig>>>,
}

impl XpdListener {
    pub fn new(db: PgPool, http: Arc<twilight_http::Client>) -> Self {
        let messages = Arc::new(RwLock::new(SentMessages::new()));
        let configs = Arc::new(RwLock::new(HashMap::new()));
        Self {
            db,
            messages,
            http,
            configs,
        }
    }

    pub fn update_config(&self, guild: Id<GuildMarker>, config: GuildConfig) {
        if let Ok(mut lock) = self.configs.write() {
            lock.insert(guild, config);
        } else {
            error!("Unable to get guild config lock!!");
        }
    }

    pub async fn get_guild_config(&self, guild: Id<GuildMarker>) -> Result<GuildConfig, Error> {
        if let Some(cfg) = self.configs.read()?.get(&guild) {
            return Ok(cfg.clone());
        }
        let config = query_as!(
            GuildConfig,
            "SELECT one_at_a_time FROM guild_configs WHERE id = $1",
            id_to_db(guild)
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or_else(GuildConfig::default);
        self.configs.write()?.insert(guild, config.clone());
        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Discord error")]
    Twilight(#[from] twilight_http::Error),
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
