use twilight_model::{
    channel::message::AllowedMentions,
    http::interaction::InteractionResponseType,
    id::{
        Id,
        marker::{GuildMarker, InteractionMarker, UserMarker},
    },
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::AuditLogEvent;
use xpd_database::AcquireWrapper as _;
use xpd_slash_defs::experience::XpCommand;
use xpd_util::snowflake_to_timestamp;

use crate::{Error, SlashState, XpdInteractionData, response::XpdInteractionResponse};

pub struct XpAuditData {
    pub interaction: Id<InteractionMarker>,
    pub invoker: Id<UserMarker>,
}

pub async fn process_xp(
    data: XpCommand,
    state: SlashState,
    guild_id: Id<GuildMarker>,
    audit: XpAuditData,
) -> Result<XpdInteractionResponse, Error> {
    let contents = process_experience(data, guild_id, state, audit).await?;
    Ok(XpdInteractionData::new()
        .allowed_mentions_o(Some(AllowedMentions::default()))
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()])
        .into_interaction_response(InteractionResponseType::ChannelMessageWithSource))
}

async fn process_experience(
    data: XpCommand,
    guild_id: Id<GuildMarker>,
    state: SlashState,
    audit: XpAuditData,
) -> Result<String, Error> {
    if !allowed_command_for_target(&data) {
        return Err(Error::BotsDontLevel);
    }
    match data {
        XpCommand::Add(add) => {
            modify_user_xp(state, guild_id, add.user.resolved.id, add.amount, audit).await
        }
        XpCommand::Remove(rm) => {
            modify_user_xp(state, guild_id, rm.user.resolved.id, -rm.amount, audit).await
        }
        XpCommand::Reset(reset) => {
            reset_user_xp(state, guild_id, reset.user.resolved.id, audit).await
        }
        XpCommand::Set(set) => {
            set_user_xp(state, guild_id, set.user.resolved.id, set.xp, audit).await
        }
    }
}

async fn modify_user_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    amount: i64,
    audit: XpAuditData,
) -> Result<String, Error> {
    let mut txn = state.db.xbegin().await?;
    let xp = xpd_database::add_xp(txn.as_mut(), user_id, guild_id, amount).await?;
    if xp.is_negative() {
        txn.rollback().await?;
        return Err(Error::XpWouldBeNegative);
    }
    let audit_event = AuditLogEvent {
        guild_id,
        user_id,
        moderator: audit.invoker,
        timestamp: snowflake_to_timestamp(audit.interaction),
        previous: xp + amount,
        delta: amount,
        reset: false,
        set: false,
    };
    xpd_database::add_audit_log_event(txn.as_mut(), audit_event).await?;

    txn.commit().await?;
    let current_level = mee6::LevelInfo::new(xp.try_into().unwrap_or(0)).level();
    let (action, targeter) = if amount.is_positive() {
        ("Added", "to")
    } else {
        ("Removed", "from")
    };
    let amount_abs = amount.abs();
    Ok(format!(
        "{action} {amount_abs} XP {targeter} <@{user_id}>, leaving them with {xp} XP at level {current_level}"
    ))
}

async fn reset_user_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    audit: XpAuditData,
) -> Result<String, Error> {
    let mut txn = state.db.xbegin().await?;
    let old_xp = xpd_database::delete_levels_user_guild(txn.as_mut(), user_id, guild_id).await?;

    let audit_event = AuditLogEvent {
        guild_id,
        user_id,
        moderator: audit.invoker,
        timestamp: snowflake_to_timestamp(audit.interaction),
        previous: old_xp,
        delta: -old_xp,
        reset: true,
        set: false,
    };
    xpd_database::add_audit_log_event(txn.as_mut(), audit_event).await?;

    txn.commit().await?;

    Ok(format!(
        "Deleted <@{user_id}> from my database in this server!"
    ))
}

async fn set_user_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    setpoint: i64,
    audit: XpAuditData,
) -> Result<String, Error> {
    let mut txn = state.db.xbegin().await?;
    let old_xp = xpd_database::user_xp(txn.as_mut(), guild_id, user_id)
        .await?
        .unwrap_or(0);
    xpd_database::set_xp(txn.as_mut(), user_id, guild_id, setpoint).await?;

    let audit_event = AuditLogEvent {
        guild_id,
        user_id,
        moderator: audit.invoker,
        timestamp: snowflake_to_timestamp(audit.interaction),
        previous: old_xp,
        delta: setpoint - old_xp,
        reset: false,
        set: true,
    };
    xpd_database::add_audit_log_event(txn.as_mut(), audit_event).await?;

    txn.commit().await?;

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
