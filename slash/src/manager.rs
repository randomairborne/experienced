use sqlx::query;
use std::fmt::Write;
use twilight_model::{
    channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
    id::{marker::GuildMarker, Id},
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::{
    cmd_defs::{
        manage::{
            XpCommandExperience, XpCommandExperienceImport, XpCommandRewards, XpCommandRewardsAdd,
            XpCommandRewardsRemove,
        },
        XpCommand,
    },
    AppState,
};

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
    data: XpCommand,
    guild_id: Option<Id<GuildMarker>>,
    state: AppState,
) -> Result<InteractionResponseData, Error> {
    let guild_id = guild_id.ok_or(Error::MissingGuildId)?;
    let contents = match data {
        XpCommand::Rewards(rewards) => process_rewards(rewards, guild_id, state).await,
        XpCommand::Experience(experience) => process_experience(experience, guild_id, state).await,
    }?;
    Ok(InteractionResponseDataBuilder::new()
        .flags(MessageFlags::EPHEMERAL)
        .embeds([EmbedBuilder::new().description(contents).build()])
        .build())
}

async fn process_experience(
    data: XpCommandExperience,
    guild_id: Id<GuildMarker>,
    state: AppState,
) -> Result<String, Error> {
    match data {
        XpCommandExperience::Import(import) => process_import(import, guild_id, state).await,
    }
}

async fn process_import(
    data: XpCommandExperienceImport,
    guild_id: Id<GuildMarker>,
    mut state: AppState,
) -> Result<String, Error> {
    let time_remaining: isize = redis::cmd("TTL")
        .arg(guild_id.get())
        .query_async(&mut state.redis)
        .await?;
    if time_remaining > 0 {
        return Ok(format!(
            "This guild is being ratelimited. Try again in {time_remaining} seconds."
        ));
    }
    let _: () = redis::cmd("SET")
        .arg(guild_id.get())
        .arg(3600)
        .arg("EX")
        .arg(3600)
        .query_async(&mut state.redis)
        .await?;
    let mee6_users: Vec<Mee6User> = state.http.get(data.levels.url).send().await?.json().await?;
    let user_count = mee6_users.len();
    let mut csv_writer = csv::Writer::from_writer(Vec::new());
    for user in mee6_users {
        #[allow(clippy::cast_possible_wrap)]
        let xp_user = XpUserGuildLevel {
            id: user.id as i64,
            #[allow(clippy::cast_possible_wrap)]
            guild: guild_id.get() as i64,
            xp: user.xp,
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
    Ok(format!("Imported {user_count} rows of user leveling data!"))
}

async fn process_rewards<'a>(
    cmd: XpCommandRewards,
    guild_id: Id<GuildMarker>,
    state: AppState,
) -> Result<String, Error> {
    match cmd {
        XpCommandRewards::Add(add) => process_rewards_add(add, state, guild_id).await,
        XpCommandRewards::Remove(remove) => process_rewards_rm(remove, state, guild_id).await,
        XpCommandRewards::List(_list) => process_rewards_list(state, guild_id).await,
    }
}

async fn process_rewards_add(
    options: XpCommandRewardsAdd,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "INSERT INTO role_rewards (id, requirement, guild) VALUES ($1, $2, $3)",
        options.role.id.get() as i64,
        options.level,
        guild_id.get() as i64
    )
    .execute(&state.db)
    .await?;
    Ok(format!(
        "Added role reward <@&{}> at level {}!",
        options.role.id, options.level
    ))
}
async fn process_rewards_rm(
    options: XpCommandRewardsRemove,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    if let Some(role) = options.role {
        #[allow(clippy::cast_possible_wrap)]
        query!(
            "DELETE FROM role_rewards WHERE id = $1 AND guild = $2",
            role.get() as i64,
            guild_id.get() as i64
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward <@&{role}>!"));
    } else if let Some(level) = options.level {
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
    #[error("Discord did not send a guild ID!")]
    MissingGuildId,
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
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Discord API error: {0}")]
    DiscordApi(#[from] twilight_http::Error),
    #[error("Discord API decoding error: {0}")]
    DiscordApiDeserialization(#[from] twilight_http::response::DeserializeBodyError),
}
