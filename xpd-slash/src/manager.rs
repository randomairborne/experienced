use std::fmt::Write;

use http_body_util::{BodyExt, Limited};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use twilight_model::{
    channel::{message::AllowedMentions, Attachment},
    http::attachment::Attachment as HttpAttachment,
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::embed::EmbedBuilder;

use crate::{
    cmd_defs::{
        manage::{
            XpCommandExperience, XpCommandRewards, XpCommandRewardsAdd, XpCommandRewardsRemove,
        },
        XpCommand,
    },
    dispatch::Respondable,
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_xp(
    data: XpCommand,
    guild_id: Id<GuildMarker>,
    respondable: Respondable,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = match data {
        XpCommand::Rewards(rewards) => process_rewards(rewards, guild_id, state).await,
        XpCommand::Experience(experience) => {
            process_experience(experience, respondable, guild_id, state).await
        }
    }?;
    Ok(XpdSlashResponse::new()
        .allowed_mentions_o(Some(AllowedMentions::default()))
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn process_experience(
    data: XpCommandExperience,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<String, Error> {
    if !allowed_command_for_target(&data) {
        return Err(Error::BotsDontLevel);
    }
    match data {
        XpCommandExperience::Import(import) => import_level_data(
            state,
            respondable,
            guild_id,
            import.levels,
            import.overwrite.unwrap_or(false),
        ),
        XpCommandExperience::Export(_) => export_level_data(state, respondable, guild_id),
        XpCommandExperience::Add(add) => {
            modify_user_xp(guild_id, add.user.resolved.id, add.amount, state).await
        }
        XpCommandExperience::Remove(rm) => {
            modify_user_xp(guild_id, rm.user.resolved.id, -rm.amount, state).await
        }
        XpCommandExperience::Reset(rst) => {
            reset_user_xp(guild_id, rst.user.resolved.id, state).await
        }
        XpCommandExperience::Set(st) => {
            set_user_xp(guild_id, st.user.resolved.id, st.xp, state).await
        }
        XpCommandExperience::ResetGuild(rst) => {
            reset_guild_xp(guild_id, rst.confirm_message, state).await
        }
    }
}

/// For commands that target a specific user, other than reset, prevent commands from being used on a bot.
const fn allowed_command_for_target(data: &XpCommandExperience) -> bool {
    match data {
        XpCommandExperience::Add(add) => !add.user.resolved.bot,
        XpCommandExperience::Remove(rm) => !rm.user.resolved.bot,
        XpCommandExperience::Set(set) => !set.user.resolved.bot,
        XpCommandExperience::Import(_)
        | XpCommandExperience::Export(_)
        | XpCommandExperience::Reset(_)
        | XpCommandExperience::ResetGuild(_) => true,
    }
}

async fn modify_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    amount: i64,
    state: SlashState,
) -> Result<String, Error> {
    let mut txn = state.db.begin().await?;
    let xp = xpd_database::add_xp(txn.as_mut(), user_id, guild_id, amount).await?;
    if xp.is_negative() {
        txn.rollback().await?;
        return Err(Error::XpWouldBeNegative);
    }
    txn.commit().await?;
    let current_level = mee6::LevelInfo::new(xp.try_into().unwrap_or(0)).level();
    let (action, targeter) = if amount.is_positive() {
        ("Added", "to")
    } else {
        ("Removed", "from")
    };
    let amount_abs = amount.abs();
    Ok(format!("{action} {amount_abs} XP {targeter} <@{user_id}>, leaving them with {xp} XP at level {current_level}"))
}

async fn reset_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    state: SlashState,
) -> Result<String, Error> {
    xpd_database::clear_xp(&state.db, user_id, guild_id).await?;
    Ok(format!(
        "Deleted <@{user_id}> from my database in this server!"
    ))
}

async fn set_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    setpoint: i64,
    state: SlashState,
) -> Result<String, Error> {
    xpd_database::set_xp(&state.db, user_id, guild_id, setpoint).await?;
    let level = mee6::LevelInfo::new(setpoint.try_into().unwrap_or(0));
    Ok(format!(
        "Set <@{user_id}>'s XP to {}, leaving them at level {}",
        level.xp(),
        level.level()
    ))
}

#[derive(Deserialize, Serialize)]
pub struct ImportUser {
    id: Id<UserMarker>,
    xp: i64,
}

#[allow(clippy::unnecessary_wraps)]
fn export_level_data(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    state
        .task_tracker
        .clone()
        .spawn(background_data_operation_wrapper(
            state,
            respondable,
            guild_id,
            None,
            false,
        ));
    Ok("Exporting level data, check back soon!".to_string())
}

async fn background_data_export(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    let levels: Vec<ImportUser> = xpd_database::export_bulk_users(&state.db, guild_id)
        .await?
        .iter()
        .map(|us| ImportUser {
            id: us.id,
            xp: us.xp,
        })
        .collect();
    let file = serde_json::to_vec_pretty(&levels)?;
    let attachment = HttpAttachment::from_bytes(format!("export-{guild_id}.json"), file, 0);
    Ok(XpdSlashResponse::new()
        .content("Exported your level data!".to_string())
        .attachments([attachment]))
}

#[allow(clippy::unnecessary_wraps)]
fn import_level_data(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    attachment: Attachment,
    overwrite: bool,
) -> Result<String, Error> {
    state.clone().spawn(background_data_operation_wrapper(
        state,
        respondable,
        guild_id,
        Some(attachment),
        overwrite,
    ));
    Ok("Importing level data, check back soon!".to_string())
}

const MAX_IMPORT_SIZE: usize = 1024 * 1024 * 10;

async fn background_data_import(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
    attachment: Attachment,
    overwrite: bool,
) -> Result<XpdSlashResponse, Error> {
    let start = Instant::now();

    let request = state.http.get(attachment.url).send().await?;
    request.error_for_status_ref()?;

    let raw_body = reqwest::Body::from(request);
    let body = Limited::new(raw_body, MAX_IMPORT_SIZE)
        .collect()
        .await
        .map_err(|_| Error::RawHttpBody)?
        .to_bytes();

    let data: Vec<ImportUser> = serde_json::from_slice(&body)?;
    let user_count = data.len();
    let mut txn = state.db.begin().await?;
    for user in data {
        if overwrite {
            xpd_database::set_xp(txn.as_mut(), user.id, guild_id, user.xp).await?;
        } else {
            xpd_database::add_xp(txn.as_mut(), user.id, guild_id, user.xp).await?;
        }
    }

    txn.commit().await?;

    let seconds = start.elapsed().as_secs_f64();
    Ok(XpdSlashResponse::with_embed_text(format!(
        "Imported XP data for {user_count} users in {seconds:.2} seconds!"
    )))
}

async fn background_data_operation_wrapper(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    attachment: Option<Attachment>,
    overwrite: bool,
) {
    let xsr = if let Some(attachment) = attachment {
        background_data_import(&state, guild_id, attachment, overwrite)
            .await
            .unwrap_or_else(|source| {
                error!(?source, "Failed to import level data");
                XpdSlashResponse::with_embed_text(format!("Failed to import level data: {source}"))
            })
    } else {
        background_data_export(&state, guild_id)
            .await
            .unwrap_or_else(|source| {
                error!(?source, "Failed to export level data");
                XpdSlashResponse::with_embed_text(format!("Failed to export level data: {source}"))
            })
    }
    .ephemeral(true);
    state.send_followup(xsr, respondable.token()).await;
}

async fn process_rewards(
    cmd: XpCommandRewards,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<String, Error> {
    match cmd {
        XpCommandRewards::Add(add) => process_rewards_add(add, state, guild_id).await,
        XpCommandRewards::Remove(remove) => process_rewards_rm(remove, state, guild_id).await,
        XpCommandRewards::List(_list) => process_rewards_list(state, guild_id).await,
    }
}

async fn process_rewards_add(
    options: XpCommandRewardsAdd,
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    xpd_database::add_reward_role(&state.db, guild_id, options.level, options.role.id).await?;
    state.invalidate_rewards(guild_id).await;
    Ok(format!(
        "Added role reward <@&{}> at level {}!",
        options.role.id, options.level
    ))
}

async fn process_rewards_rm(
    options: XpCommandRewardsRemove,
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    match xpd_database::delete_reward_role(&state.db, guild_id, options.level, options.role).await {
        Ok(count) => {
            state.invalidate_rewards(guild_id).await;
            let pluralizer = if count == 1 { "" } else { "s" };
            Ok(format!("Deleted {count} role reward{pluralizer}."))
        }
        Err(xpd_database::Error::UnspecifiedDelete) => Err(Error::WrongArgumentCount(
            "`/xp rewards remove` requires either a level or a role!",
        )),
        Err(e) => Err(e.into()),
    }
}

async fn process_rewards_list(
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    let mut roles = xpd_database::guild_rewards(&state.db, guild_id).await?;
    if roles.is_empty() {
        return Ok("No role rewards set for this server".to_string());
    }
    let mut data = String::new();

    roles.sort_by(|a, b| a.requirement.cmp(&b.requirement));

    for role in roles {
        writeln!(
            data,
            "Role reward <@&{}> at level {}",
            role.id, role.requirement
        )?;
    }
    Ok(data)
}

async fn reset_guild_xp(
    guild_id: Id<GuildMarker>,
    confirmation: String,
    state: SlashState,
) -> Result<String, Error> {
    if confirmation != crate::cmd_defs::manage::CONFIRMATION_STRING {
        return Ok("Confirmation string did not match.".to_string());
    }
    xpd_database::delete_levels_guild(&state.db, guild_id).await?;
    Ok("Done. Thank you for using Experienced.".to_string())
}
