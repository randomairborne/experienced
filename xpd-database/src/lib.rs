#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// we allow AFIT because all crates in experienced are internal
#![allow(
    async_fn_in_trait,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

mod acq_wrapper;
#[cfg(test)]
mod test;
mod util;

use std::{fmt::Display, ops::DerefMut};

pub use acq_wrapper::AcquireWrapper;
use simpleinterpolation::Interpolation;
pub use sqlx::PgPool;
use sqlx::{Acquire, PgConnection, Postgres, query, query_as};
use tokio_stream::StreamExt;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GenericMarker, GuildMarker, RoleMarker, UserMarker},
};
use util::{db_to_id, id_to_db};
use xpd_common::{
    AuditLogEvent, AuditLogEventKind, GuildConfig, RoleReward, UserInGuild, UserStatus,
};
pub async fn guild_rewards<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild_id: Id<GuildMarker>,
) -> Result<Vec<RoleReward>, Error> {
    let mut conn = conn.acquire().await?;
    let rewards: Vec<RoleReward> = query!(
        "SELECT id, requirement FROM role_rewards WHERE guild = $1",
        id_to_db(guild_id),
    )
    .fetch_all(conn.as_mut())
    .await?
    .into_iter()
    .map(|row| RoleReward {
        id: db_to_id(row.id),
        requirement: row.requirement,
    })
    .collect();
    Ok(rewards)
}

pub async fn guild_config<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<Option<GuildConfig>, Error> {
    let mut conn = conn.acquire().await?;
    let config = query_as!(
        RawGuildConfig,
        "SELECT one_at_a_time, level_up_message, level_up_channel, ping_on_level_up,\
                 max_xp_per_message, min_xp_per_message, message_cooldown, \
                 guild_card_default_show_off \
                 FROM guild_configs WHERE id = $1",
        id_to_db(guild)
    )
    .fetch_optional(conn.as_mut())
    .await?
    .map(RawGuildConfig::cook)
    .transpose()?;
    Ok(config)
}

/// Add (or, when given a negative, subtract) some amount of XP from a user in a guild.
pub async fn add_xp<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    author: Id<UserMarker>,
    guild: Id<GuildMarker>,
    amount: i64,
) -> Result<i64, Error> {
    let mut conn = conn.acquire().await?;
    let count = query!(
        "INSERT INTO levels (id, guild, xp) VALUES ($1, $2, $3) \
                    ON CONFLICT (id, guild) \
                    DO UPDATE SET xp=levels.xp+excluded.xp \
                    RETURNING xp",
        id_to_db(author),
        id_to_db(guild),
        amount
    )
    .fetch_one(conn.as_mut())
    .await?
    .xp;
    Ok(count)
}

pub async fn set_xp<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    user: Id<UserMarker>,
    guild: Id<GuildMarker>,
    amount: i64,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    if amount > 0 {
        query!(
            "INSERT INTO levels (id, guild, xp) VALUES ($1, $2, $3) \
                ON CONFLICT (id, guild) DO UPDATE SET xp=$3",
            id_to_db(user),
            id_to_db(guild),
            amount
        )
        .execute(conn.as_mut())
        .await?;
    } else {
        delete_levels_user_guild(conn.as_mut(), user, guild).await?;
    }

    Ok(())
}

#[derive(Debug, Copy, Clone, Hash)]
pub enum OnCooldown {
    Yes,
    No,
}

impl OnCooldown {
    #[must_use]
    pub const fn was_on_cooldown(self) -> bool {
        match self {
            Self::Yes => true,
            Self::No => false,
        }
    }
}

pub async fn set_cooldown<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    user: Id<UserMarker>,
    guild: Id<GuildMarker>,
    timestamp: i64,
    cooldown_duration: i64,
) -> Result<OnCooldown, Error> {
    let mut conn = conn.acquire().await?;
    let rows_affected = query!(
        "INSERT INTO cooldowns (guild_id, user_id, last_message) VALUES ($1, $2, $3)
        ON CONFLICT (guild_id, user_id) DO UPDATE SET last_message=excluded.last_message
        WHERE cooldowns.guild_id = excluded.guild_id AND cooldowns.user_id = excluded.user_id AND
        (cooldowns.last_message + $4) < excluded.last_message",
        id_to_db(guild),
        id_to_db(user),
        timestamp,
        cooldown_duration
    )
    .execute(conn.as_mut())
    .await?
    .rows_affected();
    let output = if rows_affected > 0 {
        OnCooldown::No
    } else {
        OnCooldown::Yes
    };
    Ok(output)
}

pub async fn add_audit_log_event<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    event: AuditLogEvent,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "INSERT INTO audit_logs \
            (guild, target, moderator,
                timestamp, previous, delta, kind)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id_to_db(event.guild),
        id_to_db(event.target),
        id_to_db(event.moderator),
        event.timestamp,
        event.previous,
        event.delta,
        event.kind.to_i64(),
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn get_audit_log_events<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    actions_on_user: Option<Id<UserMarker>>,
    actions_by_moderator: Option<Id<UserMarker>>,
) -> Result<Vec<AuditLogEvent>, Error> {
    let mut conn = conn.acquire().await?;
    let mut stream = query!(
        "SELECT target, moderator,
        timestamp, previous, delta, kind
        FROM audit_logs WHERE guild = $1",
        id_to_db(guild)
    )
    .fetch(conn.as_mut());
    let mut logs = Vec::new();
    while let Some(row) = stream.next().await.transpose()? {
        let target = db_to_id(row.target);
        let moderator = db_to_id(row.moderator);
        if actions_by_moderator.is_some_and(|requested_moderator| requested_moderator != moderator)
            || actions_on_user.is_some_and(|requested_user| requested_user != target)
        {
            continue;
        }
        let log = AuditLogEvent {
            guild,
            target,
            moderator: db_to_id(row.moderator),
            timestamp: row.timestamp,
            previous: row.previous,
            delta: row.delta,
            kind: AuditLogEventKind::from_i64(row.kind).ok_or(Error::UnknownAuditLogEventKind)?,
        };
        logs.push(log);
    }
    Ok(logs)
}

pub async fn delete_audit_log_events_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!("DELETE FROM audit_logs WHERE guild = $1", id_to_db(guild))
        .execute(conn.as_mut())
        .await?;
    Ok(())
}

pub async fn delete_audit_log_events_user<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    target: Id<UserMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!("DELETE FROM audit_logs WHERE target = $1", id_to_db(target))
        .execute(conn.as_mut())
        .await?;
    Ok(())
}

pub async fn delete_audit_log_events_user_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    target: Id<UserMarker>,
    guild: Id<GuildMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "DELETE FROM audit_logs WHERE target = $1 AND guild = $2",
        id_to_db(target),
        id_to_db(guild)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn get_last_message<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    user: Id<UserMarker>,
    guild: Id<GuildMarker>,
) -> Result<Option<i64>, Error> {
    let mut conn = conn.acquire().await?;
    let last_message = query!(
        "SELECT last_message FROM cooldowns WHERE guild_id = $1 AND user_id = $2",
        id_to_db(guild),
        id_to_db(user),
    )
    .fetch_optional(conn.as_mut())
    .await?
    .map(|v| v.last_message);
    Ok(last_message)
}

pub async fn delete_cooldowns_starting_before<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    timestamp: i64,
) -> Result<u64, Error> {
    let mut conn = conn.acquire().await?;
    let db_resp = query!("DELETE FROM cooldowns WHERE last_message < $1", timestamp)
        .execute(conn.as_mut())
        .await?;
    Ok(db_resp.rows_affected())
}

pub async fn delete_levels_user_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    user: Id<UserMarker>,
    guild: Id<GuildMarker>,
) -> Result<i64, Error> {
    let mut conn = conn.acquire().await?;
    let output = query!(
        "DELETE FROM levels WHERE id = $1 AND guild = $2 RETURNING xp",
        id_to_db(user),
        id_to_db(guild)
    )
    .fetch_optional(conn.as_mut())
    .await?;
    Ok(output.map_or(0, |v| v.xp))
}

pub async fn count_with_higher_xp<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    xp: i64,
) -> Result<Option<i64>, Error> {
    let mut conn = conn.acquire().await?;
    let count = query!(
        "SELECT COUNT(*) as count FROM levels WHERE xp > $1 AND guild = $2",
        xp,
        id_to_db(guild)
    )
    .fetch_one(conn.as_mut())
    .await?
    .count;
    Ok(count)
}

pub async fn levels_in_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<i64, Error> {
    let mut conn = conn.acquire().await?;
    let count = query!(
        "SELECT COUNT(id) as count FROM levels WHERE guild = $1",
        id_to_db(guild)
    )
    .fetch_one(conn.as_mut())
    .await?
    .count;
    Ok(count.unwrap_or(0))
}

pub async fn total_levels<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
) -> Result<i64, Error> {
    let mut conn = conn.acquire().await?;
    let count = query!(
        "SELECT reltuples::bigint AS count FROM pg_class
        WHERE oid = 'public.levels'::regclass",
    )
    .fetch_one(conn.as_mut())
    .await?
    .count;
    Ok(count.unwrap_or(0))
}

pub async fn user_xp<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
) -> Result<Option<i64>, Error> {
    let mut conn = conn.acquire().await?;
    // Select current XP from the database, return 0 if there is no row
    let xp = query!(
        "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
        id_to_db(user),
        id_to_db(guild)
    )
    .fetch_optional(conn.as_mut())
    .await?
    .map(|v| v.xp);
    Ok(xp)
}

pub async fn get_all_levels<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    user: Id<UserMarker>,
) -> Result<Vec<UserStatus>, Error> {
    let mut conn = conn.acquire().await?;
    let mut raw_levels =
        query!("SELECT guild, xp FROM levels WHERE id = $1", id_to_db(user)).fetch(conn.as_mut());
    // 200 was chosen because that's the max number of guilds you can be in.
    let mut output = Vec::with_capacity(200);
    while let Some(v) = raw_levels.next().await.transpose()? {
        let status = UserStatus {
            id: user,
            guild: db_to_id(v.guild),
            xp: v.xp,
        };
        output.push(status);
    }
    Ok(output)
}

pub async fn card_customizations<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    targets: &[Id<GenericMarker>],
) -> Result<Option<RawCustomizations>, Error> {
    let mut conn = conn.acquire().await?;
    let targets: Vec<i64> = targets.iter().copied().map(id_to_db).collect();
    let data = query_as!(
        RawCustomizations,
        "SELECT * FROM UNNEST($1::INT8[]) WITH ORDINALITY \
                AS ordering_ids(ord_id, ordinality) \
                INNER JOIN custom_card ON ordering_ids.ord_id = custom_card.id \
                ORDER BY ordering_ids.ordinality \
                LIMIT 1",
        &targets
    )
    .fetch_optional(conn.as_mut())
    .await?;
    Ok(data)
}

pub async fn delete_card_customizations<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    target: Id<GenericMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!("DELETE FROM custom_card WHERE id = $1", id_to_db(target))
        .execute(conn.as_mut())
        .await?;
    Ok(())
}

pub async fn delete_levels_user<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    id: Id<UserMarker>,
) -> Result<u64, Error> {
    let mut conn = conn.acquire().await?;
    let rows = query!("DELETE FROM levels WHERE id = $1", id_to_db(id))
        .execute(conn.as_mut())
        .await?
        .rows_affected();
    Ok(rows)
}

pub async fn delete_levels_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    id: Id<GuildMarker>,
) -> Result<u64, Error> {
    let mut conn = conn.acquire().await?;
    let rows = query!("DELETE FROM levels WHERE guild = $1", id_to_db(id))
        .execute(conn.as_mut())
        .await?
        .rows_affected();
    Ok(rows)
}

pub async fn ban_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    id: Id<GuildMarker>,
    duration: Option<f64>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "INSERT INTO guild_bans (id, expires) \
                VALUES ($1, \
                CASE WHEN $3 \
                THEN NULL \
                ELSE NOW() + interval '1' day * $2 END)",
        id_to_db(id),
        duration,
        duration.is_none()
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn pardon_guild<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    id: Id<GuildMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!("DELETE FROM guild_bans WHERE id = $1", id_to_db(id))
        .execute(conn.as_mut())
        .await?;
    Ok(())
}

pub async fn is_guild_banned<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<bool, Error> {
    let mut conn = conn.acquire().await?;
    let banned = query!(
        "SELECT id FROM guild_bans WHERE
            ((expires > NOW()) OR (expires IS NULL))
            AND id = $1 LIMIT 1",
        id_to_db(guild)
    )
    .fetch_optional(conn.as_mut())
    .await?
    .is_some();
    Ok(banned)
}

pub async fn update_card<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    id: Id<GenericMarker>,
    update: &CardUpdate,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "INSERT INTO custom_card (
                username,
                rank,
                level,
                border,
                background,
                progress_foreground,
                progress_background,
                foreground_xp_count,
                background_xp_count,
                font,
                toy_image,
                card_layout,
                id
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, COALESCE($12, $13), $14
            ) ON CONFLICT (id) DO UPDATE SET
                username = COALESCE($1, custom_card.username),
                rank = COALESCE($2, custom_card.rank),
                level = COALESCE($3, custom_card.level),
                border = COALESCE($4, custom_card.border),
                background = COALESCE($5, custom_card.background),
                progress_foreground = COALESCE($6, custom_card.progress_foreground),
                progress_background = COALESCE($7, custom_card.progress_background),
                foreground_xp_count = COALESCE($8, custom_card.foreground_xp_count),
                background_xp_count = COALESCE($9, custom_card.background_xp_count),
                font = COALESCE($10, custom_card.font),
                toy_image = COALESCE($11, custom_card.toy_image),
                card_layout = COALESCE($12, custom_card.card_layout, $13)",
        update.username,
        update.rank,
        update.level,
        update.border,
        update.background,
        update.progress_foreground,
        update.progress_background,
        update.foreground_xp_count,
        update.background_xp_count,
        update.font,
        update.toy_image,
        update.card_layout,
        update.card_layout_default,
        id_to_db(id)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn update_guild_config<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    cfg: UpdateGuildConfig,
) -> Result<GuildConfig, Error> {
    let mut conn = conn.acquire().await?;
    let config = query_as!(
                RawGuildConfig,
                "INSERT INTO guild_configs (id, level_up_message, level_up_channel, ping_on_level_up, \
                    max_xp_per_message, min_xp_per_message, message_cooldown, one_at_a_time,
                    guild_card_default_show_off) \
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, COALESCE($9, FALSE)) \
                ON CONFLICT (id) DO UPDATE SET \
                level_up_message = COALESCE($2, guild_configs.level_up_message), \
                level_up_channel = COALESCE($3, guild_configs.level_up_channel), \
                ping_on_level_up = COALESCE($4, guild_configs.ping_on_level_up), \
                max_xp_per_message = COALESCE($5, guild_configs.max_xp_per_message), \
                min_xp_per_message = COALESCE($6, guild_configs.min_xp_per_message), \
                message_cooldown = COALESCE($7, guild_configs.message_cooldown), \
                one_at_a_time = COALESCE($8, guild_configs.one_at_a_time), \
                guild_card_default_show_off = COALESCE($9, guild_configs.guild_card_default_show_off) \
                RETURNING one_at_a_time, level_up_message, level_up_channel, ping_on_level_up, \
                max_xp_per_message, min_xp_per_message, message_cooldown, \
                guild_card_default_show_off",
                id_to_db(guild),
                cfg.level_up_message.map(|v| v),
                cfg.level_up_channel.as_ref().map(|id| id_to_db(*id)),
                cfg.ping_users,
                cfg.max_xp_per_message,
                cfg.min_xp_per_message,
                cfg.message_cooldown,
                cfg.one_at_a_time,
                cfg.guild_card_default_show_off
            )
        .fetch_one(conn.as_mut())
        .await?
        .cook()?;
    Ok(config)
}

pub async fn delete_guild_config<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!("DELETE FROM guild_configs WHERE id = $1", id_to_db(guild))
        .execute(conn.as_mut())
        .await?;
    Ok(())
}

pub async fn add_guild_cleanup<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "INSERT INTO guild_cleanups (guild, removed_at) VALUES ($1, NOW())
        ON CONFLICT (guild) DO UPDATE SET removed_at = excluded.removed_at",
        id_to_db(guild)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn delete_guild_cleanup<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "DELETE FROM guild_cleanups WHERE guild = $1",
        id_to_db(guild)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn get_active_guild_cleanups<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
) -> Result<Vec<Id<GuildMarker>>, Error> {
    let mut conn = conn.acquire().await?;
    let mut records =
        query!("SELECT guild FROM guild_cleanups WHERE removed_at + interval '30 days' < NOW()")
            .fetch(conn.as_mut());

    let mut output = Vec::with_capacity(16);
    while let Some(v) = records.next().await {
        let v = match v {
            Ok(v) => v,
            Err(source) => {
                tracing::error!(?source, "Error getting guild cleanups");
                continue;
            }
        };
        output.push(db_to_id(v.guild));
    }
    Ok(output)
}

pub async fn add_user_guild_cleanup<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "INSERT INTO user_cleanups (guild_id, user_id, removed_at) VALUES ($1, $2, NOW())
        ON CONFLICT (guild_id, user_id) DO UPDATE SET removed_at = excluded.removed_at",
        id_to_db(guild),
        id_to_db(user)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn delete_user_guild_cleanup<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "DELETE FROM user_cleanups WHERE guild_id = $1 AND user_id = $2",
        id_to_db(guild),
        id_to_db(user)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

pub async fn get_active_user_guild_cleanups<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
) -> Result<Vec<UserInGuild>, Error> {
    let mut conn = conn.acquire().await?;
    let mut records = query!(
        "SELECT user_id, guild_id FROM user_cleanups WHERE removed_at + interval '30 days' < NOW()"
    )
    .fetch(conn.as_mut());

    let mut output = Vec::with_capacity(16);
    while let Some(v) = records.next().await {
        let v = match v {
            Ok(v) => v,
            Err(source) => {
                tracing::error!(?source, "Error getting guild cleanups");
                continue;
            }
        };
        let guild = db_to_id(v.guild_id);
        let user = db_to_id(v.user_id);
        output.push(UserInGuild { guild, user });
    }
    Ok(output)
}

pub async fn get_leaderboard_page<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserStatus>, Error> {
    let mut conn = conn.acquire().await?;
    let mut users = query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY (xp, id) DESC LIMIT $2 OFFSET $3",
        id_to_db(guild),
        limit,
        offset
    )
    .fetch(conn.as_mut());
    let mut output = Vec::with_capacity(limit.try_into().unwrap_or(10));
    while let Some(rec) = users.next().await.transpose()? {
        let status = UserStatus {
            id: db_to_id(rec.id),
            guild,
            xp: rec.xp,
        };
        output.push(status);
    }
    Ok(output)
}

pub async fn add_reward_role<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    requirement: i64,
    role: Id<RoleMarker>,
) -> Result<(), Error> {
    let mut conn = conn.acquire().await?;
    query!(
        "INSERT INTO role_rewards (id, requirement, guild) VALUES ($1, $2, $3) \
        ON CONFLICT (id, guild) DO UPDATE SET requirement = $2",
        id_to_db(role),
        requirement,
        id_to_db(guild)
    )
    .execute(conn.as_mut())
    .await?;
    Ok(())
}

/// Returns number of rows affected.
/// If two Some values are passed in, all the values that match *either* will be deleted.
/// TODO: Consider if this behavior makes sense. Maybe it should be and.
pub async fn delete_reward_role<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
    requirement: Option<i64>,
    role: Option<Id<RoleMarker>>,
) -> Result<u64, Error> {
    let mut conn = conn.acquire().await?;
    if requirement.is_none() && role.is_none() {
        return Err(Error::UnspecifiedDelete);
    }
    let rows = query!(
        "DELETE FROM role_rewards WHERE guild = $1 AND (id = $2 OR requirement = $3)",
        id_to_db(guild),
        role.map(id_to_db),
        requirement
    )
    .execute(conn.as_mut())
    .await?
    .rows_affected();
    Ok(rows)
}

pub async fn export_bulk_users<
    'a,
    D: DerefMut<Target = PgConnection> + Send,
    A: Acquire<'a, Database = Postgres, Connection = D> + Send,
>(
    conn: A,
    guild: Id<GuildMarker>,
) -> Result<Vec<UserStatus>, Error> {
    let mut conn = conn.acquire().await?;
    let mut records = query!(
        "SELECT id, xp FROM levels WHERE guild = $1",
        id_to_db(guild)
    )
    .fetch(conn.as_mut());
    let mut out = Vec::with_capacity(256);
    while let Some(Ok(rec)) = records.next().await {
        let status = UserStatus {
            id: db_to_id(rec.id),
            guild,
            xp: rec.xp,
        };
        out.push(status);
    }
    Ok(out)
}

#[derive(Default)]
pub struct UpdateGuildConfig {
    pub level_up_message: Option<String>,
    pub level_up_channel: Option<Id<ChannelMarker>>,
    pub ping_users: Option<bool>,
    pub max_xp_per_message: Option<i16>,
    pub min_xp_per_message: Option<i16>,
    pub message_cooldown: Option<i16>,
    pub one_at_a_time: Option<bool>,
    pub guild_card_default_show_off: Option<bool>,
}

macro_rules! setter {
    ($name:ident, $kind:ty) => {
        #[must_use]
        #[allow(clippy::missing_const_for_fn)]
        pub fn $name(mut self, p1: Option<$kind>) -> Self {
            if p1.is_some() {
                self.$name = p1;
            }
            self
        }
    };
}

impl UpdateGuildConfig {
    setter!(level_up_message, String);

    setter!(level_up_channel, Id<ChannelMarker>);

    setter!(ping_users, bool);

    setter!(max_xp_per_message, i16);

    setter!(min_xp_per_message, i16);

    setter!(message_cooldown, i16);

    setter!(one_at_a_time, bool);

    setter!(guild_card_default_show_off, bool);

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct CardUpdate {
    pub username: Option<String>,
    pub rank: Option<String>,
    pub level: Option<String>,
    pub border: Option<String>,
    pub background: Option<String>,
    pub progress_background: Option<String>,
    pub progress_foreground: Option<String>,
    pub foreground_xp_count: Option<String>,
    pub background_xp_count: Option<String>,
    pub font: Option<String>,
    pub toy_image: Option<String>,
    pub card_layout: Option<String>,
    pub card_layout_default: String,
}

pub struct RawCustomizations {
    pub username: Option<String>,
    pub rank: Option<String>,
    pub level: Option<String>,
    pub border: Option<String>,
    pub background: Option<String>,
    pub progress_foreground: Option<String>,
    pub progress_background: Option<String>,
    pub background_xp_count: Option<String>,
    pub foreground_xp_count: Option<String>,
    pub font: Option<String>,
    pub toy_image: Option<String>,
    pub card_layout: String,
    #[allow(dead_code)]
    id: I64Placeholder,
    #[allow(dead_code)]
    ord_id: I64Placeholder,
    #[allow(dead_code)]
    ordinality: I64Placeholder,
}

struct I64Placeholder;

impl From<i64> for I64Placeholder {
    fn from(_: i64) -> Self {
        Self
    }
}

impl From<Option<i64>> for I64Placeholder {
    fn from(_: Option<i64>) -> Self {
        Self
    }
}

pub struct RawGuildConfig {
    pub one_at_a_time: Option<bool>,
    pub level_up_message: Option<String>,
    pub level_up_channel: Option<i64>,
    pub ping_on_level_up: Option<bool>,
    pub min_xp_per_message: Option<i16>,
    pub max_xp_per_message: Option<i16>,
    pub message_cooldown: Option<i16>,
    pub guild_card_default_show_off: bool,
}

impl RawGuildConfig {
    fn cook(self) -> Result<GuildConfig, simpleinterpolation::ParseError> {
        let level_up_message = if let Some(str) = self.level_up_message {
            Some(Interpolation::new(str)?)
        } else {
            None
        };

        let gc = GuildConfig {
            one_at_a_time: self.one_at_a_time,
            level_up_message,
            level_up_channel: self.level_up_channel.map(db_to_id),
            ping_on_level_up: self.ping_on_level_up,
            min_xp_per_message: self.min_xp_per_message,
            max_xp_per_message: self.max_xp_per_message,
            cooldown: self.message_cooldown,
            guild_card_default_show_off: self.guild_card_default_show_off,
        };
        Ok(gc)
    }
}

#[derive(Debug)]
pub enum Error {
    Database(sqlx::Error),
    Interpolation(simpleinterpolation::ParseError),
    UnspecifiedDelete,
    UnknownAuditLogEventKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(de) => write!(f, "{de}"),
            Self::Interpolation(ie) => write!(f, "{ie}"),
            Self::UnspecifiedDelete => f.write_str("No constraints specified to delete by."),
            Self::UnknownAuditLogEventKind => f.write_str("Unknown audit log event kind"),
        }
    }
}

impl std::error::Error for Error {}

macro_rules! gen_from {
    ($fr:ty, $to:ty, $variant:ident) => {
        impl From<$fr> for $to {
            fn from(value: $fr) -> Self {
                Self::$variant(value)
            }
        }
    };
}

gen_from!(sqlx::Error, Error, Database);
gen_from!(simpleinterpolation::ParseError, Error, Interpolation);
