use twilight_model::id::{marker::{GuildMarker, UserMarker}, Id};

use crate::{Error, AppState};

pub async fn do_fetches(state: &AppState) {
    state.import_queue.mee6.lock()
}

async fn fetch(guild_id: Id<GuildMarker>, state: &AppState) -> Result<(), Error> {
    let mee6_users: Vec<Mee6User> = state
        .http
        .get(format!(
            "https://mee6.xyz/api/plugins/levels/leaderboard/{guild_id}?limit=1000"
        ))
        .send()
        .await?
        .json()
        .await?;
    let user_count = mee6_users.len();
    let mut csv_writer = csv::Writer::from_writer(Vec::new());
    for user in mee6_users {
        #[allow(clippy::cast_possible_wrap)]
        let xp_user = XpUserGuildLevel {
            id: user.id.get() as i64,
            guild: guild_id.get() as i64,
            xp: user.xp,
        };
        csv_writer.serialize(xp_user)?;
    }
    let csv = csv_writer.into_inner().map_err(|_| Error::CsvIntoInner)?;
    let mut copier = state
        .db
        .copy_in_raw("COPY levels FROM STDIN WITH (FORMAT csv)")
        .await?;
    copier.send(csv).await?;
    copier.finish().await?;
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Copy, Clone)]
struct Mee6User {
    pub id: Id<UserMarker>,
    pub level: i64,
    pub xp: i64,
}

#[derive(serde::Deserialize, serde::Serialize, Copy, Clone)]
struct XpUserGuildLevel {
    pub id: i64,
    pub guild: i64,
    pub xp: i64,
}