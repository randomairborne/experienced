use std::sync::Arc;

use csv::{IntoInnerError as CsvIntoInnerError, Writer as CsvWriter};
use serde::Serialize;
use tokio::join;
use twilight_model::{
    http::attachment::Attachment,
    id::{marker::GuildMarker, Id},
    user::User,
};
use xpd_common::{db_to_id, id_to_db};

use crate::{
    cmd_defs::{gdpr::GdprCommandDelete, GdprCommand},
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_gdpr(
    state: SlashState,
    cmd: GdprCommand,
    invoker: User,
) -> Result<XpdSlashResponse, Error> {
    match cmd {
        GdprCommand::Delete(data) => delete(state, data, invoker).await,
        GdprCommand::Download(_) => download(state, invoker).await,
    }
}

pub async fn delete(
    state: SlashState,
    cmd: GdprCommandDelete,
    invoker: User,
) -> Result<XpdSlashResponse, Error> {
    if cmd.username != invoker.name {
        return Ok(XpdSlashResponse::with_embed_text(
            "Please make sure the username you entered is correct!",
        ));
    }
    let levels =
        query!("DELETE FROM levels WHERE id = $1", id_to_db(invoker.id)).execute(&state.db);
    let custom_card = query!(
        "DELETE FROM custom_card WHERE id = $1",
        id_to_db(invoker.id)
    )
    .execute(&state.db);
    let (levels, custom_card) = join!(levels, custom_card);
    levels?;
    custom_card?;
    Ok(XpdSlashResponse::with_embed_text(
        "All data wiped. Thank you for using experienced.",
    ))
}

pub async fn download(state: SlashState, invoker: User) -> Result<XpdSlashResponse, Error> {
    let invoker = Arc::new(invoker);
    let levels = query!(
        "SELECT guild, xp FROM levels WHERE id = $1",
        id_to_db(invoker.id)
    )
    .fetch_all(&state.db);
    let (levels, custom_card) = join!(
        levels,
        crate::levels::get_customizations(state.clone(), invoker.clone())
    );

    let levels: Vec<UserXpArchiveEntry> = levels?
        .into_iter()
        .map(|v| UserXpArchiveEntry::from_record(v.guild, v.xp))
        .collect();

    let levels = multicsv(&levels)?;
    let custom_card = unicsv(custom_card?)?;

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
        .attachments(&[level_file, card_file])?
        .await?;

    Ok(XpdSlashResponse::with_embed_text(
        "Check your DMs, your data package has been sent to you!",
    ))
}

#[derive(Serialize)]
struct UserXpArchiveEntry {
    guild: Id<GuildMarker>,
    xp: i64,
}

impl UserXpArchiveEntry {
    const fn from_record(guild: i64, xp: i64) -> Self {
        Self {
            guild: db_to_id(guild),
            xp,
        }
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
