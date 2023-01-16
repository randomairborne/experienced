use sqlx::query;
use std::{collections::HashMap, fmt::Write};
use twilight_model::{
    application::interaction::application_command::{
        CommandData, CommandDataOption, CommandOptionValue,
    },
    channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::AppState;

pub async fn process_import(
    data: CommandData,
    guild_id: Option<Id<GuildMarker>>,
    _invoker: &User,
    state: AppState,
) -> Result<InteractionResponseData, Error> {
    let guild_id = guild_id.ok_or(Error::MissingGuildId)?.get() as i64;
    let resolved = data.resolved.ok_or(Error::NoResolvedData)?;
    for option in data.options {
        if option.name == "levels" {
            if let CommandOptionValue::Attachment(attachment_id) = option.value {
                let attachment = resolved
                    .attachments
                    .get(&attachment_id)
                    .ok_or(Error::NoAttachment)?;
                let mee6_users: Vec<Mee6User> =
                    state.http.get(&attachment.url).send().await?.json().await?;
                let mut csv_writer = csv::Writer::from_writer(Vec::new());
                for user in mee6_users {
                    let xp_user = XpUserGuildLevel {
                        id: user.id as i64,
                        guild: guild_id,
                        xp: user.xp as i64,
                    };
                    csv_writer.serialize(xp_user)?;
                }
                let csv = csv_writer.into_inner().map_err(|_| Error::CsvIntoInner)?;
                let mut copier = state
                    .db
                    .copy_in_raw("COPY levels FROM STDIN WITH (FORMAT csv)")
                    .await?;
                copier.send(csv).await?;
                copier.finish().await?;
            }
        }
    }
    Err(Error::InvalidSubcommand)
}

#[derive(serde::Deserialize, serde::Serialize, Copy, Clone)]
struct Mee6User {
    pub id: u64,
    pub level: i64,
    pub xp: i64,
}

#[derive(serde::Deserialize, serde::Serialize, Copy, Clone)]
struct XpUserGuildLevel {
    pub id: i64,
    pub guild: i64,
    pub xp: i64,
}

pub async fn process_xp(
    data: CommandData,
    guild_id: Option<Id<GuildMarker>>,
    _invoker: &User,
    state: AppState,
) -> Result<InteractionResponseData, Error> {
    let guild_id = guild_id.ok_or(Error::MissingGuildId)?;
    for maybe_group in data.options {
        if let CommandOptionValue::SubCommandGroup(group) = maybe_group.value {
            match maybe_group.name.as_str() {
                "rewards" => return process_rewards(group, state, guild_id).await,
                _ => return Err(Error::UnknownSubcommand),
            }
        }
    }
    Err(Error::InvalidSubcommand)
}

async fn process_rewards<'a>(
    options: Vec<CommandDataOption>,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<InteractionResponseData, Error> {
    for maybe_cmd in options {
        let cmd_name = maybe_cmd.name.clone();
        if let CommandOptionValue::SubCommand(opts) = maybe_cmd.value {
            let args: HashMap<String, CommandOptionValue> =
                opts.into_iter().map(|val| (val.name, val.value)).collect();
            let contents = match cmd_name.as_str() {
                "add" => process_rewards_add(args, state, guild_id).await,
                "remove" => process_rewards_rm(args, state, guild_id).await,
                "list" => process_rewards_list(state, guild_id).await,
                _ => return Err(Error::UnknownSubcommand),
            }?;
            return Ok(InteractionResponseDataBuilder::new()
                .content(contents)
                .flags(MessageFlags::EPHEMERAL)
                .build());
        }
    }
    Err(Error::InvalidSubcommand)
}

async fn process_rewards_add(
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    let level_requirement = if let CommandOptionValue::Integer(level) = options
        .get("level")
        .ok_or(Error::MissingRequiredArgument("level"))?
    {
        *level
    } else {
        return Err(Error::WrongArgumentType("level"));
    };
    let role_id = if let CommandOptionValue::Role(role) = options
        .get("role")
        .ok_or(Error::MissingRequiredArgument("role"))?
    {
        *role
    } else {
        return Err(Error::WrongArgumentType("role"));
    };
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "INSERT INTO role_rewards (id, requirement, guild) VALUES ($1, $2, $3)",
        role_id.get() as i64,
        level_requirement as i64,
        guild_id.get() as i64
    )
    .execute(&state.db)
    .await?;
    Ok(format!(
        "Added role reward <@&{role_id}> at level {level_requirement}!",
    ))
}
async fn process_rewards_rm(
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    if let Some(CommandOptionValue::Role(role)) = options.get("role") {
        #[allow(clippy::cast_possible_wrap)]
        query!(
            "DELETE FROM role_rewards WHERE id = $1 AND guild = $2",
            role.get() as i64,
            guild_id.get() as i64
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward <@&{role}>!"));
    } else if let Some(CommandOptionValue::Integer(level)) = options.get("level") {
        #[allow(clippy::cast_possible_wrap)]
        query!(
            "DELETE FROM role_rewards WHERE requirement = $1 AND guild = $2",
            level,
            guild_id.get() as i64
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward for level {level}!"));
    };
    Err(Error::WrongArgumentCount(
        "`/xp rewards remove` requires either a level or a role!",
    ))
}
async fn process_rewards_list(state: AppState, guild_id: Id<GuildMarker>) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let roles = query!(
        "SELECT * FROM role_rewards WHERE guild = $1",
        guild_id.get() as i64
    )
    .fetch_all(&state.db)
    .await?;
    let mut data = String::new();
    for role in roles {
        writeln!(
            data,
            "Role reward <@&{}> at level {}",
            role.id, role.requirement
        )?;
    }
    Ok(data)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Discord sent an invalid subcommand!")]
    InvalidSubcommand,
    #[error("Discord sent an unknown subcommand!")]
    UnknownSubcommand,
    #[error("Discord did not send required argument {0}!")]
    MissingRequiredArgument(&'static str),
    #[error("Discord sent wrong type for required argument {0}!")]
    WrongArgumentType(&'static str),
    #[error("Discord did not send a guild ID!")]
    MissingGuildId,
    #[error("Discord did not send a attachment ResolvedData!")]
    NoResolvedData,
    #[error("Discord did not send ResolvedData for an attachment!")]
    NoAttachment,
    #[error("CSV encountered an IntoInner error")]
    CsvIntoInner,
    #[error("Command had wrong number of arguments: {0}!")]
    WrongArgumentCount(&'static str),
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Colors encountered an error: {0}")]
    Color(#[from] crate::colors::Error),
    #[error("CSV encountered an error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Rust writeln! returned an error: {0}")]
    Fmt(#[from] std::fmt::Error),
    #[error("Http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Discord API error: {0}")]
    DiscordApi(#[from] twilight_http::Error),
    #[error("Discord API decoding error: {0}")]
    DiscordApiDeserialization(#[from] twilight_http::response::DeserializeBodyError),
}
