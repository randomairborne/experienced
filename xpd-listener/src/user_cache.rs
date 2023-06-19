use redis::AsyncCommands;
use twilight_model::{guild::Member, user::User};

use crate::Error;
impl crate::XpdListener {
    pub async fn set_chunk(&self, chunk: Vec<Member>) -> Result<(), Error> {
        let mut user_pairs: Vec<(String, String)> = Vec::with_capacity(chunk.len());
        for member in chunk {
            let discriminator = if member.user.discriminator == 0 {
                None
            } else {
                Some(member.user.discriminator().to_string())
            };
            let user = xpd_common::RedisUser {
                id: member.user.id.into(),
                username: Some(member.user.name),
                discriminator,
            };
            user_pairs.push((
                format!("cache-user-{}", member.user.id.get()),
                serde_json::to_string(&user)?,
            ));
        }
        self.redis
            .get()
            .await?
            .mset::<String, String, ()>(user_pairs.as_slice())
            .await?;
        Ok(())
    }

    pub async fn set_user(&self, user: &User) -> Result<(), Error> {
        self.redis
            .get()
            .await?
            .set::<String, String, ()>(
                format!("cache-user-{}", user.id.get()),
                serde_json::to_string(user)?,
            )
            .await?;
        Ok(())
    }
}
