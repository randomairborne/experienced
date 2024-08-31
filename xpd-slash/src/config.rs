use simpleinterpolation::Interpolation;
use twilight_model::{
    channel::{message::MessageFlags, ChannelType},
    id::{marker::GuildMarker, Id},
};
use xpd_common::{
    id_to_db, GuildConfig, RawGuildConfig, DEFAULT_MAX_XP_PER_MESSAGE, DEFAULT_MIN_XP_PER_MESSAGE,
    TEMPLATE_VARIABLES,
};

use crate::{
    cmd_defs::{
        config::{ConfigCommandLevels, ConfigCommandRewards},
        ConfigCommand,
    },
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_config(
    command: ConfigCommand,
    guild: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    match command {
        ConfigCommand::Reset(_) => reset_config(state, guild).await,
        ConfigCommand::Get(_) => get_config(state, guild).await,
        ConfigCommand::Rewards(r) => process_rewards_config(state, guild, r).await,
        ConfigCommand::Levels(l) => process_levels_config(state, guild, l).await,
    }
    .map(|s| XpdSlashResponse::with_embed_text(s).flags(MessageFlags::EPHEMERAL))
}

async fn process_rewards_config(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    options: ConfigCommandRewards,
) -> Result<String, Error> {
    let config = query_as!(
        RawGuildConfig,
        "INSERT INTO guild_configs (id, one_at_a_time) VALUES ($1, $2) \
            ON CONFLICT (id) DO UPDATE SET \
            one_at_a_time = COALESCE($2, excluded.one_at_a_time) \
            RETURNING one_at_a_time, level_up_message, level_up_channel, ping_on_level_up, \
            max_xp_per_message, min_xp_per_message, message_cooldown",
        id_to_db(guild_id),
        options.one_at_a_time,
    )
    .fetch_one(&state.db)
    .await?
    .try_into()?;
    state.update_config(guild_id, config).await;
    Ok("Updated rewards config!".to_string())
}

async fn process_levels_config(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    options: ConfigCommandLevels,
) -> Result<String, Error> {
    if let Some(interp_template) = options.level_up_message.as_ref() {
        if interp_template.len() > 512 {
            return Err(Error::LevelUpMessageTooLong);
        }
        let interp = Interpolation::new(interp_template.clone())?;
        for item in interp.variables_used() {
            if !TEMPLATE_VARIABLES.contains(&item) {
                return Err(Error::UnknownInterpolationVariable(item.to_string()));
            }
        }
    }

    if options
        .level_up_channel
        .as_ref()
        .is_some_and(|v| !matches!(v.kind, ChannelType::GuildText))
    {
        return Err(Error::LevelUpChannelMustBeText);
    }

    let max_xp_per_message = safecast_to_i16(options.max_xp_per_message)?;
    let min_xp_per_message = safecast_to_i16(options.min_xp_per_message)?;
    let message_cooldown = safecast_to_i16(options.message_cooldown)?;

    let mut txn = state.db.begin().await?;

    let config: GuildConfig = query_as!(
        RawGuildConfig,
        "INSERT INTO guild_configs (id, level_up_message, level_up_channel, ping_on_level_up) \
            VALUES ($1, $2, $3, $4) \
            ON CONFLICT (id) DO UPDATE SET \
            level_up_message = COALESCE($2, excluded.level_up_message), \
            level_up_channel = COALESCE($3, excluded.level_up_channel), \
            ping_on_level_up = COALESCE($4, excluded.ping_on_level_up), \
            max_xp_per_message = COALESCE($5, excluded.max_xp_per_message), \
            min_xp_per_message = COALESCE($6, excluded.min_xp_per_message), \
            message_cooldown = COALESCE($7, excluded.message_cooldown) \
            RETURNING one_at_a_time, level_up_message, level_up_channel, ping_on_level_up, \
            max_xp_per_message, min_xp_per_message, message_cooldown",
        id_to_db(guild_id),
        options.level_up_message,
        options.level_up_channel.as_ref().map(|ic| id_to_db(ic.id)),
        options.ping_users,
        max_xp_per_message,
        min_xp_per_message,
        message_cooldown
    )
    .fetch_one(txn.as_mut())
    .await?
    .try_into()?;

    validate_config(&config)?;
    let msg = config.to_string();
    // commit config to memory, no turning back
    txn.commit().await?;
    state.update_config(guild_id, config).await;

    Ok(msg)
}

fn safecast_to_i16(ou16: Option<i64>) -> Result<Option<i16>, Error> {
    ou16.map(TryInto::try_into).transpose().map_err(Into::into)
}

async fn reset_config(state: SlashState, guild_id: Id<GuildMarker>) -> Result<String, Error> {
    query!(
        "DELETE FROM guild_configs WHERE id = $1",
        id_to_db(guild_id)
    )
    .execute(&state.db)
    .await?;
    state.update_config(guild_id, GuildConfig::default()).await;
    Ok("Reset guild reward config, but NOT rewards themselves!".to_string())
}

async fn get_config(state: SlashState, guild_id: Id<GuildMarker>) -> Result<String, Error> {
    let config: GuildConfig = query_as!(
        RawGuildConfig,
        "SELECT one_at_a_time, level_up_message, level_up_channel, ping_on_level_up, max_xp_per_message, \
        min_xp_per_message, message_cooldown FROM guild_configs \
        WHERE id = $1",
        id_to_db(guild_id),
    )
    .fetch_optional(&state.db)
    .await?
    .map_or_else(|| Ok(GuildConfig::default()), TryInto::try_into)?;
    Ok(config.to_string())
}

fn validate_config(config: &GuildConfig) -> Result<(), GuildConfigErrorReport> {
    let max_xp_per_msg = config
        .max_xp_per_message
        .unwrap_or(DEFAULT_MAX_XP_PER_MESSAGE);
    let min_xp_per_msg = config
        .min_xp_per_message
        .unwrap_or(DEFAULT_MIN_XP_PER_MESSAGE);
    if max_xp_per_msg < min_xp_per_msg {
        return Err(GuildConfigErrorReport::MinXpIsMoreThanMax {
            min: min_xp_per_msg,
            max: max_xp_per_msg,
        });
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum GuildConfigErrorReport {
    #[error("The selected minimum XP value of {min} is more than the selected maximum of {max}")]
    MinXpIsMoreThanMax { min: i16, max: i16 },
}
