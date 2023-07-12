#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Templating error: {0}")]
    Tera(#[from] tera::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Redis pool error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),
    #[error("Non-integer value where integer expected: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("JSON deserialization failed: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("This server does not have Experienced, or no users have leveled up.")]
    NoLeveling,
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        eprintln!("{self:?}");
        self.to_string().into_response()
    }
}
