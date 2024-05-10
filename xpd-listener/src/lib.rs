use std::sync::{Arc, RwLock};

use expiringmap::ExpiringSet;
use sqlx::PgPool;
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

mod message;

type SentMessages = ExpiringSet<(Id<GuildMarker>, Id<UserMarker>)>;

#[derive(Clone)]
pub struct XpdListener {
    db: PgPool,
    messages: Arc<RwLock<SentMessages>>,
    http: Arc<twilight_http::Client>,
}

impl XpdListener {
    pub fn new(db: PgPool, http: Arc<twilight_http::Client>) -> Self {
        let messages = Arc::new(RwLock::new(SentMessages::new()));
        Self { db, messages, http }
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
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned
    }
}
