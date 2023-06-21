use crate::AppState;
use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use tokio::task::JoinHandle;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use xpd_slash::XpdSlash;

#[allow(clippy::unused_async)]
pub async fn handle(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<InteractionResponse>, Error> {
    let body = body.to_vec();
    crate::discord_sig_validation::validate_discord_sig(&headers, &body, &state.pubkey)?;
    let interaction: Interaction = serde_json::from_slice(&body)?;
    let interaction_id = interaction.id;
    let interaction_token = interaction.token.clone();
    let responder_handle = tokio::spawn(state.bot.clone().run(interaction));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    if responder_handle.is_finished() {
        match responder_handle.await {
            Ok(v) => {
                return Ok(Json(v));
            }
            Err(e) => {
                error!("Handler panicked with {e}");
                return Ok(Json(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(
                        InteractionResponseDataBuilder::new()
                            .content(format!("Handler panicked with {e}"))
                            .flags(MessageFlags::EPHEMERAL)
                            .build(),
                    ),
                }));
            }
        }
    }
    tokio::spawn(respond_to_discord_later(
        state.bot,
        interaction_id,
        interaction_token,
        responder_handle,
    ));
    let response = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .build(),
        ),
    };
    Ok(Json(response))
}

async fn respond_to_discord_later(
    state: XpdSlash,
    id: Id<InteractionMarker>,
    token: String,
    handle: JoinHandle<InteractionResponse>,
) {
    let response = match handle.await {
        Ok(v) => v,
        Err(e) => {
            error!("Handler panicked with {e}");
            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .content(format!("Handler panicked with {e}"))
                        .flags(MessageFlags::EPHEMERAL)
                        .build(),
                ),
            }
        }
    };
    if let Err(e) = state
        .client()
        .interaction(state.id())
        .create_response(id, &token, &response)
        .await
    {
        error!("Failed to create true response: {e}");
    }
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
