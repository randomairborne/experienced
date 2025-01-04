use twilight_model::http::interaction::InteractionResponseType;

use crate::{response::XpdInteractionResponse, XpdSlashResponse};

pub fn help() -> XpdInteractionResponse {
    const HELP_MESSAGE: &str = "Visit [the docs](<https://xp.valk.sh/docs/>) or [join the discord](<https://valk.sh/discord>)";
    XpdSlashResponse::with_embed_text(HELP_MESSAGE)
        .ephemeral(true)
        .into_interaction_response(InteractionResponseType::DeferredChannelMessageWithSource)
}
