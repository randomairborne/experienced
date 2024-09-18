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
    let rows = xpd_database::delete_levels_guild(&state.db, guild).await?;
    Ok(format!(
        "Reset levels for guild {guild}. It had {rows} users worth of data."
    ))
}

async fn reset_user(state: SlashState, leave: AdminCommandResetUser) -> Result<String, Error> {
    let rows = xpd_database::delete_levels_user(&state.db, leave.user).await?;
    Ok(format!(
        "Reset your levels. They had level data in {rows} guilds."
    ))
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
    xpd_database::ban_guild(&state.db, guild, ban.duration).await?;
    Ok(format!("Banned guild {guild}"))
}

async fn pardon_guild(state: SlashState, pardon: AdminCommandPardonGuild) -> Result<String, Error> {
    let guild: Id<GuildMarker> = pardon.guild.parse()?;
    xpd_database::pardon_guild(&state.db, guild).await?;
    Ok(format!("Pardoned guild {guild}"))
}
