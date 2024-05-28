use std::fmt::Write;

use http_body_util::{BodyExt, Limited};
use serde::{Deserialize, Serialize};
use sqlx::Executor;
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
use xpd_common::{db_to_id, id_to_db, GuildConfig};

use crate::{
    cmd_defs::{
        manage::{
            XpCommandExperience, XpCommandRewards, XpCommandRewardsAdd, XpCommandRewardsConfig,
            XpCommandRewardsRemove,
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
    match data {
        XpCommandExperience::Import(import) => {
            import_level_data(state, respondable, guild_id, import.levels).await
        }
        XpCommandExperience::Export(_) => export_level_data(state, respondable, guild_id).await,
        XpCommandExperience::Add(add) => {
            modify_user_xp(guild_id, add.user, add.amount, state).await
        }
        XpCommandExperience::Remove(rm) => {
            modify_user_xp(guild_id, rm.user, -rm.amount, state).await
        }
        XpCommandExperience::Reset(rst) => reset_user_xp(guild_id, rst.user, state).await,
    }
}

async fn modify_user_xp(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    amount: i64,
    state: SlashState,
) -> Result<String, Error> {
    let mut txn = state.db.begin().await?;
    let xp = query!(
        "UPDATE levels SET xp = xp + $3 WHERE id = $1 AND guild = $2 RETURNING xp",
        id_to_db(user_id),
        id_to_db(guild_id),
        amount
    )
    .fetch_one(txn.as_mut())
    .await?
    .xp;
    if xp.is_negative() {
        txn.rollback().await?;
        return Err(Error::XpWouldBeNegative);
    }
    let current_level = mee6::LevelInfo::new(u64::try_from(xp).unwrap_or(0)).level();
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
    query!(
        "DELETE FROM levels WHERE id = $1 AND guild = $2",
        id_to_db(user_id),
        id_to_db(guild_id)
    )
    .execute(&state.db)
    .await?;
    Ok(format!(
        "Deleted <@{user_id}> from my database in this server!"
    ))
}

#[derive(Deserialize, Serialize)]
pub struct ImportUser {
    id: Id<UserMarker>,
    xp: i64,
}

#[allow(clippy::unused_async)]
async fn export_level_data(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    tokio::spawn(background_data_operation_wrapper(
        state,
        respondable,
        guild_id,
        None,
    ));
    Ok("Exporting level data, check back soon!".to_string())
}

async fn background_data_export(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<XpdSlashResponse, Error> {
    let levels: Vec<ImportUser> = query!(
        "SELECT id, xp FROM levels WHERE guild = $1",
        id_to_db(guild_id)
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|v| ImportUser {
        id: db_to_id(v.id),
        xp: v.xp,
    })
    .collect();
    let file = serde_json::to_vec_pretty(&levels)?;
    let attachment = HttpAttachment::from_bytes(format!("export-{guild_id}.json"), file, 0);
    Ok(XpdSlashResponse::new()
        .content("Exported your level data!".to_string())
        .attachments([attachment]))
}

#[allow(clippy::unused_async)]
async fn import_level_data(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    attachment: Attachment,
) -> Result<String, Error> {
    tokio::spawn(background_data_operation_wrapper(
        state,
        respondable,
        guild_id,
        Some(attachment),
    ));
    Ok("Importing level data, check back soon!".to_string())
}

const MAX_IMPORT_SIZE: usize = 1024 * 1024 * 10;

async fn background_data_import(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
    attachment: Attachment,
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

    let mut txn = state.db.begin().await?;

    let db_guild = id_to_db(guild_id);
    for user in &data {
        let query = query!(
            "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) \
                ON CONFLICT (id, guild) \
                DO UPDATE SET xp=levels.xp+excluded.xp",
            id_to_db(user.id),
            user.xp,
            db_guild,
        );
        if let Err(err) = txn.execute(query).await {
            txn.rollback().await?;
            return Err(err.into());
        };
    }

    txn.commit().await?;

    let seconds = start.elapsed().as_secs_f64();
    let users = data.len();
    Ok(XpdSlashResponse::with_embed_text(format!(
        "Imported XP data for {users} users in {seconds:.2} seconds!"
    )))
}

async fn background_data_operation_wrapper(
    state: SlashState,
    respondable: Respondable,
    guild_id: Id<GuildMarker>,
    attachment: Option<Attachment>,
) {
    let xsr = if let Some(attachment) = attachment {
        background_data_import(&state, guild_id, attachment)
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
        XpCommandRewards::Config(config) => process_rewards_config(state, guild_id, config).await,
        XpCommandRewards::ResetConfig(_) => process_rewards_reset_config(state, guild_id).await,
    }
}

async fn process_rewards_add(
    options: XpCommandRewardsAdd,
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    query!(
        "INSERT INTO role_rewards (id, requirement, guild) VALUES ($1, $2, $3)",
        id_to_db(options.role.id),
        options.level,
        id_to_db(guild_id)
    )
    .execute(&state.db)
    .await?;
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
    if let Some(role) = options.role {
        query!(
            "DELETE FROM role_rewards WHERE id = $1 AND guild = $2",
            id_to_db(role),
            id_to_db(guild_id)
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward <@&{role}>!"));
    } else if let Some(level) = options.level {
        query!(
            "DELETE FROM role_rewards WHERE requirement = $1 AND guild = $2",
            level,
            id_to_db(guild_id)
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward for level {level}!"));
    };
    state.invalidate_rewards(guild_id).await;
    Err(Error::WrongArgumentCount(
        "`/xp rewards remove` requires either a level or a role!",
    ))
}
async fn process_rewards_list(
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    let mut roles = query!(
        "SELECT id, requirement FROM role_rewards WHERE guild = $1",
        id_to_db(guild_id)
    )
    .fetch_all(&state.db)
    .await?;
    let mut data = String::new();

    roles.sort_by(|a, b| a.requirement.cmp(&b.requirement));

    for role in roles {
        writeln!(
            data,
            "Role reward <@&{}> at level {}",
            role.id, role.requirement
        )?;
    }
    if data.is_empty() {
        data = "No role rewards set for this server".to_string();
    }
    Ok(data)
}

async fn process_rewards_config(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    options: XpCommandRewardsConfig,
) -> Result<String, Error> {
    let config = query_as!(
        GuildConfig,
        "INSERT INTO guild_configs (id, one_at_a_time) VALUES ($1, $2) \
            ON CONFLICT (id) DO UPDATE SET \
            one_at_a_time = COALESCE($2, excluded.one_at_a_time)\
            RETURNING one_at_a_time",
        id_to_db(guild_id),
        options.one_at_a_time
    )
    .fetch_one(&state.db)
    .await?;
    state.update_config(guild_id, config).await;
    Ok("Updated guild config!".to_string())
}

async fn process_rewards_reset_config(
    state: SlashState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    query!(
        "DELETE FROM guild_configs WHERE id = $1",
        id_to_db(guild_id)
    )
    .execute(&state.db)
    .await?;
    state.update_config(guild_id, GuildConfig::default()).await;
    Ok("Reset guild reward config, but NOT rewards themselves!".to_string())
}
