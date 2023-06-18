use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use twilight_model::{
    application::interaction::Interaction,
    http::interaction::{InteractionResponse, InteractionResponseType},
};

use crate::AppState;

#[allow(clippy::unused_async)]
pub async fn handle(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<InteractionResponse>, Error> {
    let body = body.to_vec();
    crate::discord_sig_validation::validate_discord_sig(&headers, &body, &state.pubkey)?;
    let interaction: Interaction = serde_json::from_slice(&body)?;
    tokio::spawn(state.bot.run(interaction));
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    };
    Ok(Json(response))
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Signature validation error: {0}")]
    Validation(#[from] crate::discord_sig_validation::SignatureValidationError),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("{self}");
        axum::response::Response::builder()
            .body(axum::body::boxed(axum::body::Full::from(self.to_string())))
            .unwrap()
    }
}
