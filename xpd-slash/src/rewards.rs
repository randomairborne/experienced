use std::fmt::Write;

use twilight_model::{
    channel::message::AllowedMentions,
    id::{marker::GuildMarker, Id},
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_slash_defs::rewards::{RewardsCommand, RewardsCommandAdd, RewardsCommandRemove};

use crate::{Error, SlashState, XpdSlashResponse};

pub async fn process_rewards(
    cmd: RewardsCommand,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = match cmd {
        RewardsCommand::Add(add) => process_rewards_add(add, state, guild_id).await,
        RewardsCommand::Remove(remove) => process_rewards_rm(remove, state, guild_id).await,
        RewardsCommand::List(_list) => process_rewards_list(state, guild_id).await,
    }?;
    Ok(XpdSlashResponse::new()
        .allowed_mentions(AllowedMentions::default())
        .ephemeral(true)
        .embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn process_rewards_add(
    options: RewardsCommandAdd,
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
    options: RewardsCommandRemove,
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
