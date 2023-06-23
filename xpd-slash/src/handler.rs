use crate::AppState;
use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use xpd_slash::{XpdSlash, XpdSlashResponse};

#[allow(clippy::unused_async)]
pub async fn handle(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<InteractionResponse>, Error> {
    let body = body.to_vec();
    crate::discord_sig_validation::validate_discord_sig(&headers, &body, &state.pubkey)?;
    trace!("deserializing interaction");
    let interaction: Interaction = serde_json::from_slice(&body)?;
    let token = interaction.token.clone();
    if interaction.kind == InteractionType::Ping {
        return Ok(Json(InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        }));
    }
    trace!("beginning response");
    let mut responder_handle = tokio::spawn(state.bot.clone().run(interaction));
    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {},
        v = &mut responder_handle => return Ok(Json(
            build_response_now(state.bot, v.unwrap_or_else(handler_panicked), token).await
        )),
    }
    trace!("Spawning long-running follow-up task");
    tokio::spawn(async move {
        let response = responder_handle.await.unwrap_or_else(handler_panicked);
        if let Err(source) = state.bot.send_followup(response, &token).await {
            error!(?source, "Followup validate failed");
        };
    });
    trace!("responding with slowDCMWS");
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    };
    Ok(Json(response))
}

async fn build_response_now(
    state: XpdSlash,
    response: XpdSlashResponse,
    token: String,
) -> InteractionResponse {
    trace!("responding directly");
    if response.attachments.is_none() || response.attachments.as_ref().is_some_and(Vec::is_empty) {
        return InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(response.into()),
        };
    }
    trace!("Spawning short-running follow-up task");
    tokio::spawn(async move {
        if let Err(source) = state.send_followup(response, &token).await {
            error!(?source, "Followup validate failed");
        }
    });
    trace!("responding with quickDCMWS");
    InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    }
}

/// This method just takes a source, outputs an error, and returns that error in a response
fn handler_panicked(source: impl std::error::Error) -> XpdSlashResponse {
    error!(?source, "Handler panicked");
    XpdSlashResponse::new().content(format!("Handler panicked with {source}"))
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Signature validation error: {0}")]
    Validation(#[from] crate::discord_sig_validation::SignatureValidationError),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("twilight_validate error: {0}")]
    TwilightValidateMessage(#[from] twilight_validate::message::MessageValidationError),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("{self}");
        axum::response::Response::builder()
            .body(axum::body::boxed(axum::body::Full::from(self.to_string())))
            .unwrap()
    }
}
