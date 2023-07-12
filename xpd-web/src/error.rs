use std::sync::{Arc, OnceLock};

use axum::{http::StatusCode, response::Html};
use tera::Tera;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQLx error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Templating error")]
    Tera(#[from] tera::Error),
    #[error("Redis error")]
    Redis(#[from] redis::RedisError),
    #[error("Redis pool error")]
    RedisPool(#[from] deadpool_redis::PoolError),
    #[error("JSON deserialization failed")]
    SerdeJson(#[from] serde_json::Error),
    #[error("This server does not have Experienced, or no users have leveled up.")]
    NoLeveling,
}

pub static ERROR_TERA: OnceLock<Arc<Tera>> = OnceLock::new();
const TERA_ERROR: &str = "
<!DOCTYPE html>
<html lang=\"en-US\">
<head>
    <meta charset=\"utf-8\" />
    <title>Error error, on the wall</title>
</head>
<body>
<p>
There was an error processing your request.<br>
In addition, there was an problem displaying what went wrong.<br>
This error has been logged. We are sorry for the inconvenience.
</p>
<body>";

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        eprintln!("{self:?}");
        let Some(tera) = ERROR_TERA.get() else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Tera not initialized, this should be impossible",
            )
                .into_response();
        };
        let mut context = tera::Context::new();
        context.insert("error", &self.to_string());
        let status = match self {
            Self::Sqlx(_)
            | Self::Tera(_)
            | Self::Redis(_)
            | Self::RedisPool(_)
            | Self::SerdeJson(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NoLeveling => StatusCode::NOT_FOUND,
        };
        let render = if status == StatusCode::NOT_FOUND {
            tera.render("404.html", &context)
        } else {
            tera.render("5xx.html", &context)
        };
        let html = match render {
            Ok(v) => v,
            Err(source) => {
                error!(?source, "Failed to template error response");
                return (StatusCode::INTERNAL_SERVER_ERROR, Html(TERA_ERROR)).into_response();
            }
        };
        (status, Html(html)).into_response()
    }
}
