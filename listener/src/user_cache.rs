use redis::AsyncCommands;
use twilight_model::{guild::Member, user::User};

use crate::Error;

pub async fn set_chunk(
    redis: &mut redis::aio::Connection,
    chunk: Vec<Member>,
) -> Result<(), Error> {
    let mut user_pairs: Vec<(u64, String)> = Vec::with_capacity(chunk.len());
    for member in chunk {
        user_pairs.push((member.user.id.get(), serde_json::to_string(&member.user)?));
    }
    Ok(redis
        .set_multiple::<u64, String, ()>(user_pairs.as_slice())
        .await?)
}

pub async fn set_user(redis: &mut redis::aio::Connection, user: User) -> Result<(), Error> {
    Ok(redis
        .set::<u64, String, ()>(user.id.get(), serde_json::to_string(&user)?)
        .await?)
}
