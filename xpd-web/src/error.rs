use crate::AppState;
use axum::{http::StatusCode, response::Html};

#[allow(clippy::module_name_repetitions)]
pub struct HttpError {
    inner: Error,
    state: AppState,
}

impl HttpError {
    pub const fn new(inner: Error, state: AppState) -> Self {
        Self { inner, state }
    }
    pub const fn status(&self) -> StatusCode {
        match self.inner {
            Error::Sqlx(_)
            | Error::Tera(_)
            | Error::Redis(_)
            | Error::RedisPool(_)
            | Error::SerdeJson(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NoLeveling => StatusCode::NOT_FOUND,
        }
    }
}

impl std::fmt::Debug for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

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

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            error!(?self.inner, "Internal server error");
        } else {
            trace!(?self.inner, "Application error");
        }
        let mut context = tera::Context::new();
        context.insert("error", &self.inner.to_string());
        context.insert("root_url", &self.state.root_url);
        let render = if status == StatusCode::NOT_FOUND {
            self.state.tera.render("404.html", &context)
        } else {
            self.state.tera.render("5xx.html", &context)
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
