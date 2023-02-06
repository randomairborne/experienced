#![allow(clippy::module_name_repetitions)]
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption, ResolvedUser,
};

use crate::colors::Color;

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
    pub background: Option<Color>,
    #[command(desc = "What color to use for the border")]
    pub border: Option<Color>,
    #[command(desc = "What color to use for the important informational text")]
    pub important: Option<Color>,
    #[command(desc = "What color to use for the secondary informational text")]
    pub secondary: Option<Color>,
    #[command(desc = "What color to use for your rank")]
    pub rank: Option<Color>,
    #[command(desc = "What color to use for your level")]
    pub level: Option<Color>,
    #[command(desc = "What color to use for the progress bar's filled part")]
    pub progress_foreground: Option<Color>,
    #[command(desc = "What color to use for the progress bar's empty part")]
    pub progress_background: Option<Color>,
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
    #[option(name = "Parrot", value = "parrot.png")]
    Parrot,
    #[option(name = "Fox", value = "fox.png")]
    Fox,
    #[option(name = "Steve heart", value = "steveheart.png")]
    SteveHeart,
    #[option(name = "Grass block", value = "grassblock.png")]
    GrassBlock,
    #[option(name = "Diamond pickaxe", value = "pickaxe.png")]
    Pickaxe,
    #[option(name = "Tree", value = "tree.png")]
    Tree,
}
