use std::ops::Deref;

use twilight_interactions::command::CommandOption;
use twilight_interactions::command::CreateOption;
use xpd_rank_card::colors::{
    Color, Colors, DEFAULT_BACKGROUND, DEFAULT_BORDER, DEFAULT_IMPORTANT, DEFAULT_LEVEL,
    DEFAULT_PROGRESS_BACKGROUND, DEFAULT_PROGRESS_FOREGROUND, DEFAULT_RANK, DEFAULT_SECONDARY,
};
use xpd_rank_card::from_maybe_hex;
pub async fn for_user(
    db: &sqlx::PgPool,
    id: twilight_model::id::Id<twilight_model::id::marker::UserMarker>,
) -> Colors {
    #[allow(clippy::cast_possible_wrap)]
    let Ok(colors) =
        sqlx::query!("SELECT * FROM custom_card WHERE id = $1", id.get() as i64)
            .fetch_one(db)
            .await else {
        return Colors::default();
    };
    Colors {
        important: from_maybe_hex!(colors.important, DEFAULT_IMPORTANT),
        secondary: from_maybe_hex!(colors.secondary, DEFAULT_SECONDARY),
        rank: from_maybe_hex!(colors.rank, DEFAULT_RANK),
        level: from_maybe_hex!(colors.level, DEFAULT_LEVEL),
        border: from_maybe_hex!(colors.border, DEFAULT_BORDER),
        background: from_maybe_hex!(colors.background, DEFAULT_BACKGROUND),
        progress_foreground: from_maybe_hex!(
            colors.progress_foreground,
            DEFAULT_PROGRESS_FOREGROUND
        ),
        progress_background: from_maybe_hex!(
            colors.progress_background,
            DEFAULT_PROGRESS_BACKGROUND
        ),
    }
}

pub struct ColorOption(Color);

impl Deref for ColorOption {
    type Target = Color;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandOption for ColorOption {
    fn from_option(
        value: twilight_model::application::interaction::application_command::CommandOptionValue,
        _data: twilight_interactions::command::internal::CommandOptionData,
        _resolved: Option<&twilight_model::application::interaction::application_command::CommandInteractionDataResolved>,
    ) -> Result<Self, twilight_interactions::error::ParseOptionErrorType> {
        if let twilight_model::application::interaction::application_command::CommandOptionValue::String(string) = value {
            Ok(Self(Color::from_hex(&string).map_err(|e| twilight_interactions::error::ParseOptionErrorType::InvalidChoice(format!("{e}")))?))
        } else {
            Err(twilight_interactions::error::ParseOptionErrorType::InvalidType(value.kind()))
        }
    }
}

impl CreateOption for ColorOption {
    fn create_option(
        data: twilight_interactions::command::internal::CreateOptionData,
    ) -> twilight_model::application::command::CommandOption {
        twilight_model::application::command::CommandOption {
            autocomplete: Some(data.autocomplete),
            channel_types: None,
            choices: None,
            description: data.description,
            description_localizations: data.description_localizations,
            kind: twilight_model::application::command::CommandOptionType::String,
            max_length: Some(7),
            max_value: None,
            min_length: Some(6),
            min_value: None,
            name: data.name,
            name_localizations: data.name_localizations,
            options: None,
            required: data.required,
        }
    }
}
