use std::fmt::Write;
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};
use twilight_util::builder::embed::EmbedBuilder;

use crate::{
    cmd_defs::{
        AdminCommand,
        admin::{AdminCommandBanUser, AdminCommandLeave}
    },
    Error, SlashState, XpdSlashResponse,
};

pub async fn process_xp(
    data: AdminCommand,
    interaction_token: String,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdSlashResponse, Error> {
    let contents = match data {
        AdminCommand::Leave(leave) => {}
        AdminCommand::ResetGuild(reset_guild) => {}
        AdminCommand::ResetUser(reset_user) => {}
        AdminCommand::SetNick(set_nick) => {}
        AdminCommand::BanUser(ban_user) => {}
        AdminCommand::BanGuild(ban_guild) => {}
    }?;
    Ok(XpdSlashResponse::new().embeds([EmbedBuilder::new().description(contents).build()]))
}

async fn leave()