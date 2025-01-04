use http_body_util::{BodyExt, Limited};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use twilight_model::{
    channel::{message::AllowedMentions, Attachment},
    http::attachment::Attachment as HttpAttachment,
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_slash_defs::manage::{ManageCommand, CONFIRMATION_STRING};

use crate::{dispatch::Respondable, Error, SlashState, XpdSlashResponse};

pub async fn process_manage(
    data: ManageCommand,
    guild_id: Id<GuildMarker>,
    respondable: Respondable,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = match data {
        ManageCommand::ResetGuild(rg) => {
            reset_guild_xp(state, guild_id, rg.confirm_message).await?
        }
        ManageCommand::Import(import) => import_level_data(
            state,
            respondable,
            guild_id,
            import.levels,
            import.overwrite.unwrap_or(false),
        )?,
        ManageCommand::Export(_) => export_level_data(state, respondable, guild_id)?,
    };
    Ok(XpdSlashResponse::new()
        .allowed_mentions(AllowedMentions::default())
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

#[derive(Deserialize, Serialize)]
pub struct ImportUser {
    id: Id<UserMarker>,
    xp: i64,
}

#[allow(clippy::unnecessary_wraps)]
fn export_level_data(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    state
        .task_tracker
        .clone()
        .spawn(background_data_operation_wrapper(
            state,
            respondable,
            guild_id,
            None,
            false,
        ));
    Ok("Exporting level data, check back soon!".to_string())
}

async fn background_data_export(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    let levels: Vec<ImportUser> = xpd_database::export_bulk_users(&state.db, guild_id)
        .await?
        .iter()
        .map(|us| ImportUser {
            id: us.id,
            xp: us.xp,
        })
        .collect();
    let file = serde_json::to_vec_pretty(&levels)?;
    let attachment = HttpAttachment::from_bytes(format!("export-{guild_id}.json"), file, 0);
    Ok(XpdSlashResponse::new()
        .content("Exported your level data!".to_string())
        .attachments([attachment]))
}

#[allow(clippy::unnecessary_wraps)]
fn import_level_data(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    attachment: Attachment,
    overwrite: bool,
) -> Result<String, Error> {
    state.clone().spawn(background_data_operation_wrapper(
        state,
        respondable,
        guild_id,
        Some(attachment),
        overwrite,
    ));
    Ok("Importing level data, check back soon!".to_string())
}

const MAX_IMPORT_SIZE: usize = 1024 * 1024 * 10;

async fn background_data_import(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
    attachment: Attachment,
    overwrite: bool,
) -> Result<XpdSlashResponse, Error> {
    let start = Instant::now();

    let request = state.http.get(attachment.url).send().await?;
    request.error_for_status_ref()?;

    let raw_body = reqwest::Body::from(request);
    let body = Limited::new(raw_body, MAX_IMPORT_SIZE)
        .collect()
        .await
        .map_err(|_| Error::RawHttpBody)?
        .to_bytes();

    let data: Vec<ImportUser> = serde_json::from_slice(&body)?;
    let user_count = data.len();
    let mut txn = state.db.begin().await?;
    for user in data {
        if overwrite {
            xpd_database::set_xp(txn.as_mut(), user.id, guild_id, user.xp).await?;
        } else {
            xpd_database::add_xp(txn.as_mut(), user.id, guild_id, user.xp).await?;
        }
    }

    txn.commit().await?;

    let seconds = start.elapsed().as_secs_f64();
    Ok(XpdSlashResponse::with_embed_text(format!(
        "Imported XP data for {user_count} users in {seconds:.2} seconds!"
    )))
}

async fn background_data_operation_wrapper(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    attachment: Option<Attachment>,
    overwrite: bool,
) {
    let xsr = if let Some(attachment) = attachment {
        background_data_import(&state, guild_id, attachment, overwrite)
            .await
            .unwrap_or_else(|source| {
                error!(?source, "Failed to import level data");
                XpdSlashResponse::with_embed_text(format!("Failed to import level data: {source}"))
            })
    } else {
        background_data_export(&state, guild_id)
            .await
            .unwrap_or_else(|source| {
                error!(?source, "Failed to export level data");
                XpdSlashResponse::with_embed_text(format!("Failed to export level data: {source}"))
            })
    }
    .ephemeral(true);
    state.send_followup(xsr, respondable.token()).await;
}

async fn reset_guild_xp(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    confirmation: String,
) -> Result<String, Error> {
    if confirmation != CONFIRMATION_STRING {
        return Ok("Confirmation string did not match.".to_string());
    }

    let mut txn = state.db.begin().await?;
    xpd_database::delete_levels_guild(&mut txn, guild_id).await?;
    xpd_database::delete_audit_log_events_guild(&mut txn, guild_id).await?;
    txn.commit().await?;

    Ok("Done. Thank you for using Experienced.".to_string())
}
