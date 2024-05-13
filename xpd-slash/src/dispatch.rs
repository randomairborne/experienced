use twilight_interactions::command::CommandModel;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{application_command::CommandData, Interaction, InteractionData},
    },
    http::interaction::InteractionResponse,
    id::{marker::GuildMarker, Id},
    user::User,
};

use crate::{
    cmd_defs::{
        AdminCommand, CardCommand, GdprCommand, GuildCardCommand, LeaderboardCommand, XpCommand,
    },
    leaderboard::{process_message_component, process_modal_submit},
    Error, SlashState, XpdSlashResponse,
};

pub async fn process(
    interaction: Interaction,
    state: SlashState,
) -> Result<InteractionResponse, Error> {
    trace!(?interaction, "got interaction");
    let Some(data) = interaction.data else {
        return Err(Error::NoInteractionData);
    };
    let invoker = match interaction.member {
        Some(val) => val.user,
        None => interaction.user,
    }
    .ok_or(Error::NoInvoker)?;
    let guild_id = interaction.guild_id;
    match data {
        InteractionData::ApplicationCommand(cmd) => {
            process_app_cmd(state, *cmd, invoker, guild_id).await
        }
        InteractionData::MessageComponent(mcd) => {
            process_message_component(mcd, guild_id.ok_or(Error::NoGuildId)?, state).await
        }
        InteractionData::ModalSubmit(mid) => {
            process_modal_submit(mid, guild_id.ok_or(Error::NoGuildId)?, state).await
        }
        _ => Err(Error::NoInteractionData),
    }
}

async fn process_app_cmd(
    state: SlashState,
    data: CommandData,
    invoker: User,
    guild_id: Option<Id<GuildMarker>>,
) -> Result<InteractionResponse, Error> {
    match data.kind {
        CommandType::ChatInput => process_slash_cmd(data, guild_id, invoker, state).await,
        CommandType::User => {
            process_user_cmd(data, guild_id.ok_or(Error::NoGuildId)?, invoker, state)
                .await
                .map(Into::into)
        }
        CommandType::Message => {
            process_msg_cmd(data, guild_id.ok_or(Error::NoGuildId)?, invoker, state)
                .await
                .map(Into::into)
        }
        _ => Err(Error::WrongInteractionData),
    }
}

async fn process_slash_cmd(
    data: CommandData,
    guild_id: Option<Id<GuildMarker>>,
    invoker: User,
    state: SlashState,
) -> Result<InteractionResponse, Error> {
    match data.name.as_str() {
        "help" => Ok(crate::help::help().into()),
        "rank" => {
            let target = crate::cmd_defs::RankCommand::from_interaction(data.into())?
                .user
                .map_or_else(|| invoker.clone(), |v| v.resolved);
            crate::levels::get_level(guild_id.ok_or(Error::NoGuildId)?, target, invoker.id, state)
                .await
                .map(Into::into)
        }
        "xp" => crate::manager::process_xp(
            XpCommand::from_interaction(data.into())?,
            guild_id.ok_or(Error::NoGuildId)?,
            state,
        )
        .await
        .map(Into::into),
        "admin" => crate::admin::process_admin(
            AdminCommand::from_interaction(data.into())?,
            guild_id.ok_or(Error::NoGuildId)?,
            invoker.id,
            state,
        )
        .await
        .map(Into::into),
        "card" => crate::manage_card::user_card_update(
            CardCommand::from_interaction(data.into())?,
            invoker,
            &state,
            guild_id,
        )
        .await
        .map(Into::into),
        "guild-card" => Ok(crate::manage_card::guild_card_update(
            GuildCardCommand::from_interaction(data.into())?,
            &state,
            guild_id.ok_or(Error::NoGuildId)?,
        )
        .await?
        .into()),
        "gdpr" => {
            crate::gdpr::process_gdpr(state, GdprCommand::from_interaction(data.into())?, invoker)
                .await
                .map(Into::into)
        }
        "leaderboard" => crate::leaderboard::leaderboard(
            state,
            guild_id.ok_or(Error::NoGuildId)?,
            LeaderboardCommand::from_interaction(data.into())?,
        )
        .await
        .map(Into::into),
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
    crate::levels::get_level(guild_id, user.clone(), invoker.id, state).await
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
    crate::levels::get_level(guild_id, user.clone(), invoker.id, state).await
}
