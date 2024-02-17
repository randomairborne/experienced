use twilight_interactions::command::CommandModel;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{application_command::CommandData, Interaction, InteractionData},
    },
    id::{marker::GuildMarker, Id},
    user::User,
};

use crate::{
    cmd_defs::{AdminCommand, CardCommand, GdprCommand, XpCommand},
    Error, SlashState, XpdSlashResponse,
};

pub async fn process(
    interaction: Interaction,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    process_app_cmd(interaction, state).await
}

async fn process_app_cmd(
    interaction: Interaction,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    trace!("{interaction:#?}");
    let data = if let Some(data) = interaction.data {
        if let InteractionData::ApplicationCommand(cmd) = data {
            *cmd
        } else {
            return Err(Error::WrongInteractionData);
        }
    } else {
        return Err(Error::NoInteractionData);
    };
    let invoker = match interaction.member {
        Some(val) => val.user,
        None => interaction.user,
    }
    .ok_or(Error::NoInvoker)?;
    let guild_id = interaction.guild_id.ok_or(Error::NoGuildId)?;
    match data.kind {
        CommandType::ChatInput => {
            process_slash_cmd(data, guild_id, invoker, state, interaction.token).await
        }
        CommandType::User => process_user_cmd(data, guild_id, invoker, state).await,
        CommandType::Message => process_msg_cmd(data, guild_id, invoker, state).await,
        _ => Err(Error::WrongInteractionData),
    }
}

async fn process_slash_cmd(
    data: CommandData,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: SlashState,
    interaction_token: String,
) -> Result<XpdSlashResponse, Error> {
    match data.name.as_str() {
        "help" => Ok(crate::help::help()),
        "rank" => {
            let target = crate::cmd_defs::RankCommand::from_interaction(data.into())?
                .user
                .map_or_else(|| invoker.clone(), |v| v.resolved);
            crate::levels::get_level(guild_id, target, invoker, state).await
        }
        "xp" => {
            crate::manager::process_xp(
                XpCommand::from_interaction(data.into())?,
                interaction_token,
                guild_id,
                state,
            )
            .await
        }
        "admin" => {
            crate::admin::process_admin(
                AdminCommand::from_interaction(data.into())?,
                guild_id,
                invoker.id,
                state,
            )
            .await
        }
        "card" => Ok(crate::manage_card::card_update(
            CardCommand::from_interaction(data.into())?,
            invoker,
            &state,
            guild_id,
        )
        .await?),
        "gdpr" => Ok(crate::gdpr::process_gdpr(
            state,
            GdprCommand::from_interaction(data.into())?,
            invoker,
        )
        .await?),
        "leaderboard" => Ok(crate::levels::leaderboard(&state.root_url, guild_id)),
        _ => Err(Error::UnrecognizedCommand),
    }
}

async fn process_user_cmd(
    data: CommandData,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let msg_id = data.target_id.ok_or(Error::NoMessageTargetId)?;
    let user = data
        .resolved
        .as_ref()
        .ok_or(Error::NoResolvedData)?
        .users
        .get(&msg_id.cast())
        .ok_or(Error::NoTarget)?;
    crate::levels::get_level(guild_id, user.clone(), invoker, state).await
}

async fn process_msg_cmd(
    data: CommandData,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let msg_id = data.target_id.ok_or(Error::NoMessageTargetId)?;
    let user = &data
        .resolved
        .as_ref()
        .ok_or(Error::NoResolvedData)?
        .messages
        .get(&msg_id.cast())
        .ok_or(Error::NoTarget)?
        .author;
    crate::levels::get_level(guild_id, user.clone(), invoker, state).await
}
