use twilight_model::{
    gateway::payload::incoming::GuildAuditLogEntryCreate,
    guild::audit_log::AuditLogEventType,
    id::{
        Id,
        marker::{AuditLogEntryMarker, GuildMarker, UserMarker},
    },
};
use xpd_common::{AuditLogEvent, AuditLogEventKind};
use xpd_database::{AcquireWrapper as _, PgPool};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DiscordAuditLogClearEvent {
    moderator: Id<UserMarker>,
    target: Id<UserMarker>,
    guild: Id<GuildMarker>,
    kind: DiscordAuditLogClearType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DiscordAuditLogClearType {
    Kick,
    Ban,
}

impl DiscordAuditLogClearType {
    fn from_audit_log_type(ty: AuditLogEventType) -> Option<Self> {
        match ty {
            AuditLogEventType::MemberKick => Some(Self::Kick),
            AuditLogEventType::MemberBanAdd => Some(Self::Ban),
            _ => None,
        }
    }
}

pub async fn audit_log(db: &PgPool, audit_log: GuildAuditLogEntryCreate) -> Result<(), Error> {
    if let Some(kind) = DiscordAuditLogClearType::from_audit_log_type(audit_log.action_type) {
        let Some(guild) = audit_log.guild_id else {
            return Err(AuditLogError::GuildMissing.into());
        };
        let Some(target) = audit_log.target_id else {
            return Err(AuditLogError::TargetMissing.into());
        };
        let Some(moderator) = audit_log.user_id else {
            return Err(AuditLogError::ModeratorMissing.into());
        };
        let ev = DiscordAuditLogClearEvent {
            kind,
            guild,
            target: target.cast(),
            moderator,
        };
        take_audit_log_action(db, audit_log.id, ev).await?;
    };
    Ok(())
}

async fn take_audit_log_action(
    db: &PgPool,
    id: Id<AuditLogEntryMarker>,
    event: DiscordAuditLogClearEvent,
) -> Result<(), Error> {
    let mut txn = db.xbegin().await?;
    let old_xp = xpd_database::user_xp(txn.as_mut(), event.guild, event.target)
        .await?
        .unwrap_or(0);

    let kind = match event.kind {
        DiscordAuditLogClearType::Kick => AuditLogEventKind::KickReset,
        DiscordAuditLogClearType::Ban => AuditLogEventKind::BanReset,
    };

    let audit_log_event = AuditLogEvent {
        guild: event.guild,
        target: event.target,
        moderator: event.moderator,
        timestamp: xpd_util::snowflake_to_timestamp(id),
        previous: old_xp,
        delta: -old_xp,
        kind,
    };

    xpd_database::add_audit_log_event(txn.as_mut(), audit_log_event).await?;
    xpd_database::delete_levels_user_guild(txn.as_mut(), event.target, event.guild).await?;
    txn.commit().await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum AuditLogError {
    #[error("Discord did not send a guild for the audit log event!")]
    GuildMissing,
    #[error("Discord did not send a target for the audit log event!")]
    TargetMissing,
    #[error("Discord did not send a moderator for the audit log event!")]
    ModeratorMissing,
    #[error("Discord did not send a moderator for the audit log event!")]
    UnusedKind,
}
