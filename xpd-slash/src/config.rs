use simpleinterpolation::Interpolation;
use twilight_model::{
    channel::{message::MessageFlags, ChannelType},
    id::{
        marker::{GuildMarker, RoleMarker},
        Id,
    },
};
use xpd_common::{
    GuildConfig, DEFAULT_MAX_XP_PER_MESSAGE, DEFAULT_MIN_XP_PER_MESSAGE, TEMPLATE_VARIABLES,
};
use xpd_database::UpdateGuildConfig;
use xpd_util::CanAddRole;

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
        ConfigCommand::Get(_) => xpd_database::guild_config(&state.db, guild)
            .await
            .map(|v| v.unwrap_or_default().to_string())
            .map_err(Into::into),
        ConfigCommand::Rewards(r) => process_rewards_config(state, guild, r).await,
        ConfigCommand::Levels(l) => process_levels_config(state, guild, l).await,
        ConfigCommand::PermsCheckup(_) => process_perm_checkup(state, guild).await,
    }
    .map(|s| XpdSlashResponse::with_embed_text(s).flags(MessageFlags::EPHEMERAL))
}

async fn process_rewards_config(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    options: ConfigCommandRewards,
) -> Result<String, Error> {
    let new_cfg = UpdateGuildConfig::new().one_at_a_time(options.one_at_a_time);
    let mut update_txn = state.db.begin().await?;
    let config = xpd_database::update_guild_config(&mut update_txn, guild_id, new_cfg).await?;
    validate_config(&config)?;
    update_txn.commit().await?;
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

    let new_cfg = UpdateGuildConfig {
        level_up_message: options.level_up_message,
        level_up_channel: options.level_up_channel.map(|v| v.id),
        ping_users: options.ping_users,
        max_xp_per_message,
        min_xp_per_message,
        message_cooldown,
        one_at_a_time: None,
    };
    let mut validate_txn = state.db.begin().await?;
    let config = xpd_database::update_guild_config(&mut validate_txn, guild_id, new_cfg).await?;
    validate_config(&config)?;
    validate_txn.commit().await?;
    let msg = config.to_string();
    state.update_config(guild_id, config).await;

    Ok(msg)
}

fn safecast_to_i16(ou16: Option<i64>) -> Result<Option<i16>, Error> {
    ou16.map(TryInto::try_into).transpose().map_err(Into::into)
}

async fn reset_config(state: SlashState, guild_id: Id<GuildMarker>) -> Result<String, Error> {
    xpd_database::delete_guild_config(&state.db, guild_id).await?;
    state.update_config(guild_id, GuildConfig::default()).await;
    Ok("Reset guild reward config, but NOT rewards themselves!".to_string())
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

async fn process_perm_checkup(
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    let config = xpd_database::guild_config(&state.db, guild_id)
        .await?
        .unwrap_or_default();
    let can_msg_in_level_up = config
        .level_up_channel
        .map(|level_up| xpd_util::can_create_message(&state.cache, state.my_id.cast(), level_up))
        .transpose()?;

    let rewards: Vec<Id<RoleMarker>> = xpd_database::guild_rewards(&state.db, guild_id)
        .await?
        .iter()
        .map(|v| v.id)
        .collect();

    let can_add_roles =
        xpd_util::can_add_roles(&state.cache, state.my_id.cast(), guild_id, &rewards)?;
    let good_msg_state = EmojiFormatBool(can_msg_in_level_up != Some(false));

    let can_add_roles = match can_add_roles {
        CanAddRole::Yes => "✅",
        CanAddRole::NoManageRoles => "⚠️ No manage roles permission!",
        CanAddRole::HighestRoleIsLowerRoleThanTarget => {
            "⚠️ My highest role is lower than the ones I need to assign!"
        }
        CanAddRole::RoleIsManaged => "⚠️ That role is managed by another bot.",
    };
    Ok(format!(
        "Can add roles: {can_add_roles}\nCan message in level up channel: {good_msg_state}"
    ))
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
struct EmojiFormatBool(pub bool);

impl std::fmt::Display for EmojiFormatBool {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 {
            f.write_str("✅")
        } else {
            f.write_str("❎")
        }
    }
}
