use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::XpdSlashResponse;

pub fn help() -> XpdSlashResponse {
    const HELP_MESSAGE: &str = "Visit [the docs](<https://xp.valk.sh/docs/>) or [join the discord](<https://valk.sh/discord>)";
    XpdSlashResponse::with_embed_text(HELP_MESSAGE).ephemeral(true)
}
