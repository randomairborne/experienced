use twilight_model::{
    http::interaction::InteractionResponse,
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::{
    embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder},
    InteractionResponseDataBuilder,
};

pub fn help(guild_id: Option<Id<GuildMarker>>, invoker: &User) -> InteractionResponse {
    let help_help = EmbedFieldBuilder::new("/help", "This command! Takes no arguments.")
        .inline()
        .build();
    let rank_help = EmbedFieldBuilder::new(
        "/rank",
        "Get someone's rank. Takes one optional argument, for the user to rank.",
    )
    .inline()
    .build();
    let card_help = EmbedFieldBuilder::new("/card", "Usually, edit your card. Anything with an open-ended input needs a hex code. You can `/card fetch` anyone's card with its optional user argument.")
        .inline()
        .build();
    let xp_help = EmbedFieldBuilder::new("/xp", "Commands to manage the bot in this server.")
        .inline()
        .build();
    let mut help_embed_builder = EmbedBuilder::new()
        .color(0x33_33_66)
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
        );
    if let Some(id) = guild_id {
        let author = EmbedAuthorBuilder::new("Leaderboard")
            .url(format!("https://xp.valk.sh/{id}"))
            .build();
        help_embed_builder = help_embed_builder.author(author);
    }
    let help_embed = help_embed_builder.build();
    let data = InteractionResponseDataBuilder::new()
        .embeds([help_embed])
        .build();
    let data = Some(data);
    InteractionResponse {
        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
        data,
    }
}
