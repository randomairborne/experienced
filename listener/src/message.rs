use rand::Rng;
use sqlx::{query, PgPool};
use std::sync::Arc;
use twilight_model::{id::Id, gateway::payload::incoming::MessageCreate};

pub async fn save(
    msg: MessageCreate,
    db: PgPool,
    mut redis: redis::aio::Connection,
    http: Arc<twilight_http::Client>,
) {
    if let Some(guild_id) = msg.guild_id {
        let has_sent: bool = redis::cmd("GET")
            .arg(format!("{guild_id}-{}", msg.author.id))
            .query_async(&mut redis)
            .await
            .unwrap_or(false);
        if !msg.author.bot && !has_sent {
            let xp_count = rand::thread_rng().gen_range(15..=25);

            #[allow(clippy::cast_possible_wrap)]
            if let Err(e) = query!(
                "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) ON CONFLICT (id, guild) DO UPDATE SET xp=levels.xp+excluded.xp",
                msg.author.id.get() as i64,
                i64::from(xp_count),
                guild_id.get() as i64
            )
            .execute(&db)
            .await
            {
                eprintln!("SQL insert error: {e:?}");
            };
            if let Err(e) = redis::cmd("SET")
                .arg(format!("{guild_id}-{}", msg.author.id))
                .arg(true)
                .arg("EX")
                .arg(60)
                .query_async::<redis::aio::Connection, ()>(&mut redis)
                .await
            {
                eprintln!("Redis error: {e}");
                return;
            };
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            let xp = match query!(
                "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
                msg.author.id.get() as i64,
                guild_id.get() as i64
            )
            .fetch_one(&db)
            .await
            {
                Ok(xp) => xp,
                Err(e) => {
                    eprintln!("SQL select error: {e:?}");
                    return;
                }
            }
            .xp as u64;
            let level_info = mee6::LevelInfo::new(xp);
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            let reward = match query!("SELECT id FROM role_rewards WHERE guild = $1 AND requirement <= $2 ORDER BY requirement DESC LIMIT 1", guild_id.get() as i64, level_info.level() as i64)
                .fetch_one(&db)
                .await
            {
                Ok(rw) => rw.id as u64,
                Err(e) => {
                    if matches!(e, sqlx::Error::RowNotFound) {return;}
                    eprintln!("SQL select error: {e:?}");
                    return;
                }
            };
            http.add_guild_member_role(guild_id, msg.author.id, Id::new(reward))
                .await
                .ok();
        }
    }
}
