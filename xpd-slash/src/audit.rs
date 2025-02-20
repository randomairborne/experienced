use twilight_model::{
    http::{attachment::Attachment, interaction::InteractionResponseType},
    id::{Id, marker::GuildMarker},
};
use xpd_slash_defs::audit::AuditLogCommand;

use crate::{Error, SlashState, XpdInteractionData, response::XpdInteractionResponse};

pub async fn process_audit_logs(
    command: AuditLogCommand,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    let mut logs =
        xpd_database::get_audit_log_events(&state.db, guild_id, command.user, command.moderator)
            .await?;
    logs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    let mut file = Vec::with_capacity(logs.len() * 128);
    {
        let mut csv_writer = csv::Writer::from_writer(&mut file);
        for log in logs {
            csv_writer.serialize(log)?;
        }
        csv_writer.flush()?;
    }

    if file.is_empty() {
        Ok(XpdInteractionData::new()
            .content("No moderator actions recorded yet, audit log is empty.".to_string())
            .ephemeral(true)
            .into_interaction_response(InteractionResponseType::ChannelMessageWithSource))
    } else {
        let attachment = Attachment {
            description: Some("Audit logs this server".to_owned()),
            file,
            filename: "audit_log.txt".to_string(),
            id: 0,
        };
        Ok(XpdInteractionData::new()
            .attachments([attachment])
            .ephemeral(true)
            .into_interaction_response(InteractionResponseType::ChannelMessageWithSource))
    }
}
