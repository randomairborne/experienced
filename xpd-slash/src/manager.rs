use std::fmt::Write;

use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::id_to_db;

use crate::{
    cmd_defs::{
        manage::{
            XpCommandExperience, XpCommandRewards, XpCommandRewardsAdd, XpCommandRewardsRemove,
        },
        XpCommand,
    },
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_xp(
    data: XpCommand,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = match data {
        XpCommand::Rewards(rewards) => process_rewards(rewards, guild_id, state).await,
        XpCommand::Experience(experience) => process_experience(experience, guild_id, state).await,
    }?;
    Ok(XpdSlashResponse::new()
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn process_experience(
    data: XpCommandExperience,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<String, Error> {
    match data {
        XpCommandExperience::Import(_) => Ok(import_level_data()),
        XpCommandExperience::Add(add) => {
            modify_user_xp(guild_id, add.user, add.amount, state).await
        }
        XpCommandExperience::Remove(rm) => {
            modify_user_xp(guild_id, rm.user, -rm.amount, state).await
        }
        XpCommandExperience::Reset(rst) => reset_user_xp(guild_id, rst.user, state).await,
    }
}

async fn modify_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    amount: i64,
    state: SlashState,
) -> Result<String, Error> {
    let xp = query!(
        "UPDATE levels SET xp = xp + $3 WHERE id = $1 AND guild = $2 RETURNING xp",
        id_to_db(user_id),
        id_to_db(guild_id),
        amount
    )
    .fetch_one(&state.db)
    .await?
    .xp;
    let current_level = mee6::LevelInfo::new(u64::try_from(xp).unwrap_or(0)).level();
    let (action, targeter) = if amount.is_positive() {
        ("Added", "to")
    } else {
        ("Removed", "from")
    };
    let amount_abs = amount.abs();
    Ok(format!("{action} {amount_abs} XP {targeter} <@{user_id}>, leaving them with {xp} XP at level {current_level}"))
}

async fn reset_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    state: SlashState,
) -> Result<String, Error> {
    query!(
        "DELETE FROM levels WHERE id = $1 AND guild = $2",
        id_to_db(user_id),
        id_to_db(guild_id)
    )
    .execute(&state.db)
    .await?;
    Ok(format!(
        "Deleted <@{user_id}> from my database in this server!"
    ))
}

fn import_level_data() -> String {
    concat!(
        "MEE6 has disabled our ability to automatically import your leveling data.",
        "\n",
        "Please join our [support server](https://valk.sh/discord) for further information."
    )
    .to_string()
}

async fn process_rewards<'a>(
    cmd: XpCommandRewards,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<String, Error> {
    match cmd {
        XpCommandRewards::Add(add) => process_rewards_add(add, state, guild_id).await,
        XpCommandRewards::Remove(remove) => process_rewards_rm(remove, state, guild_id).await,
        XpCommandRewards::List(_list) => process_rewards_list(state, guild_id).await,
    }
}

async fn process_rewards_add(
    options: XpCommandRewardsAdd,
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    query!(
        "INSERT INTO role_rewards (id, requirement, guild) VALUES ($1, $2, $3)",
        id_to_db(options.role.id),
        options.level,
        id_to_db(guild_id)
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
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    if let Some(role) = options.role {
        query!(
            "DELETE FROM role_rewards WHERE id = $1 AND guild = $2",
            id_to_db(role),
            id_to_db(guild_id)
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward <@&{role}>!"));
    } else if let Some(level) = options.level {
        query!(
            "DELETE FROM role_rewards WHERE requirement = $1 AND guild = $2",
            level,
            id_to_db(guild_id)
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward for level {level}!"));
    };
    Err(Error::WrongArgumentCount(
        "`/xp rewards remove` requires either a level or a role!",
    ))
}
async fn process_rewards_list(
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    let roles = query!(
        "SELECT * FROM role_rewards WHERE guild = $1",
        id_to_db(guild_id)
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
    if data.is_empty() {
        data = "No role rewards set for this server".to_string();
    }
    Ok(data)
}
