use crate::AppState;
use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use tokio::task::JoinHandle;

use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    channel::message::MessageFlags,
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
    let interaction: Interaction = serde_json::from_slice(&body)?;
    let interaction_token = interaction.token.clone();
    if interaction.kind == InteractionType::Ping {
        return Ok(Json(InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        }));
    }
    let responder_handle = tokio::spawn(state.bot.clone().run(interaction));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    if responder_handle.is_finished() {
        match responder_handle.await {
            Ok(v) => {
                trace!("responding directly");
                return Ok(Json(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(v.into()),
                }));
            }
            Err(e) => {
                error!("Handler panicked with {e}");
                return Ok(Json(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(
                        XpdSlashResponse::new()
                            .content(format!("Handler panicked with {e}"))
                            .flags(MessageFlags::EPHEMERAL)
                            .into(),
                    ),
                }));
            }
        }
    }
    trace!("Spawning follow-up task");
    tokio::spawn(respond_to_discord_later(
        state.bot,
        interaction_token,
        responder_handle,
    ));
    trace!("responding with DCMWS");
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    };
    Ok(Json(response))
}

async fn respond_to_discord_later(
    state: XpdSlash,
    token: String,
    handle: JoinHandle<XpdSlashResponse>,
) {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let response = match handle.await {
        Ok(v) => v,
        Err(source) => {
            error!(?source, "Handler panicked");
            XpdSlashResponse::new().content(format!("Handler panicked: {source}"))
        }
    };
    if let Err(source) = state.send_followup(response, &token).await {
        error!(?source, "Followup validate failed");
    }
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
