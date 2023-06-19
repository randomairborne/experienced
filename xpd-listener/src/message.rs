use rand::Rng;
use redis::AsyncCommands;
use sqlx::query;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    id::{
        marker::{GuildMarker, RoleMarker},
        Id,
    },
};

use crate::{Error, XpdListener};
impl XpdListener {
    pub async fn save(&self, msg: MessageCreate) -> Result<(), Error> {
        if let Some(guild_id) = msg.guild_id {
            self.save_msg_send(guild_id, msg).await?;
        }
        Ok(())
    }
    async fn save_msg_send(
        &self,
        guild_id: Id<GuildMarker>,
        msg: MessageCreate,
    ) -> Result<(), Error> {
        let has_sent_key = format!("cooldown-{guild_id}-{}", msg.author.id);
        let has_sent: bool = self.redis.get().await?.get(&has_sent_key).await?;
        if !msg.author.bot && !has_sent {
            let xp_count: i64 = rand::thread_rng().gen_range(15..=25);
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            let xp = query!(
                "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) ON CONFLICT (id, guild) DO UPDATE SET xp=levels.xp+excluded.xp RETURNING xp",
                msg.author.id.get() as i64,
                xp_count,
                guild_id.get() as i64
            )
            .fetch_one(&self.db)
            .await?.xp as u64;
            self.redis
                .get()
                .await?
                .set_ex(&has_sent_key, true, 60)
                .await?;
            let level_info = mee6::LevelInfo::new(xp);
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            let reward = query!(
                "SELECT id FROM role_rewards
                    WHERE guild = $1 AND requirement <= $2
                    ORDER BY requirement DESC LIMIT 1",
                guild_id.get() as i64,
                level_info.level() as i64
            )
            .fetch_optional(&self.db)
            .await?
            .map(|v| Id::<RoleMarker>::new(v.id as u64));
            if let Some(reward) = reward {
                if let Some(member) = &msg.member {
                    if member.roles.contains(&reward) {
                        return Ok(());
                    }
                }
                self.http
                    .add_guild_member_role(guild_id, msg.author.id, reward)
                    .await?;
            }
        }
        Ok(())
    }
}
