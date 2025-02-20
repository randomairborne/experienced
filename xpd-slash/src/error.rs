#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Interaction parser encountered an error!")]
    Parse(#[from] twilight_interactions::error::ParseError),
    #[error("Processing task panicked!")]
    TaskPanicked(#[from] tokio::task::JoinError),
    #[error("Discord error!")]
    TwilightHttp(#[from] twilight_http::Error),
    #[error("HTTP error!")]
    ReqwestHttp(#[from] reqwest::Error),
    #[error("Invalid message attachment!")]
    ImageSourceAttachment(
        #[from] twilight_util::builder::embed::image_source::ImageSourceAttachmentError,
    ),
    #[error("SVG renderer encountered an error!")]
    ImageGenerator(#[from] xpd_rank_card::Error),
    #[error("Database encountered an error")]
    Database(#[from] xpd_database::Error),
    #[error("Manual SQLx use encountered an error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Command had wrong number of arguments!")]
    WrongArgumentCount(&'static str),
    #[error("Rust writeln! returned an error")]
    Fmt(#[from] std::fmt::Error),
    #[error("Could not convert string to int")]
    StrToInt(#[from] std::num::ParseIntError),
    #[error("Could not convert one type of int to another")]
    InvalidInt(#[from] std::num::TryFromIntError),
    #[error("CSV error")]
    Csv(#[from] csv::Error),
    #[error("JSON error")]
    Json(#[from] serde_json::Error),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("Could not build template: {0}")]
    SimpleInterpolation(#[from] simpleinterpolation::ParseError),
    #[error("Discord API decoding error")]
    DiscordApiDeserialization(#[from] twilight_http::response::DeserializeBodyError),
    #[error("Invalid guild config: {0}")]
    InvalidGuildConfig(#[from] crate::config::GuildConfigErrorReport),
    #[error("Permission fetch error: {0}")]
    CacheChannel(#[from] xpd_util::PermissionCheckError),
    #[error("Discord sent a command that is not known!")]
    UnrecognizedCommand,
    #[error("Discord did not send a user object for the command invoker when it was required!")]
    NoInvoker,
    #[error("Discord did not send a user object for the command target when it was required!")]
    NoTarget,
    #[error("Discord did not send part of the Resolved Data!")]
    NoResolvedData,
    #[error("Discord did not send target ID for message!")]
    NoMessageTargetId,
    #[error("Discord sent interaction data for an unsupported interaction type!")]
    WrongInteractionData,
    #[error("Discord did not send any interaction data!")]
    NoInteractionData,
    #[error("Discord did not send a guild ID!")]
    NoGuildId,
    #[error("CSV encountered an IntoInner error")]
    CsvIntoInner,
    #[error("Invalid font")]
    InvalidFont,
    #[error("Invalid card")]
    InvalidCard,
    #[error("This command only works in the control guild!")]
    NotControlGuild,
    #[error("This command only works as a control user!")]
    NotControlUser,
    #[error(
        "That file is too big to import automatically. Please email valk@randomairborne.dev or [join our support server](https://discord.com/invite/KWkPYxqNKe) to set up imports for your server."
    )]
    ImportFileTooBig,
    #[error("This page does not exist!")]
    NoUsersForPage,
    #[error("This page does not exist!")]
    PageDoesNotExist,
    #[error("This modal did not contain any action rows!")]
    NoModalActionRow,
    #[error("This modal did not contain the required form field!")]
    NoFormField,
    #[error("This modal did not contain the required form data!")]
    NoDestinationInComponent,
    #[error("HTTP body error!")]
    RawHttpBody,
    #[error("That would make this user's XP negative!")]
    XpWouldBeNegative,
    #[error("Unknown variable `{0}` used in level-up message!")]
    UnknownInterpolationVariable(String),
    #[error("Level up message must be less than 512 characters!")]
    LevelUpMessageTooLong,
    #[error("Level up channel must be a text channel!")]
    LevelUpChannelMustBeText,
    #[error("That card does not exist!")]
    UnknownCard,
    #[error("That toy does not exist!")]
    UnknownToy,
    #[error("That font does not exist!")]
    UnknownFont,
    #[error("There is no autocomplete for that command.")]
    NoAutocompleteForCommand,
    #[error("Discord didn't send an interaction message for that message component")]
    NoInteractionMessage,
    #[error("Discord sent an interaction response message without interaction invocation data")]
    NoInteractionInvocationOnInteractionMessage,
    #[error("You didn't create this leaderboard.")]
    NotYourLeaderboard,
    #[error(
        "Bots do not have leveling data. If one does somehow, you can still use /xp experience reset on it."
    )]
    BotsDontLevel,
    #[error("Nobody in this server is ranked yet.")]
    NoRanksYet,
    #[error("This user does not have a most recent message.")]
    NoLastMessage,
}
