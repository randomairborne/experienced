use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use redis::AsyncCommands;
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use xpd_common::{id_to_db, RedisGuild, RedisUser};

use crate::{
    error::{Error, HttpError},
    AppState,
};

#[derive(serde::Serialize, Debug)]
pub struct User {
    id: u64,
    level: u64,
    name: Option<String>,
    discriminator: Option<u16>,
}

const PAGE_SIZE: i64 = 50;

#[derive(serde::Deserialize)]
pub struct FetchQuery {
    #[serde(default = "get_0")]
    page: i64,
}

const fn get_0() -> i64 {
    0
}

pub async fn fetch_stats(
    weird_guild_id: Option<Path<Id<GuildMarker>>>,
    State(state): State<AppState>,
    Query(query): Query<FetchQuery>,
) -> Result<Html<String>, HttpError> {
    let Some(Path(guild_id)) = weird_guild_id else {
        return Err(HttpError::new(Error::NoLeveling, state));
    };
    let offset = query.page * PAGE_SIZE;
    let guild = get_redis_guild(&state, guild_id)
        .await
        .map_err(|e| HttpError::new(e, state.clone()))?;

    let user_rows = sqlx::query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT $2 OFFSET $3",
        id_to_db(guild_id),
        PAGE_SIZE + 1,
        offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| HttpError::new(e.into(), state.clone()))?;
    if user_rows.is_empty() {
        return Err(HttpError::new(Error::NoLeveling, state));
    }
    #[allow(clippy::cast_possible_truncation)]
    let has_next_page = user_rows.len() > PAGE_SIZE as usize;
    let mut ids_to_indices: HashMap<Id<UserMarker>, usize> =
        HashMap::with_capacity(user_rows.len());
    let mut users: Vec<User> = user_rows
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            ids_to_indices.insert(Id::new(u64::try_from(v.id).unwrap_or(0)), i);
            #[allow(clippy::cast_sign_loss)]
            let id = v.id as u64;
            User {
                id,
                level: mee6::LevelInfo::new(u64::try_from(v.xp).unwrap_or(0)).level(),
                name: None,
                discriminator: None,
            }
        })
        .collect();
    // if we have 51 users, the 51st user is the first user on the next page
    if has_next_page {
        users.pop();
    }
    let user_strings: Vec<Option<String>> = if users.is_empty() {
        Vec::new()
    } else {
        state
            .redis
            .get()
            .await
            .map_err(|e| HttpError::new(e.into(), state.clone()))?
            .mget(
                users
                    .iter()
                    .map(|v| format!("cache-user-{}", v.id))
                    .collect::<Vec<String>>(),
            )
            .await
            .map_err(|e| HttpError::new(e.into(), state.clone()))?
    };
    for user_string in user_strings.into_iter().flatten() {
        let user: RedisUser = match serde_json::from_str(&user_string) {
            Ok(v) => v,
            Err(source) => {
                error!(?source, "Failed to deserialize user from redis");
                continue;
            }
        };
        if let Some(i) = ids_to_indices.get(&user.id) {
            users[*i].discriminator = user.discriminator;
            users[*i].name = user.username;
        }
    }
    let mut context = tera::Context::new();
    context.insert("users", &users);
    context.insert("offset", &offset);
    context.insert("page", &query.page);
    context.insert("guild", &guild);
    context.insert("root_url", &state.root_url);
    context.insert("has_next_page", &has_next_page);
    let rendered = state
        .tera
        .render("leaderboard.html", &context)
        .map_err(|e| HttpError::new(e.into(), state))?;
    Ok(Html(rendered))
}

async fn get_redis_guild(state: &AppState, guild: Id<GuildMarker>) -> Result<RedisGuild, Error> {
    let maybe_guild_string: Option<String> = state
        .redis
        .get()
        .await?
        .get(format!("cache-guild-{guild}"))
        .await?;
    Ok(if let Some(guild_string) = maybe_guild_string {
        serde_json::from_str(&guild_string)?
    } else {
        RedisGuild {
            id: guild,
            name: "(name not in cache)".to_string(),
            banner_hash: None,
            icon_hash: None,
        }
    })
}
