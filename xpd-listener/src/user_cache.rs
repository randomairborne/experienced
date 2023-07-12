use redis::AsyncCommands;
use twilight_model::{
    guild::{Guild, Member},
    user::User,
};

use crate::Error;
impl crate::XpdListener {
    pub async fn set_chunk(&self, chunk: Vec<Member>) -> Result<(), Error> {
        let mut user_pairs: Vec<(String, String)> = Vec::with_capacity(chunk.len());
        for member in chunk {
            let discriminator = if member.user.discriminator == 0 {
                None
            } else {
                Some(member.user.discriminator)
            };
            let user = xpd_common::RedisUser {
                id: member.user.id,
                username: Some(member.user.name),
                discriminator,
                avatar_hash: member.user.avatar,
                banner_hash: member.user.banner,
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

    pub async fn set_guild(&self, guild: Guild) -> Result<(), Error> {
        let interop_guild = xpd_common::RedisGuild {
            id: guild.id,
            name: guild.name,
            banner_hash: guild.banner,
            icon_hash: guild.icon,
        };
        self.set_chunk(guild.members).await?;
        self.redis
            .get()
            .await?
            .set(
                format!("cache-guild-{}", guild.id),
                serde_json::to_string(&interop_guild)?,
            )
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
