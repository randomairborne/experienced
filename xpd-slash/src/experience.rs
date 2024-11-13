use twilight_model::{
    channel::message::AllowedMentions,
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_slash_defs::experience::XpCommand;

use crate::{Error, SlashState, XpdSlashResponse};

pub async fn process_xp(
    data: XpCommand,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = process_experience(data, guild_id, state).await?;
    Ok(XpdSlashResponse::new()
        .allowed_mentions_o(Some(AllowedMentions::default()))
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn process_experience(
    data: XpCommand,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<String, Error> {
    if !allowed_command_for_target(&data) {
        return Err(Error::BotsDontLevel);
    }
    match data {
        XpCommand::Add(add) => {
            modify_user_xp(state, guild_id, add.user.resolved.id, add.amount).await
        }
        XpCommand::Remove(rm) => {
            modify_user_xp(state, guild_id, rm.user.resolved.id, -rm.amount).await
        }
        XpCommand::Reset(reset) => reset_user_xp(state, guild_id, reset.user.resolved.id).await,
        XpCommand::Set(set) => set_user_xp(state, guild_id, set.user.resolved.id, set.xp).await,
    }
}

async fn modify_user_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    amount: i64,
) -> Result<String, Error> {
    let mut txn = state.db.begin().await?;
    let xp = xpd_database::add_xp(txn.as_mut(), user_id, guild_id, amount).await?;
    if xp.is_negative() {
        txn.rollback().await?;
        return Err(Error::XpWouldBeNegative);
    }
    txn.commit().await?;
    let current_level = mee6::LevelInfo::new(xp.try_into().unwrap_or(0)).level();
    let (action, targeter) = if amount.is_positive() {
        ("Added", "to")
    } else {
        ("Removed", "from")
    };
    let amount_abs = amount.abs();
    Ok(format!("{action} {amount_abs} XP {targeter} <@{user_id}>, leaving them with {xp} XP at level {current_level}"))
}

async fn reset_user_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
) -> Result<String, Error> {
    xpd_database::delete_levels_user_guild(&state.db, user_id, guild_id).await?;
    Ok(format!(
        "Deleted <@{user_id}> from my database in this server!"
    ))
}

async fn set_user_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    setpoint: i64,
) -> Result<String, Error> {
    xpd_database::set_xp(&state.db, user_id, guild_id, setpoint).await?;
    let level = mee6::LevelInfo::new(setpoint.try_into().unwrap_or(0));
    Ok(format!(
        "Set <@{user_id}>'s XP to {}, leaving them at level {}",
        level.xp(),
        level.level()
    ))
}

/// For commands that target a specific user, other than reset, prevent commands from being used on a bot.
const fn allowed_command_for_target(data: &XpCommand) -> bool {
    match data {
        XpCommand::Add(add) => !add.user.resolved.bot,
        XpCommand::Remove(rm) => !rm.user.resolved.bot,
        XpCommand::Set(set) => !set.user.resolved.bot,
        XpCommand::Reset(_) => true,
    }
}
