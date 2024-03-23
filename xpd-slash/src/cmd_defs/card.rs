#![allow(clippy::module_name_repetitions)]
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption, ResolvedUser,
};
use xpd_rank_card::{customizations::Color, Font};

#[derive(CommandModel, CreateCommand)]
#[command(name = "reset", desc = "Reset your card to defaults")]
pub struct CardCommandReset;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "fetch",
    desc = "Get your current card settings, including defaults."
)]
pub struct CardCommandFetch {
    #[command(desc = "User to fetch settings of")]
    pub user: Option<ResolvedUser>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "fetch",
    desc = "Get your server's current default card settings."
)]
pub struct GuildCardCommandFetch;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "edit",
    desc = "Edit card colors by specifying hex codes for values you would like to change."
)]
pub struct CardCommandEdit {
    #[command(desc = "What color to use for the background")]
    pub background: Option<ColorOption>,
    #[command(desc = "What color to use for the border")]
    pub border: Option<ColorOption>,
    #[command(desc = "What color to use for your username")]
    pub username: Option<ColorOption>,
    #[command(desc = "What color to use for your rank")]
    pub rank: Option<ColorOption>,
    #[command(desc = "What color to use for your level")]
    pub level: Option<ColorOption>,
    #[command(desc = "What color to use for the progress bar's filled part")]
    pub progress_foreground: Option<ColorOption>,
    #[command(desc = "What color to use for the progress bar's empty part")]
    pub progress_background: Option<ColorOption>,
    #[command(desc = "What color to use for the xp count when in the progress bar's filled part")]
    pub foreground_xp_count: Option<ColorOption>,
    #[command(desc = "What color to use for the xp count when in the progress bar's empty part")]
    pub background_xp_count: Option<ColorOption>,
    #[command(desc = "What font to use in the card")]
    pub font: Option<CardCommandEditFont>,
    #[command(desc = "What toy image to use in the card")]
    pub toy_image: Option<CardCommandEditToy>,
    #[command(desc = "What layout to use for the card")]
    pub card_layout: Option<CardCommandEditLayout>,
}

#[derive(CommandOption, CreateOption)]
pub enum CardCommandEditFont {
    #[option(name = "Mojangles", value = 0)]
    Mojang,
    #[option(name = "Roboto", value = 1)]
    Roboto,
    #[option(name = "JetBrains Mono", value = 2)]
    JetBrainsMono,
    #[option(name = "Montserrat Alt1", value = 3)]
    MontserratAlt1,
}

impl CardCommandEditFont {
    pub const fn as_xpd_rank_card(&self) -> Font {
        match self {
            Self::Mojang => Font::Mojang,
            Self::Roboto => Font::Roboto,
            Self::JetBrainsMono => Font::JetBrainsMono,
            Self::MontserratAlt1 => Font::MontserratAlt1,
        }
    }
}

#[derive(CommandOption, CreateOption, Default)]
pub enum CardCommandEditLayout {
    #[default]
    #[option(name = "Classic", value = "classic.svg")]
    Classic,
    #[option(name = "Vertical", value = "vertical.svg")]
    Vertical,
}

#[derive(
    CommandOption, CreateOption, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, sqlx::Type,
)]
pub enum CardCommandEditToy {
    #[option(name = "None", value = "None")]
    None,
    #[option(name = "Airplane", value = "airplane.png")]
    Airplane,
    #[option(name = "Bee", value = "bee.png")]
    Bee,
    #[option(name = "Biscuit", value = "biscuit.png")]
    Biscuit,
    #[option(name = "Chicken", value = "chicken.png")]
    Chicken,
    #[option(name = "Cow", value = "cow.png")]
    Cow,
    #[option(name = "Fox", value = "fox.png")]
    Fox,
    #[option(name = "Grass Block", value = "grassblock.png")]
    GrassBlock,
    #[option(name = "Parrot", value = "parrot.png")]
    Parrot,
    #[option(name = "Pickaxe", value = "pickaxe.png")]
    Pickaxe,
    #[option(name = "Pig", value = "pig.png")]
    Pig,
    #[option(name = "Blue Potion", value = "potion_blue.png")]
    PotionBlue,
    #[option(name = "Purple Potion", value = "potion_purple.png")]
    PotionPurple,
    #[option(name = "Red Potion", value = "potion_red.png")]
    PotionRed,
    #[option(name = "Sheep", value = "sheep.png")]
    Sheep,
    #[option(name = "Steve Hug", value = "steveheart.png")]
    SteveHeart,
    #[option(name = "Tree", value = "tree.png")]
    Tree,
}

pub struct ColorOption(Color);

impl ColorOption {
    pub fn string(self) -> String {
        self.to_string()
    }
}

impl std::ops::Deref for ColorOption {
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
