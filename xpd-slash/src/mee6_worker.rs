use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

use crate::{Error, SlashState};

pub async fn do_fetches(state: SlashState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let Some((guild_id, interaction_token)) = state.import_queue.mee6.lock().pop_front() else { continue; };
        if let Err(e) = get_guild(guild_id, &state).await {
            error!("worker failed to fetch: {e:?}");
            match state
                .client
                .interaction(state.my_id)
                .update_response(&interaction_token)
                .content(Some(&format!(
                    "Worker failed to fetch from mee6 api: {e:?}\nPlease join our support server: https://valk.sh/discord"
                ))) {
                Ok(v) => match v.await {
                    Ok(_m) => {}
                    Err(e) => error!("worker failed to update response: {e:?}"),
                },
                Err(e) => error!("invalid worker message: {e:?}"),
            };
            continue;
        }
        match state
            .client
            .interaction(state.my_id)
            .update_response(&interaction_token)
            .content(Some("Finished updating!"))
        {
            Ok(v) => match v.await {
                Ok(_m) => {}
                Err(e) => error!("worker failed to update response: {e:?}"),
            },
            Err(e) => error!("invalid worker message: {e:?}"),
        };
    }
}

async fn get_guild(guild_id: Id<GuildMarker>, state: &SlashState) -> Result<(), Error> {
    let mut page = 0;
    while fetch(guild_id, page, state).await? {
        page += 1;
    }
    Ok(())
}

async fn fetch(guild_id: Id<GuildMarker>, page: usize, state: &SlashState) -> Result<bool, Error> {
    let mee6_data: Mee6ApiResponse = state
        .http
        .get(format!(
            "https://mee6.xyz/api/plugins/levels/leaderboard/{guild_id}?limit=1000&page={page}"
        ))
        .send()
        .await?
        .json()
        .await?;
    let mee6_users = mee6_data.players;
    let mut trans = state.db.begin().await?;
    for user in &mee6_users {
        #[allow(clippy::cast_possible_wrap)]
        let xp_user = XpUserGuildLevel {
            id: user.id.get() as i64,
            guild: guild_id.get() as i64,
            xp: user.xp,
        };
        sqlx::query!(
            "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3)",
            xp_user.id,
            xp_user.xp,
            xp_user.guild
        )
        .execute(trans.as_mut())
        .await?;
    }
    Ok(mee6_users.is_empty())
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct Mee6ApiResponse {
    pub players: Vec<Mee6User>,
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
