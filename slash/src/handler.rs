use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{processor::CommandProcessorError, AppState};

pub async fn handle(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<axum::Json<InteractionResponse>, Error> {
    let body = body.to_vec();
    crate::discord_sig_validation::validate_discord_sig(&headers, &body, &state.pubkey)?;
    let interaction: Interaction = serde_json::from_slice(&body)?;
    let response = match crate::processor::process(interaction, state).await {
        Ok(val) => val,
        Err(CommandProcessorError::Manager(crate::manager::Error::Color(e))) => {
            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .content(e.to_string())
                        .build(),
                ),
            }
        }
        Err(e) => {
            error!("{e}");
            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .content(e.to_string())
                        .build(),
                ),
            }
        }
    };
    Ok(Json(response))
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Signature validation error: {0}")]
    Validation(#[from] crate::discord_sig_validation::SignatureValidationError),
    #[error("serde_json validation error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("{self}");
        axum::response::Response::builder()
            .body(axum::body::boxed(axum::body::Full::from(self.to_string())))
            .unwrap()
    }
}
