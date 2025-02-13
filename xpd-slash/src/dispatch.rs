use twilight_interactions::command::CommandModel;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{
            application_command::CommandData, Interaction, InteractionData, InteractionType,
        },
    },
    id::{
        marker::{GuildMarker, InteractionMarker},
        Id,
    },
};
use xpd_common::MemberDisplayInfo;
use xpd_slash_defs::{
    admin::AdminCommand,
    audit::AuditLogCommand,
    card::{CardCommand, GuildCardCommand},
    config::ConfigCommand,
    experience::XpCommand,
    gdpr::GdprCommand,
    levels::{LeaderboardCommand, RankCommand},
    manage::ManageCommand,
    rewards::RewardsCommand,
};

use crate::{
    experience::XpAuditData,
    leaderboard::{process_message_component, process_modal_submit},
    response::XpdInteractionResponse,
    Error, SlashState,
};

#[derive(Clone, Debug)]
pub struct Respondable {
    token: String,
    id: Id<InteractionMarker>,
}

impl Respondable {
    #[allow(clippy::missing_const_for_fn)]
    pub fn token(&self) -> &str {
        &self.token
    }

    pub const fn id(&self) -> Id<InteractionMarker> {
        self.id
    }
}

pub async fn process(
    interaction: Interaction,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    trace!(?interaction, "got interaction");
    let respondable = Respondable {
        token: interaction.token.clone(),
        id: interaction.id,
    };
    let Some(data) = interaction.data else {
        return Err(Error::NoInteractionData);
    };

    if matches!(
        interaction.kind,
        InteractionType::ApplicationCommandAutocomplete
    ) {
        return if let InteractionData::ApplicationCommand(data) = data {
            Ok(crate::autocomplete::autocomplete(&state, *data))
        } else {
            Err(Error::WrongInteractionData)
        };
    }

    let invoker: MemberDisplayInfo = match interaction.member {
        Some(member) => MemberDisplayInfo::from_partial_member(member),
        None => interaction.user.map(MemberDisplayInfo::from),
    }
    .ok_or(Error::NoInvoker)?;

    let guild_id = interaction.guild_id;
    match data {
        InteractionData::ApplicationCommand(cmd) => {
            process_app_cmd(state, *cmd, respondable, invoker, guild_id).await
        }
        InteractionData::MessageComponent(mcd) => {
            let Some(original_msg) = interaction.message else {
                return Err(Error::NoInteractionMessage);
            };
            process_message_component(
                *mcd,
                original_msg,
                guild_id.ok_or(Error::NoGuildId)?,
                invoker.id,
                state,
                respondable,
            )
            .await
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
    respondable: Respondable,
    invoker: MemberDisplayInfo,
    guild_id: Option<Id<GuildMarker>>,
) -> Result<XpdInteractionResponse, Error> {
    match data.kind {
        CommandType::ChatInput => {
            process_slash_cmd(data, guild_id, respondable, invoker, state).await
        }
        CommandType::User => {
            process_user_cmd(data, guild_id.ok_or(Error::NoGuildId)?, invoker, state).await
        }
        CommandType::Message => {
            process_msg_cmd(data, guild_id.ok_or(Error::NoGuildId)?, invoker, state).await
        }
        _ => Err(Error::WrongInteractionData),
    }
}

#[allow(clippy::too_many_lines)]
async fn process_slash_cmd(
    data: CommandData,
    guild_id: Option<Id<GuildMarker>>,
    respondable: Respondable,
    invoker: MemberDisplayInfo,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    match data.name.as_str() {
        "help" => Ok(crate::help::help()),
        "rank" => {
            let data = RankCommand::from_interaction(data.into())?;
            let target = data.user.map_or_else(
                || invoker.clone(),
                |ru| {
                    let (nick, local_avatar) = ru
                        .member
                        .map_or_else(|| (None, None), |im| (im.nick, im.avatar));
                    MemberDisplayInfo {
                        id: ru.resolved.id,
                        name: ru.resolved.name,
                        global_name: ru.resolved.global_name,
                        nick,
                        avatar: ru.resolved.avatar,
                        local_avatar,
                        bot: ru.resolved.bot,
                    }
                },
            );
            crate::levels::get_level(
                guild_id.ok_or(Error::NoGuildId)?,
                target,
                invoker.id,
                data.show_off,
                state,
            )
            .await
        }
        "xp" => {
            crate::experience::process_xp(
                XpCommand::from_interaction(data.into())?,
                state,
                guild_id.ok_or(Error::NoGuildId)?,
                XpAuditData {
                    interaction: respondable.id,
                    invoker: invoker.id,
                },
            )
            .await
        }
        "config" => {
            crate::config::process_config(
                ConfigCommand::from_interaction(data.into())?,
                guild_id.ok_or(Error::NoGuildId)?,
                state,
            )
            .await
        }
        "admin" => {
            crate::admin::process_admin(
                AdminCommand::from_interaction(data.into())?,
                guild_id.ok_or(Error::NoGuildId)?,
                invoker.id,
                state,
            )
            .await
        }
        "audit" => {
            crate::audit::process_audit_logs(
                AuditLogCommand::from_interaction(data.into())?,
                guild_id.ok_or(Error::NoGuildId)?,
                state,
            )
            .await
        }
        "card" => {
            crate::manage_card::user_card_update(
                CardCommand::from_interaction(data.into())?,
                invoker,
                &state,
                guild_id,
            )
            .await
        }
        "guild-card" => Ok(crate::manage_card::guild_card_update(
            GuildCardCommand::from_interaction(data.into())?,
            &state,
            guild_id.ok_or(Error::NoGuildId)?,
        )
        .await?),
        "gdpr" => {
            crate::gdpr::process_gdpr(state, GdprCommand::from_interaction(data.into())?, invoker)
                .await
        }
        "leaderboard" => {
            crate::leaderboard::leaderboard(
                state,
                guild_id.ok_or(Error::NoGuildId)?,
                LeaderboardCommand::from_interaction(data.into())?,
            )
            .await
        }
        "manage" => {
            crate::manager::process_manage(
                ManageCommand::from_interaction(data.into())?,
                guild_id.ok_or(Error::NoGuildId)?,
                respondable,
                state,
            )
            .await
        }
        "rewards" => {
            crate::rewards::process_rewards(
                RewardsCommand::from_interaction(data.into())?,
                guild_id.ok_or(Error::NoGuildId)?,
                state,
            )
            .await
        }
        _ => Err(Error::UnrecognizedCommand),
    }
}

const DEFAULT_SHOWOFF: Option<bool> = None;

async fn process_user_cmd(
    data: CommandData,
    guild_id: Id<GuildMarker>,
    invoker: MemberDisplayInfo,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    let msg_id = data.target_id.ok_or(Error::NoMessageTargetId)?;
    let resolved = data.resolved.as_ref().ok_or(Error::NoResolvedData)?;
    let user = resolved
        .users
        .get(&msg_id.cast())
        .ok_or(Error::NoTarget)?
        .clone();

    let nick = resolved.members.get(&user.id).and_then(|v| v.nick.clone());
    let target = MemberDisplayInfo::from(user).with_nick(nick);

    crate::levels::get_level(guild_id, target, invoker.id, DEFAULT_SHOWOFF, state).await
}

async fn process_msg_cmd(
    data: CommandData,
    guild_id: Id<GuildMarker>,
    invoker: MemberDisplayInfo,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    let msg_id = data.target_id.ok_or(Error::NoMessageTargetId)?;
    let resolved = &data.resolved.as_ref().ok_or(Error::NoResolvedData)?;
    let user = resolved
        .messages
        .get(&msg_id.cast())
        .ok_or(Error::NoTarget)?
        .author
        .clone();

    let nick = resolved.members.get(&user.id).and_then(|v| v.nick.clone());
    let target = MemberDisplayInfo::from(user).with_nick(nick);

    crate::levels::get_level(guild_id, target, invoker.id, DEFAULT_SHOWOFF, state).await
}
