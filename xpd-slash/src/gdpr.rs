use std::sync::Arc;

use csv::{IntoInnerError as CsvIntoInnerError, Writer as CsvWriter};
use serde::Serialize;
use tokio::{join, try_join};
use twilight_model::{
    http::attachment::Attachment,
    id::{marker::GuildMarker, Id},
};
use xpd_common::{MemberDisplayInfo};
use xpd_database::{Database, RawCustomizations};
use xpd_rank_card::customizations::Customizations;

use crate::{
    cmd_defs::{gdpr::GdprCommandDelete, GdprCommand},
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
    if cmd.username != invoker.name {
        Ok(XpdSlashResponse::with_embed_text(
            "Please make sure the username you entered is correct!",
        ))
    } else {
        state.query_clear_all_user_data(invoker.id).await?;
        Ok(
            XpdSlashResponse::with_embed_text("All data wiped. Thank you for using experienced.")
                .ephemeral(true),
        )
    }
}

async fn download(
    state: SlashState,
    invoker: MemberDisplayInfo,
) -> Result<XpdSlashResponse, Error> {
    let invoker = Arc::new(invoker);
    let levels = state.query_get_all_levels(invoker.id).await?;

    let invoker_id = &[invoker.id.cast()];
    let custom_card = get_customizations(&state, invoker_id).await?;

    let levels: Vec<UserXpArchiveEntry> = levels
        .into_iter()
        .map(|v| UserXpArchiveEntry::from_record(v.guild, v.xp))
        .collect();

    let levels = multicsv(&levels)?;
    let custom_card = unicsv(custom_card)?;

    let level_file = Attachment::from_bytes(format!("leveling-{}.csv", invoker.id), levels, 1);
    let card_file = Attachment::from_bytes(format!("card-{}.csv", invoker.id), custom_card, 2);

    let dm_channel = state
        .client
        .create_private_channel(invoker.id)
        .await?
        .model()
        .await?;

    state
        .client
        .create_message(dm_channel.id)
        .attachments(&[level_file, card_file])
        .await?;

    Ok(
        XpdSlashResponse::with_embed_text(
            "Check your DMs, your data package has been sent to you!",
        )
        .ephemeral(true),
    )
}

#[derive(Serialize)]
struct UserXpArchiveEntry {
    guild: Id<GuildMarker>,
    xp: i64,
}

impl UserXpArchiveEntry {
    fn from_record(guild: Id<GuildMarker>, xp: i64) -> Self {
        Self { guild, xp }
    }
}

fn unicsv<T: Serialize>(data: T) -> Result<Vec<u8>, Error> {
    let mut data_wtr = CsvWriter::from_writer(Vec::new());
    data_wtr.serialize(data)?;
    Ok(data_wtr
        .into_inner()
        .map_err(CsvIntoInnerError::into_error)?)
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
