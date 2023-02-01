use rand::Rng;
use redis::AsyncCommands;
use sqlx::{query, PgPool};
use std::sync::Arc;
use twilight_model::{gateway::payload::incoming::MessageCreate, id::Id};

pub async fn save(
    msg: MessageCreate,
    db: PgPool,
    mut redis: redis::aio::ConnectionManager,
    http: Arc<twilight_http::Client>,
) -> Result<(), crate::Error> {
    if let Some(guild_id) = msg.guild_id {
        let has_sent_key = format!("cooldown-{guild_id}-{}", msg.author.id);
        let has_sent: bool = redis.get(&has_sent_key)
            .await?;
        if !msg.author.bot && !has_sent {
            let xp_count = rand::thread_rng().gen_range(15..=25);
            #[allow(clippy::cast_possible_wrap)]
            query!(
                "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) ON CONFLICT (id, guild) DO UPDATE SET xp=levels.xp+excluded.xp",
                msg.author.id.get() as i64,
                i64::from(xp_count),
                guild_id.get() as i64
            )
            .execute(&db)
            .await?;
            redis.set_ex(&has_sent_key, true, 60)
                .await?;
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            let xp = query!(
                "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
                msg.author.id.get() as i64,
                guild_id.get() as i64
            )
            .fetch_one(&db)
            .await?
            .xp as u64;
            let level_info = mee6::LevelInfo::new(xp);
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            let reward = query!("SELECT id FROM role_rewards WHERE guild = $1 AND requirement <= $2 ORDER BY requirement DESC LIMIT 1", guild_id.get() as i64, level_info.level() as i64)
                .fetch_optional(&db)
                .await?;
            if let Some(reward) = reward {
                #[allow(clippy::cast_sign_loss)]
                let id = reward.id as u64;
                http.add_guild_member_role(guild_id, msg.author.id, Id::new(id))
                    .await?;
            }
        }
    }

    Ok(())
}
