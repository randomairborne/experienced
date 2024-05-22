use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::XpdSlashResponse;

pub fn help() -> XpdSlashResponse {
    let help_help = EmbedFieldBuilder::new("/help", "This command! Takes no arguments.")
        .inline()
        .build();
    let rank_help = EmbedFieldBuilder::new(
        "/rank",
        "Get someone's rank. Takes one optional argument, for the user to rank.",
    )
    .inline()
    .build();
    let card_help = EmbedFieldBuilder::new("/card", "Interact with cards. Anything with an open-ended input needs a hex code. You can `/card fetch` anyone's card with its optional user argument.")
        .inline()
        .build();
    let xp_help = EmbedFieldBuilder::new("/xp", "Commands to manage the bot in this server.")
        .inline()
        .build();
    let author = EmbedAuthorBuilder::new("Join our support server!")
        .url("https://valk.sh/discord")
        .build();
    let footer = EmbedFooterBuilder::new("Thank you for using experienced.")
        .icon_url(ImageSource::url("https://xp.valk.sh/favicon.png").unwrap())
        .build();
    let help_embed = EmbedBuilder::new()
        .color(crate::THEME_COLOR)
        .author(author)
        .title("Experienced Help")
        .field(help_help)
        .field(rank_help)
        .field(card_help)
        .field(xp_help)
        .footer(footer)
        .build();
    XpdSlashResponse::new().ephemeral(true).embeds([help_embed])
}
