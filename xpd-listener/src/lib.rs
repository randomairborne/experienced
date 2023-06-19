use std::sync::Arc;

use sqlx::PgPool;

mod message;
mod user_cache;

#[derive(Clone)]
pub struct XpdListener {
    db: PgPool,
    redis: deadpool_redis::Pool,
    http: Arc<twilight_http::Client>,
}

impl XpdListener {
    pub fn new(db: PgPool, redis: deadpool_redis::Pool, http: Arc<twilight_http::Client>) -> Self {
        Self { db, redis, http }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_redis::PoolError),
    #[error("Discord error: {0}")]
    Twilight(#[from] twilight_http::Error),
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}
