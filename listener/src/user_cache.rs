use redis::AsyncCommands;
use twilight_model::gateway::payload::incoming::MemberChunk;

use crate::Error;

pub async fn set_chunk(
    redis: &mut redis::aio::Connection,
    member_chunk: MemberChunk,
) -> Result<(), Error> {
    Ok(redis
        .set_multiple::<u64, String, ()>(
            member_chunk
                .members
                .iter()
                .map(|v| (v.user.id.get(), serde_json::to_string(&v.user).unwrap()))
                .collect::<Vec<(u64, String)>>()
                .as_slice(),
        )
        .await?)
}
