use simpleinterpolation::Interpolation;
use twilight_model::{
    channel::ChannelType,
    id::{marker::GuildMarker, Id},
};
use xpd_common::{id_to_db, GuildConfig, RawGuildConfig, TEMPLATE_VARIABLES};

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
        ConfigCommand::Rewards(r) => process_rewards_config(state, guild, r).await,
        ConfigCommand::Levels(l) => process_levels_config(state, guild, l).await,
    }
    .map(XpdSlashResponse::with_embed_text)
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
            RETURNING one_at_a_time, level_up_message, level_up_channel",
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

    let config: GuildConfig = query_as!(
        RawGuildConfig,
        "INSERT INTO guild_configs (id, level_up_message, level_up_channel) VALUES ($1, $2, $3) \
            ON CONFLICT (id) DO UPDATE SET \
            level_up_message = COALESCE($2, excluded.level_up_message), \
            level_up_channel = COALESCE($3, excluded.level_up_channel) \
            RETURNING one_at_a_time, level_up_message, level_up_channel",
        id_to_db(guild_id),
        options.level_up_message,
        options.level_up_channel.as_ref().map(|ic| id_to_db(ic.id))
    )
    .fetch_one(&state.db)
    .await?
    .try_into()?;

    let message = if let Some(message) = options.level_up_message {
        let new_channel = if let Some(channel) = options.level_up_channel {
            format!("<#{}>.", channel.id)
        } else {
            "the same channel as the message that caused the level-up".to_string()
        };
        format!("Level-up message is `{message}`, and it will be sent in {new_channel}.")
    } else {
        "Settings left unchanged.".to_string()
    };
    state.update_config(guild_id, config).await;
    Ok(message)
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
