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
    #[error("SQLx encountered an error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Command had wrong number of arguments!")]
    WrongArgumentCount(&'static str),
    #[error("Rust writeln! returned an error")]
    Fmt(#[from] std::fmt::Error),
    #[error("Could not convert string to int")]
    StrToInt(#[from] std::num::ParseIntError),
    #[error("CSV error")]
    Csv(#[from] csv::Error),
    #[error("JSON error")]
    Json(#[from] serde_json::Error),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("Could not build template: {0}")]
    SimpleInterpolation(#[from] simpleinterpolation::Error),
    #[error("Discord API decoding error")]
    DiscordApiDeserialization(#[from] twilight_http::response::DeserializeBodyError),
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
    #[error("That file is too big to import automatically. Please email valk@randomairborne.dev or [join our support server](https://discord.com/invite/KWkPYxqNKe) to set up imports for your server.")]
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
}
