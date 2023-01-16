use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand};

#[derive(CommandModel, CreateCommand)]
#[command(name = "reset", desc = "Reset your card to defaults")]
pub struct CommandReset;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "fetch",
    desc = "Get your current card settings, including defaults."
)]
pub struct CommandFetch;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "edit",
    desc = "Edit card colors by specifying hex codes for values you would like to change."
)]
pub struct CommandEdit {
    #[command(desc = "What color to use for the background")]
    pub background: Option<String>,
    #[command(desc = "What color to use for the border")]
    pub border: Option<String>,
    #[command(desc = "What color to use for the important informational text")]
    pub important: Option<String>,
    #[command(desc = "What color to use for the secondary informational text")]
    pub secondary: Option<String>,
    #[command(desc = "What color to use for your rank")]
    pub rank: Option<String>,
    #[command(desc = "What color to use for your level")]
    pub level: Option<String>,
    #[command(desc = "What color to use for the progress bar's filled part")]
    pub progress_foreground: Option<String>,
    #[command(desc = "What color to use for the progress bar's empty part")]
    pub progress_background: Option<String>,
    #[command(desc = "What font to use in the card")]
    pub font: Option<String>,
}

#[derive(CommandOption)]
pub enum CommandEditFont {
    #[option(name = "Mojangles", value = "Mojang")]
    Mojang,
    #[option(name = "Roboto", value = "Roboto")]
    Roboto,
    #[option(name = "JetBrains Mono", value = "JetBrains Mono")]
    JetBrainsMono,
    #[option(name = "Montserrat Alt1", value = "Montserrat Alt1")]
    MontserratAlt1,
}
