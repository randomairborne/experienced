use std::time::Duration;

use rand::Rng;
use sqlx::query;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    id::{marker::GuildMarker, Id},
};
use xpd_common::{db_to_id, id_to_db};

const MESSAGE_COOLDOWN: Duration = Duration::from_secs(60);

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
        if msg.author.bot {
            return Ok(());
        }

        let user_cooldown_key = (guild_id, msg.author.id);
        let has_sent = self.messages.read()?.contains(&user_cooldown_key);
        if !has_sent {
            let xp_count: i64 = rand::thread_rng().gen_range(15..=25);
            let xp_record = query!(
                "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3)
                    ON CONFLICT (id, guild)
                    DO UPDATE SET xp=levels.xp+excluded.xp
                    RETURNING xp",
                id_to_db(msg.author.id),
                xp_count,
                id_to_db(guild_id)
            )
            .fetch_one(&self.db)
            .await?;
            let xp = u64::try_from(xp_record.xp).unwrap_or(0);
            self.messages
                .write()?
                .insert(user_cooldown_key, MESSAGE_COOLDOWN);
            let level_info = mee6::LevelInfo::new(xp);
            let reward = query!(
                "SELECT id FROM role_rewards
                    WHERE guild = $1 AND requirement <= $2
                    ORDER BY requirement DESC LIMIT 1",
                id_to_db(guild_id),
                i64::try_from(level_info.level()).unwrap_or(i64::MAX)
            )
            .fetch_optional(&self.db)
            .await?
            .map(|v| db_to_id(v.id));
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
