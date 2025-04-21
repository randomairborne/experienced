use twilight_model::http::interaction::InteractionResponseType;
use xpd_common::{CURRENT_GIT_REV_COUNT, CURRENT_GIT_SHA};

use crate::{XpdInteractionData, response::XpdInteractionResponse};

pub fn help() -> XpdInteractionResponse {
    const HELP_MESSAGE: &str = "Visit [the docs](<https://xp.valk.sh/docs/>) or [join the discord](<https://valk.sh/discord>) to learn how to use Experienced!";
    let message = format!(
        "{HELP_MESSAGE}\n\nBot git revision `{CURRENT_GIT_SHA}`, commit number {CURRENT_GIT_REV_COUNT}"
    );
    XpdInteractionData::with_embed_text(message)
        .ephemeral(true)
        .into_interaction_response(InteractionResponseType::ChannelMessageWithSource)
}
