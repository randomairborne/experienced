use std::sync::Arc;

use csv::{IntoInnerError as CsvIntoInnerError, Writer as CsvWriter};
use serde::Serialize;
use twilight_model::{
    http::attachment::Attachment,
    id::{marker::GuildMarker, Id},
};
use xpd_common::MemberDisplayInfo;

use crate::{
    cmd_defs::gdpr::{GdprCommand, GdprCommandDelete},
    levels::get_customizations,
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_gdpr(
    state: SlashState,
    cmd: GdprCommand,
    invoker: MemberDisplayInfo,
) -> Result<XpdSlashResponse, Error> {
    match cmd {
        GdprCommand::Delete(data) => delete(state, data, invoker).await,
        GdprCommand::Download(_) => download(state, invoker).await,
    }
}

async fn delete(
    state: SlashState,
    cmd: GdprCommandDelete,
    invoker: MemberDisplayInfo,
) -> Result<XpdSlashResponse, Error> {
    if cmd.user == invoker.id {
        let mut txn = state.db.begin().await?;
        xpd_database::delete_levels_user(&mut txn, invoker.id).await?;
        xpd_database::delete_card_customizations(&mut txn, invoker.id.cast()).await?;
        txn.commit().await?;
        Ok(
            XpdSlashResponse::with_embed_text("All data wiped. Thank you for using experienced.")
                .ephemeral(true),
        )
    } else {
        Ok(XpdSlashResponse::with_embed_text(
            "Please make sure the username you entered is correct!",
        )
        .ephemeral(true))
    }
}

async fn download(
    state: SlashState,
    invoker: MemberDisplayInfo,
) -> Result<XpdSlashResponse, Error> {
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

    Ok(XpdSlashResponse::new()
        .content("Here you go!".to_string())
        .attachments([level_file, card_file])
        .ephemeral(true))
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
