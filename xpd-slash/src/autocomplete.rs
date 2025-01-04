use twilight_interactions::command::{AutocompleteValue, CommandModel};
use twilight_model::{
    application::{
        command::{CommandOptionChoice, CommandOptionChoiceValue},
        interaction::application_command::CommandData,
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use xpd_rank_card::NameableItem;
use xpd_slash_defs::card::CardCommandAutocomplete;

use crate::{
    manage_card::CUSTOM_CARD_NULL_SENTINEL, response::XpdInteractionResponse, Error, SlashState,
    XpdSlashResponse,
};

fn empty_response<T: std::fmt::Debug>(error: T) -> XpdInteractionResponse {
    warn!(?error, "Failed to autocomplete");
    let ird = InteractionResponseDataBuilder::new().build();
    InteractionResponse {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        data: Some(ird),
    }
    .into()
}

pub fn autocomplete(state: &SlashState, data: CommandData) -> XpdInteractionResponse {
    autocomplete_inner(state, data).unwrap_or_else(empty_response)
}

pub fn autocomplete_inner(
    state: &SlashState,
    data: CommandData,
) -> Result<XpdInteractionResponse, Error> {
    debug!(options = ?data, "Got autocomplete");
    let choices = match data.name.as_str() {
        "card" => card_autocomplete(data, state)?.into_iter(),
        "guild-card" => card_autocomplete(data, state)?.into_iter(),
        _ => return Err(Error::NoAutocompleteForCommand),
    };

    let ird = XpdSlashResponse::new().choices(choices.take(25).collect::<Vec<_>>());
    Ok(XpdInteractionResponse::new(
        InteractionResponseType::ApplicationCommandAutocompleteResult,
        ird,
    ))
}

fn card_autocomplete(
    data: CommandData,
    state: &SlashState,
) -> Result<impl IntoIterator<Item = CommandOptionChoice>, Error> {
    let card_autocomplete = CardCommandAutocomplete::from_interaction(data.into())?;

    let CardCommandAutocomplete::Edit(edit) = card_autocomplete else {
        return Err(Error::NoAutocompleteForCommand);
    };

    let fonts = choices(&edit.font, &state.svg.config().fonts, false);
    let cards = choices(&edit.card_layout, &state.svg.config().cards, false);
    let toys = choices(&edit.toy_image, &state.svg.config().toys, true);

    debug!(interaction = ?edit, ?fonts, ?cards, ?toys, "picked out some choices");

    let choice_chain = fonts.into_iter().chain(toys).chain(cards);
    Ok(choice_chain)
}

fn choices<I: NameableItem>(
    auto: &AutocompleteValue<String>,
    options: &[I],
    nullable: bool,
) -> Vec<CommandOptionChoice> {
    let AutocompleteValue::Focused(input) = auto else {
        return Vec::new();
    };

    let mut output = Vec::with_capacity(options.len() + 1);

    if nullable {
        output.push(CommandOptionChoice {
            name: "None".to_string(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(CUSTOM_CARD_NULL_SENTINEL.to_string()),
        });
    }

    for item in options {
        if !item.display_name().contains(input) {
            continue;
        }
        let coc = CommandOptionChoice {
            name: item.display_name().to_owned(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(item.internal_name().to_owned()),
        };
        output.push(coc);
    }
    output
}
