use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::id_to_db;

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
        AdminCommand::Leave(lg) => leave_guild(state, lg).await,
        AdminCommand::ResetGuild(rg) => reset_guild(state, rg).await,
        AdminCommand::ResetUser(ru) => reset_user(state, ru).await,
        AdminCommand::SetNick(sn) => set_nick(state, sn).await,
        AdminCommand::BanGuild(bg) => ban_guild(state, bg).await,
        AdminCommand::PardonGuild(pg) => pardon_guild(state, pg).await,
    }?;
    Ok(XpdSlashResponse::new()
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn leave_guild(state: SlashState, leave: AdminCommandLeave) -> Result<String, Error> {
    let guild: Id<GuildMarker> = leave.guild.parse()?;
    state.client.leave_guild(guild).await?;
    Ok(format!("Left guild {guild}"))
}

async fn reset_guild(state: SlashState, leave: AdminCommandResetGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = leave.guild.parse()?;
    query!("DELETE FROM levels WHERE guild = $1", id_to_db(guild))
        .execute(&state.db)
        .await?;
    Ok(format!("Reset levels for guild {guild}"))
}

async fn reset_user(state: SlashState, leave: AdminCommandResetUser) -> Result<String, Error> {
    let guild_db = id_to_db(leave.user);
    query!("DELETE FROM levels WHERE id = $1", guild_db)
        .execute(&state.db)
        .await?;
    Ok(format!("Reset global levels for <@{}>", leave.user))
}

async fn set_nick(state: SlashState, nick: AdminCommandSetNick) -> Result<String, Error> {
    let guild: Id<GuildMarker> = nick.guild.parse()?;
    state
        .client
        .update_current_member(guild)
        .nick(nick.name.as_deref())
        .await?;
    Ok(format!(
        "Set nickname to {} in {guild}",
        nick.name.unwrap_or_else(|| "{default}".to_string())
    ))
}

async fn ban_guild(state: SlashState, ban: AdminCommandBanGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = ban.guild.parse()?;
    query!(
        "INSERT INTO guild_bans (id, expires) \
            VALUES ($1, \
            CASE WHEN $3 \
            THEN NULL \
            ELSE NOW() + interval '1' day * $2 END)",
        id_to_db(guild),
        ban.duration,
        ban.duration.is_none()
    )
    .execute(&state.db)
    .await?;
    Ok(format!("Banned guild {guild}"))
}

async fn pardon_guild(state: SlashState, pardon: AdminCommandPardonGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = pardon.guild.parse()?;
    query!("DELETE FROM guild_bans WHERE id = $1", id_to_db(guild))
        .execute(&state.db)
        .await?;
    Ok(format!("Pardoned guild {guild}"))
}
