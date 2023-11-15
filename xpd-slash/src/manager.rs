use std::fmt::Write;

use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::EmbedBuilder;

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
    interaction_token: String,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = match data {
        XpCommand::Rewards(rewards) => process_rewards(rewards, guild_id, state).await,
        XpCommand::Experience(experience) => {
            process_experience(experience, guild_id, interaction_token, state).await
        }
    }?;
    Ok(XpdSlashResponse::new().embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn process_experience(
    data: XpCommandExperience,
    guild_id: Id<GuildMarker>,
    interaction_token: String,
    state: SlashState,
) -> Result<String, Error> {
    match data {
        XpCommandExperience::Import(_) => {
            import_level_data(guild_id, interaction_token, state).await
        }
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
    #[allow(clippy::cast_possible_wrap)]
    let xp = query!(
        "UPDATE levels SET xp = xp + $3 WHERE id = $1 AND guild = $2 RETURNING xp",
        user_id.get() as i64,
        guild_id.get() as i64,
        amount
    )
    .fetch_one(&state.db)
    .await?
    .xp;
    #[allow(clippy::cast_sign_loss)]
    let current_level = mee6::LevelInfo::new(xp as u64).level();
    let action = if amount.is_positive() {
        "Added"
    } else {
        "Removed"
    };
    let amount_abs = amount.abs();
    Ok(format!("{action} {amount_abs} XP from <@{user_id}>, leaving them with {xp} XP at level {current_level}"))
}

async fn reset_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    state: SlashState,
) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "DELETE FROM levels WHERE id = $1 AND guild = $2",
        user_id.get() as i64,
        guild_id.get() as i64
    )
    .execute(&state.db)
    .await?;
    Ok(format!(
        "Deleted <@{user_id}> from my database in this server!"
    ))
}

async fn import_level_data(
    guild_id: Id<GuildMarker>,
    interaction_token: String,
    state: SlashState,
) -> Result<String, Error> {
    let ratelimiting_key = format!("ratelimit-import-mee6-{}", guild_id.get());
    let mut redis = state.redis.get().await?;
    let time_remaining_option: Option<isize> = redis::cmd("TTL")
        .arg(&ratelimiting_key)
        .query_async(&mut redis)
        .await?;
    let time_remaining = time_remaining_option.unwrap_or(0);
    if time_remaining > 0 {
        return Ok(format!(
            "This guild is being ratelimited. Try again in {time_remaining} seconds."
        ));
    }
    let total_users = state
        .client
        .guild(guild_id)
        .with_counts(true)
        .await?
        .model()
        .await?
        .approximate_member_count;
    if let Some(total) = total_users {
        if total > 10_000 {
            return Err(Error::TooManyUsersForImport);
        }
    }
    state
        .import_queue
        .mee6
        .lock()
        .push_back((guild_id, interaction_token));
    {
        let mut redis = state.redis.get().await?;
        redis::cmd("SET")
            .arg(ratelimiting_key)
            .arg(3600)
            .arg("EX")
            .arg(3600)
            .query_async(&mut redis)
            .await?;
    }
    Ok("Importing user data- check back here soon!".to_string())
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
    state: SlashState,
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
async fn process_rewards_list(
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
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
    if data.is_empty() {
        data = "No role rewards set for this server".to_string();
    }
    Ok(data)
}
