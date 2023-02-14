use std::fmt::Display;

use twilight_interactions::command::{CreateOption, CommandOption};
use twilight_model::{
    channel::message::{Embed, MessageFlags},
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::{processor::CommandProcessorError, AppState};

pub async fn modify(
    toy: Toy,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    #[allow(clippy::cast_possible_wrap)]
    let xp = query!(
        "SELECT * FROM levels WHERE id = $1 AND guild = $2",
        invoker.id.get() as i64,
        guild_id.get() as i64
    )
    .fetch_one(&state.db)
    .await?
    .xp;
    #[allow(clippy::cast_sign_loss)]
    let level_info = mee6::LevelInfo::new(xp as u64);
    if level_info.level() < toy.level_requirement() {
        let embed = EmbedBuilder::new()
            .description(format!(
                "You need at least {} levels for {toy} (you have {})",
                toy.level_requirement(),
                level_info.level()
            ))
            .build();
        return Ok(ephemeral_embed_response(embed));
    }
    let embed = EmbedBuilder::new()
        .description(format!("Set your toy to {toy}"))
        .build();
    Ok(ephemeral_embed_response(embed))
}

#[derive(Clone, Copy, Debug, CreateOption, CommandOption)]
pub enum Toy {
    #[option(name = "Fox", value = "fox")]
    Fox,
    #[option(name = "Parrot", value = "parrot")]
    Parrot,
    #[option(name = "Grass Block", value = "grassblock")]
    GrassBlock,
    #[option(name = "Pickaxe", value = "pickaxe")]
    Pickaxe,
    #[option(name = "Steve heart", value = "steveheart")]
    SteveHeart,
    #[option(name = "Tree", value = "tree")]
    Tree,
}

impl Toy {
    pub const fn level_requirement(self) -> u64 {
        match self {
            Self::Pickaxe => 10,
            Self::Fox | Self::Parrot => 5,
            Self::GrassBlock | Self::SteveHeart | Self::Tree => 0,
        }
    }
}

impl Display for Toy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Fox => "Fox",
            Self::Parrot => "Parrot",
            Self::GrassBlock => "Grass block",
            Self::Pickaxe => "Pickaxe",
            Self::SteveHeart => "Steve heart",
            Self::Tree => "Tree",
        };
        f.write_str(text)
    }
}

fn ephemeral_embed_response(embed: Embed) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .embeds([embed])
                .flags(MessageFlags::EPHEMERAL)
                .build(),
        ),
    }
}
