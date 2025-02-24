use std::sync::Arc;

use csv::{IntoInnerError as CsvIntoInnerError, Writer as CsvWriter};
use serde::Serialize;
use twilight_model::{
    http::{attachment::Attachment, interaction::InteractionResponseType},
    id::{Id, marker::GuildMarker},
};
use xpd_common::MemberDisplayInfo;
use xpd_database::AcquireWrapper as _;
use xpd_slash_defs::gdpr::{GdprCommand, GdprCommandDelete};

use crate::{
    Error, SlashState, XpdInteractionData, levels::get_customizations,
    response::XpdInteractionResponse,
};

pub async fn process_gdpr(
    state: SlashState,
    cmd: GdprCommand,
    invoker: MemberDisplayInfo,
) -> Result<XpdInteractionResponse, Error> {
    match cmd {
        GdprCommand::Delete(data) => delete(state, data, invoker).await,
        GdprCommand::Download(_) => download(state, invoker).await,
    }
}

async fn delete(
    state: SlashState,
    cmd: GdprCommandDelete,
    invoker: MemberDisplayInfo,
) -> Result<XpdInteractionResponse, Error> {
    if cmd.user == invoker.id {
        let mut txn = state.db.xbegin().await?;
        xpd_database::delete_levels_user(&mut txn, invoker.id).await?;
        xpd_database::delete_card_customizations(&mut txn, invoker.id.cast()).await?;
        xpd_database::delete_audit_log_events_user(&mut txn, invoker.id).await?;
        txn.commit().await?;
        Ok(
            XpdInteractionData::with_embed_text("All data wiped. Thank you for using experienced.")
                .ephemeral(true)
                .into_interaction_response(InteractionResponseType::ChannelMessageWithSource),
        )
    } else {
        Ok(XpdInteractionData::with_embed_text(
            "Please make sure the username you entered is correct!",
        )
        .ephemeral(true)
        .into_interaction_response(InteractionResponseType::ChannelMessageWithSource))
    }
}

async fn download(
    state: SlashState,
    invoker: MemberDisplayInfo,
) -> Result<XpdInteractionResponse, Error> {
    let invoker = Arc::new(invoker);
    let levels = xpd_database::get_all_levels(&state.db, invoker.id).await?;

    let invoker_id = &[invoker.id.cast()];
    let custom_card = get_customizations(&state, invoker_id).await?;

    let levels: Vec<UserXpArchiveEntry> = levels
        .into_iter()
        .map(|v| UserXpArchiveEntry::from_record(v.guild, v.xp))
        .collect();

    let levels = multicsv(&levels)?;
    let custom_card = multicsv(&[custom_card])?;

    let level_file = Attachment::from_bytes(format!("leveling-{}.csv", invoker.id), levels, 1);
    let card_file = Attachment::from_bytes(format!("card-{}.csv", invoker.id), custom_card, 2);

    let attachments: Vec<Attachment> = [level_file, card_file]
        .into_iter()
        .filter(|v| !v.file.is_empty())
        .collect();

    Ok(XpdInteractionData::new()
        .content("Here you go!".to_string())
        .attachments(attachments)
        .ephemeral(true)
        .into_interaction_response(InteractionResponseType::DeferredChannelMessageWithSource))
}

#[derive(Serialize)]
struct UserXpArchiveEntry {
    guild: Id<GuildMarker>,
    xp: i64,
}

impl UserXpArchiveEntry {
    const fn from_record(guild: Id<GuildMarker>, xp: i64) -> Self {
        Self { guild, xp }
    }
}

fn multicsv<T: Serialize>(data: &[T]) -> Result<Vec<u8>, Error> {
    let mut data_wtr = CsvWriter::from_writer(Vec::new());
    for datum in data {
        data_wtr.serialize(datum)?;
    }
    Ok(data_wtr
        .into_inner()
        .map_err(CsvIntoInnerError::into_error)?)
}
