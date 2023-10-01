use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::EmbedBuilder;

use crate::{
    cmd_defs::{
        admin::{
            AdminCommandBanGuild, AdminCommandLeave, AdminCommandPardonGuild,
            AdminCommandResetGuild, AdminCommandResetUser, AdminCommandSetNick,
        },
        AdminCommand,
    },
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_admin(
    data: AdminCommand,
    guild_id: Id<GuildMarker>,
    invoker: Id<UserMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    if guild_id != state.control_guild {
        return Err(Error::NotControlGuild);
    };
    if !state.owners.contains(&invoker) {
        return Err(Error::NotControlUser);
    }
    let contents = match data {
        AdminCommand::Leave(leave) => do_leave(state, leave).await,
        AdminCommand::ResetGuild(reset_guild) => do_reset_guild(state, reset_guild).await,
        AdminCommand::ResetUser(reset_user) => do_reset_user(state, reset_user).await,
        AdminCommand::SetNick(set_nick) => do_set_nick(state, set_nick).await,
        AdminCommand::BanGuild(ban_guild) => do_ban_guild(state, ban_guild).await,
        AdminCommand::PardonGuild(pardon_guild) => do_pardon_guild(state, pardon_guild).await,
    }?;
    Ok(XpdSlashResponse::new().embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn do_leave(state: SlashState, leave: AdminCommandLeave) -> Result<String, Error> {
    let guild: Id<GuildMarker> = leave.guild.parse()?;
    state.client.leave_guild(guild).await?;
    Ok(format!("Left guild {guild}"))
}

async fn do_reset_guild(state: SlashState, leave: AdminCommandResetGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = leave.guild.parse()?;
    #[allow(clippy::cast_possible_wrap)]
    let guild_db = guild.get() as i64;
    query!("DELETE FROM levels WHERE guild = $1", guild_db)
        .execute(&state.db)
        .await?;
    Ok(format!("Reset levels for guild {guild}"))
}

async fn do_reset_user(state: SlashState, leave: AdminCommandResetUser) -> Result<String, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let guild_db = leave.user.get() as i64;
    query!("DELETE FROM levels WHERE id = $1", guild_db)
        .execute(&state.db)
        .await?;
    query!("DELETE FROM custom_card WHERE id = $1", guild_db)
        .execute(&state.db)
        .await?;
    Ok(format!("Reset global levels for <@{}>", leave.user))
}

async fn do_set_nick(state: SlashState, nick: AdminCommandSetNick) -> Result<String, Error> {
    let guild: Id<GuildMarker> = nick.guild.parse()?;
    state
        .client
        .update_current_member(guild)
        .nick(nick.name.as_deref())?
        .await?;
    Ok(format!(
        "Set nickname to {} in {guild}",
        nick.name.unwrap_or_else(|| "{default}".to_string())
    ))
}

async fn do_ban_guild(state: SlashState, ban: AdminCommandBanGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = ban.guild.parse()?;
    #[allow(clippy::cast_possible_wrap)]
    let guild_db = guild.get() as i64;
    query!(
        "INSERT INTO guild_bans (id, expires) \
            VALUES ($1, \
            CASE WHEN $3 \
            THEN NULL \
            ELSE NOW() + interval '1' day * $2 END)",
        guild_db,
        ban.duration,
        ban.duration.is_none()
    )
    .execute(&state.db)
    .await?;
    Ok(format!("Banned guild {guild}"))
}

async fn do_pardon_guild(
    state: SlashState,
    pardon: AdminCommandPardonGuild,
) -> Result<String, Error> {
    let guild: Id<GuildMarker> = pardon.guild.parse()?;
    #[allow(clippy::cast_possible_wrap)]
    let guild_db = guild.get() as i64;
    query!("DELETE FROM guild_bans WHERE id = $1", guild_db)
        .execute(&state.db)
        .await?;
    Ok(format!("Pardoned guild {guild}"))
}
