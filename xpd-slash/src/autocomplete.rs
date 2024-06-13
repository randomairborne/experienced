use twilight_model::{
    application::interaction::application_command::CommandData,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub async fn autocomplete(data: Box<CommandData>) -> InteractionResponse {
    let ird = InteractionResponseDataBuilder::new().choices([]).build();
    InteractionResponse {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        data: Some(ird),
    }
}
