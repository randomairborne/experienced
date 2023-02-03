use twilight_model::{
    http::interaction::InteractionResponse,
    user::User,
};
use twilight_util::builder::{
    embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder},
    InteractionResponseDataBuilder,
};

pub fn help(invoker: &User) -> InteractionResponse {
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
    let help_embed = EmbedBuilder::new()
        .color(crate::THEME_COLOR)
        .title("Experienced Help")
        .field(help_help)
        .field(rank_help)
        .field(card_help)
        .field(xp_help)
        .footer(
            EmbedFooterBuilder::new(format!(
                "Requested by {}#{}",
                invoker.name,
                invoker.discriminator()
            ))
            .build(),
        )
        .build();
    let data = InteractionResponseDataBuilder::new()
        .embeds([help_embed])
        .build();
    let data = Some(data);
    InteractionResponse {
        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
        data,
    }
}
