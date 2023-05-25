#[derive(Debug, thiserror::Error)]
pub enum Error {
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
    #[error("There are too many users to import automatically. Please email valk@randomairborne.dev to set up imports for your server.")]
    TooManyUsersForImport,
    #[error("Interaction parser encountered an error: {0}!")]
    Parse(#[from] twilight_interactions::error::ParseError),
    #[error("Discord error: {0}!")]
    TwilightHttp(#[from] twilight_http::Error),
    #[error("HTTP error: {0}!")]
    ReqwestHttp(#[from] reqwest::Error),
    #[error("Invalid constructed message: {0}!")]
    Validate(#[from] twilight_validate::message::MessageValidationError),
    #[error("Invalid message attachment: {0}!")]
    ImageSourceAttachment(
        #[from] twilight_util::builder::embed::image_source::ImageSourceAttachmentError,
    ),
    #[error("SVG renderer encountered an error: {0}!")]
    ImageGenerator(#[from] xpd_rank_card::Error),
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Command had wrong number of arguments: {0}!")]
    WrongArgumentCount(&'static str),
    #[error("CSV encountered an error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Rust writeln! returned an error: {0}")]
    Fmt(#[from] std::fmt::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Discord API decoding error: {0}")]
    DiscordApiDeserialization(#[from] twilight_http::response::DeserializeBodyError),
}
