use crate::AppState;
use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use tokio::task::JoinHandle;
use twilight_http::request::application::interaction::CreateFollowup;
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::InteractionMarker, Id},
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
    tokio::spawn(respond_to_discord_later(
        state.bot,
        interaction_token,
        responder_handle,
    ));
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
    let response = match handle.await {
        Ok(v) => v,
        Err(e) => {
            error!("Handler panicked with {e}");
            XpdSlashResponse::new().content(format!("Handler panicked with {e}"))
        }
    };
    let client = state.client().clone();
    let iclient = client.interaction(state.id());
    let followup = iclient.create_followup(&token);
    match build_followup(followup, response) {
        Ok(v) => {
            if let Err(e) = v.await {
                error!(?e, "Failed to create true response");
            }
        }
        Err(e) => {
            error!(?e, "Failed to build followup");
            state
                .client()
                .interaction(state.id())
                .create_followup(&token)
                .content(&format!("failed to build followup message: {e}"))
                .unwrap()
                .await;
        }
    }
}

fn build_followup(
    mut followup: CreateFollowup,
    response: XpdSlashResponse,
) -> Result<CreateFollowup, twilight_validate::message::MessageValidationError> {
    if let Some(option) = response.allowed_mentions {
        followup = followup.allowed_mentions(Some(&option));
    }
    if let Some(option) = response.attachments {
        followup = followup.attachments(&option)?;
    }
    if let Some(option) = response.components {
        followup = followup.components(&option)?;
    }
    if let Some(option) = response.content {
        followup = followup.content(option)?;
    }
    if let Some(option) = response.embeds {
        followup = followup.embeds(&option)?;
    }
    if let Some(option) = response.flags {
        followup = followup.flags(option);
    }
    if let Some(option) = response.tts {
        followup = followup.tts(option);
    }
    Ok(followup)
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
