use twilight_model::{
    gateway::payload::incoming::GuildAuditLogEntryCreate,
    guild::audit_log::AuditLogEventType,
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};
use xpd_database::PgPool;

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
        take_audit_log_action(&db, ev).await?;
    };
    Ok(())
}

async fn take_audit_log_action(db: &PgPool, event: DiscordAuditLogClearEvent) -> Result<(), Error> {
    let txn = db.begin().await.map_err(xpd_database::Error::from)?;
    
    txn.commit().await.map_err(xpd_database::Error::from)?;
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
