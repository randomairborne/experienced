use std::{
    process::{ExitCode, Termination},
    time::{Duration, UNIX_EPOCH},
};

use sqlx::{Connection, PgConnection, Postgres, Transaction};
use twilight_model::id::{marker::GuildMarker, Id};
use xpd_common::DISCORD_EPOCH_SECS;

#[macro_use]
extern crate tracing;

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().json().init();
    info!(
        version = xpd_common::CURRENT_GIT_SHA,
        "Starting experienced cleanup!"
    );
    let database_url = valk_utils::get_var("DATABASE_URL");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(&database_url))
}

async fn async_main(database_url: &str) -> Result<(), Error> {
    debug!(database_url, "Connecting to database");
    let mut conn = PgConnection::connect(database_url).await?;
    info!(database_url, "Connected to database");
    let cleanups = xpd_database::get_active_guild_cleanups(&mut conn).await?;
    info!(?cleanups, count = cleanups.len(), "Got cleanups");
    for guild in cleanups {
        debug!(%guild, "Cleaning guild");
        let mut txn = conn.begin().await?;
        if let Err(source) = cleanup_guild(&mut txn, guild).await {
            error!(%guild, ?source, "Unable to invalidate rewards for guild");
            txn.rollback().await?;
            continue;
        }
        if let Err(source) = txn.commit().await {
            error!(%guild, ?source, "Unable to commit changes for guild");
        }
        info!(%guild, "Cleaned guild");
    }
    cleanup_cooldowns(&mut conn).await?;
    info!("Done!");
    Ok(())
}

async fn cleanup_cooldowns(db: &mut PgConnection) -> Result<(), Error> {
    let discord_epoch = Duration::from_secs(DISCORD_EPOCH_SECS.try_into().unwrap());
    let max_message_cooldown =
        Duration::from_secs(xpd_common::MAX_MESSAGE_COOLDOWN.try_into().unwrap());

    let now_discord = UNIX_EPOCH
        .elapsed()?
        .checked_add(discord_epoch)
        .and_then(|v| v.checked_sub(max_message_cooldown))
        .ok_or(Error::GenericTime)?
        .as_secs()
        .try_into()
        .unwrap_or(0); // nothing can start before 0
    warn!(now_discord, "Deleting cooldowns starting before");
    xpd_database::delete_cooldowns_starting_before(db, now_discord).await?;
    Ok(())
}

async fn cleanup_guild(
    db: &mut Transaction<'_, Postgres>,
    guild: Id<GuildMarker>,
) -> Result<(), Error> {
    debug!(%guild, "Deleting guild configs");
    xpd_database::delete_guild_config(db.as_mut(), guild).await?;
    debug!(%guild, "Deleting guild card customizations");
    xpd_database::delete_card_customizations(db.as_mut(), guild.cast()).await?;
    debug!(%guild, "Deleting guild rewards");
    let rewards = xpd_database::guild_rewards(db.as_mut(), guild).await?;
    debug!(%guild, count = rewards.len(), "Deleting guild rewards");
    for reward in rewards {
        trace!(%guild, id = %reward.id, requirement = reward.requirement, "Deleting guild reward");
        xpd_database::delete_reward_role(db.as_mut(), guild, None, Some(reward.id)).await?;
    }
    debug!(%guild, "Deleting guild levels");
    xpd_database::delete_levels_guild(db.as_mut(), guild).await?;
    debug!(%guild, "Acknowledging guild has been cleaned up");
    xpd_database::delete_guild_cleanup(db.as_mut(), guild).await?;
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Sqlx(sqlx::Error),
    DbReq(xpd_database::Error),
    SystemTime(std::time::SystemTimeError),
    GenericTime,
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<xpd_database::Error> for Error {
    fn from(value: xpd_database::Error) -> Self {
        Self::DbReq(value)
    }
}

impl From<std::time::SystemTimeError> for Error {
    fn from(value: std::time::SystemTimeError) -> Self {
        Self::SystemTime(value)
    }
}

impl Termination for Error {
    fn report(self) -> ExitCode {
        ExitCode::FAILURE
    }
}
