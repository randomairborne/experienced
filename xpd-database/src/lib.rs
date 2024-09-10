#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// we allow AFIT because all crates in experienced are internal
#![allow(async_fn_in_trait)]

mod util;

use std::fmt::Display;

use simpleinterpolation::Interpolation;
pub use sqlx::PgPool;
use sqlx::{query, query_as};
use tokio_stream::StreamExt;
use twilight_model::id::{
    marker::{GenericMarker, GuildMarker, UserMarker},
    Id,
};
use util::{db_to_id, id_to_db};
use xpd_common::{GuildConfig, RoleReward, UserStatus};

pub trait Database {
    fn db(&self) -> &PgPool;

    async fn query_guild_rewards(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Vec<RoleReward>, Error> {
        let rewards: Vec<RoleReward> = query!(
            "SELECT id, requirement FROM role_rewards WHERE guild = $1",
            id_to_db(guild_id),
        )
        .fetch_all(self.db())
        .await?
        .into_iter()
        .map(|row| RoleReward {
            id: db_to_id(row.id),
            requirement: row.requirement,
        })
        .collect();
        Ok(rewards)
    }

    async fn query_guild_config(
        &self,
        guild: Id<GuildMarker>,
    ) -> Result<Option<GuildConfig>, Error> {
        let config = query_as!(
            RawGuildConfig,
            "SELECT one_at_a_time, level_up_message, level_up_channel, ping_on_level_up,\
             max_xp_per_message, min_xp_per_message, message_cooldown \
             FROM guild_configs WHERE id = $1",
            id_to_db(guild)
        )
        .fetch_optional(self.db())
        .await?
        .map(RawGuildConfig::cook)
        .transpose()?;
        Ok(config)
    }

    async fn query_add_xp(
        &self,
        author: Id<UserMarker>,
        guild: Id<GuildMarker>,
        amount: i64,
    ) -> Result<i64, Error> {
        let count = query!(
            "INSERT INTO levels (id, guild, xp) VALUES ($1, $2, $3) \
                ON CONFLICT (id, guild) \
                DO UPDATE SET xp=levels.xp+excluded.xp \
                RETURNING xp",
            id_to_db(author),
            id_to_db(guild),
            amount
        )
        .fetch_one(self.db())
        .await?
        .xp;
        Ok(count)
    }

    async fn query_count_with_higher_xp(
        &self,
        guild: Id<GuildMarker>,
        xp: i64,
    ) -> Result<Option<i64>, Error> {
        let count = query!(
            "SELECT COUNT(*) as count FROM levels WHERE xp > $1 AND guild = $2",
            xp,
            id_to_db(guild)
        )
        .fetch_one(self.db())
        .await?
        .count;
        Ok(count)
    }

    async fn query_user_xp(
        &self,
        guild: Id<GuildMarker>,
        user: Id<UserMarker>,
    ) -> Result<Option<i64>, Error> {
        // Select current XP from the database, return 0 if there is no row
        let xp = query!(
            "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
            id_to_db(user),
            id_to_db(guild)
        )
        .fetch_optional(self.db())
        .await?
        .map(|v| v.xp);
        Ok(xp)
    }

    async fn query_clear_all_user_data(&self, user: Id<UserMarker>) -> Result<(), Error> {
        let user_id = id_to_db(user);
        let mut txn = self.db().begin().await?;
        query!("DELETE FROM levels WHERE id = $1", user_id)
            .execute(txn.as_mut())
            .await?;
        query!("DELETE FROM custom_card WHERE id = $1", user_id)
            .execute(txn.as_mut())
            .await?;
        txn.commit().await?;
        Ok(())
    }

    async fn query_get_all_levels(&self, user: Id<UserMarker>) -> Result<Vec<UserStatus>, Error> {
        let mut raw_levels =
            query!("SELECT guild, xp FROM levels WHERE id = $1", id_to_db(user)).fetch(self.db());
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

    async fn query_card_customizations(
        &self,
        targets: &[Id<GenericMarker>],
    ) -> Result<Option<RawCustomizations>, Error> {
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
        .fetch_optional(self.db())
        .await?;
        Ok(data)
    }
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
    id: I64Placeholder,
    ord_id: I64Placeholder,
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
}

impl RawGuildConfig {
    fn cook(self) -> Result<GuildConfig, simpleinterpolation::Error> {
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
        };
        Ok(gc)
    }
}

#[derive(Debug)]
pub enum Error {
    Database(sqlx::Error),
    Interpolation(simpleinterpolation::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(de) => write!(f, "{de}"),
            Self::Interpolation(ie) => write!(f, "{ie}"),
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
gen_from!(simpleinterpolation::Error, Error, Interpolation);
