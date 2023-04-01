#![allow(clippy::module_name_repetitions)]
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption, ResolvedUser,
};

use crate::colors::ColorOption;

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
    name = "edit",
    desc = "Edit card colors by specifying hex codes for values you would like to change."
)]
pub struct CardCommandEdit {
    #[command(desc = "What color to use for the background")]
    pub background: Option<ColorOption>,
    #[command(desc = "What color to use for the border")]
    pub border: Option<ColorOption>,
    #[command(desc = "What color to use for the important informational text")]
    pub important: Option<ColorOption>,
    #[command(desc = "What color to use for the secondary informational text")]
    pub secondary: Option<ColorOption>,
    #[command(desc = "What color to use for your rank")]
    pub rank: Option<ColorOption>,
    #[command(desc = "What color to use for your level")]
    pub level: Option<ColorOption>,
    #[command(desc = "What color to use for the progress bar's filled part")]
    pub progress_foreground: Option<ColorOption>,
    #[command(desc = "What color to use for the progress bar's empty part")]
    pub progress_background: Option<ColorOption>,
    #[command(desc = "What font to use in the card")]
    pub font: Option<CardCommandEditFont>,
    #[command(desc = "What toy image to use in the card")]
    pub toy_image: Option<CardCommandEditToy>,
}

#[derive(CommandOption, CreateOption)]
pub enum CardCommandEditFont {
    #[option(name = "Mojangles", value = "Mojang")]
    Mojang,
    #[option(name = "Roboto", value = "Roboto")]
    Roboto,
    #[option(name = "JetBrains Mono", value = "JetBrains Mono")]
    JetBrainsMono,
    #[option(name = "Montserrat Alt1", value = "Montserrat-Alt1")]
    MontserratAlt1,
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
