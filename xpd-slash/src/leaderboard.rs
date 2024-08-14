use std::{convert::TryInto, fmt::Write};

use twilight_model::{
    application::interaction::{
        message_component::MessageComponentInteractionData, modal::ModalInteractionData,
    },
    channel::{
        message::{
            component::{ActionRow, Button, ButtonStyle, TextInput, TextInputStyle},
            Component, MessageFlags, ReactionType,
        },
        Message,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::{
    embed::{EmbedBuilder, EmbedFooterBuilder},
    InteractionResponseDataBuilder,
};
use xpd_common::id_to_db;

use crate::{cmd_defs::LeaderboardCommand, Error, SlashState};

pub async fn leaderboard(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    guild_command: LeaderboardCommand,
) -> Result<InteractionResponse, Error> {
    // "zpage" means "zero-indexed page", which is how this is represented internally.
    // We add one whenever we show it to the user, and subtract one every time we get it from the user.
    let zpage = if let Some(pick) = guild_command.page {
        pick - 1
    } else if let Some(pick) = guild_command.user {
        state.get_user_stats(pick.resolved.id, guild_id).await?.rank / 10
    } else {
        0
    };
    Ok(InteractionResponse {
        data: Some(gen_leaderboard(guild_id, state.db, zpage, guild_command.show_off).await?),
        kind: InteractionResponseType::ChannelMessageWithSource,
    })
}

const USERS_PER_PAGE_USIZE: usize = 10;
#[allow(clippy::cast_possible_wrap)]
const USERS_PER_PAGE: i64 = USERS_PER_PAGE_USIZE as i64;

async fn gen_leaderboard(
    guild_id: Id<GuildMarker>,
    db: sqlx::PgPool,
    zpage: i64,
    show_off: Option<bool>,
) -> Result<InteractionResponseData, Error> {
    if zpage.is_negative() {
        return Err(Error::PageDoesNotExist);
    }
    let users = query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT $2 OFFSET $3",
        id_to_db(guild_id),
        USERS_PER_PAGE + 1,
        zpage * USERS_PER_PAGE
    )
        .fetch_all(&db)
        .await?;
    if users.is_empty() {
        return Err(Error::NoUsersForPage);
    }
    let one_more_page_bro = users.len() >= (USERS_PER_PAGE_USIZE + 1);
    let last_user_idx = users.len().clamp(0, USERS_PER_PAGE_USIZE);
    let users = &users[0..last_user_idx];
    // this is kinda the only way to do this
    // It's designed to only allocate once, at the start here
    let mut description = String::with_capacity(users.len() * 128);
    for (i, user) in users.iter().enumerate() {
        let level = mee6::LevelInfo::new(user.xp.try_into().unwrap_or(0)).level();
        let rank: i64 = i
            .try_into()
            .map_or(-1, |v: i64| v + (zpage * USERS_PER_PAGE) + 1);
        writeln!(description, "**#{rank}.** <@{}> - Level {level}", user.id).ok();
    }
    if description.is_empty() {
        description += "Nobody is ranked yet.";
    }
    let embed = EmbedBuilder::new()
        .description(description)
        .footer(EmbedFooterBuilder::new(format!("Page {}", zpage + 1)).build())
        .color(crate::THEME_COLOR)
        .build();
    let back_button = Component::Button(Button {
        custom_id: Some((zpage - 1).to_string()),
        disabled: zpage == 0,
        emoji: Some(ReactionType::Unicode {
            name: "⬅".to_string(),
        }),
        label: Some("Previous".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let select_button = Component::Button(Button {
        custom_id: Some("jump_modal".to_string()),
        disabled: !one_more_page_bro && zpage == 0,
        emoji: None,
        label: Some("Go to page".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let forward_button = Component::Button(Button {
        custom_id: Some((zpage + 1).to_string()),
        disabled: !one_more_page_bro,
        emoji: Some(ReactionType::Unicode {
            name: "➡️".to_string(),
        }),
        label: Some("Next".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let flags = if show_off.is_some_and(|v| v) {
        MessageFlags::empty()
    } else {
        MessageFlags::EPHEMERAL
    };
    Ok(InteractionResponseDataBuilder::new()
        .components([Component::ActionRow(ActionRow {
            components: vec![back_button, select_button, forward_button],
        })])
        .embeds([embed])
        .flags(flags)
        .build())
}

pub async fn process_modal_submit(
    data: ModalInteractionData,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<InteractionResponse, Error> {
    // You can't get this modal unless you are the triggering user
    let actions = data.components.first().ok_or(Error::NoModalActionRow)?;
    let field = actions.components.first().ok_or(Error::NoFormField)?;
    let choice: i64 = field
        .value
        .as_ref()
        .ok_or(Error::NoDestinationInComponent)?
        .parse()?;
    let zpage = choice - 1;
    Ok(InteractionResponse {
        kind: InteractionResponseType::UpdateMessage,
        data: Some(gen_leaderboard(guild_id, state.db, zpage, Some(true)).await?),
    })
}

pub async fn process_message_component(
    data: MessageComponentInteractionData,
    original_message: Message,
    guild_id: Id<GuildMarker>,
    invoker_id: Id<UserMarker>,
    state: SlashState,
) -> Result<InteractionResponse, Error> {
    if original_message
        .interaction
        .ok_or(Error::NoInteractionInvocationOnInteractionMessage)?
        .user
        .id
        != invoker_id
    {
        return Err(Error::NotYourLeaderboard);
    }
    if data.custom_id == "jump_modal" {
        let input = TextInput {
            custom_id: "jump_modal_input".to_string(),
            label: "Jump Destination".to_string(),
            max_length: Some(8),
            min_length: Some(1),
            placeholder: Some("What page to jump to".to_string()),
            required: Some(true),
            style: TextInputStyle::Short,
            value: None,
        };
        return Ok(InteractionResponse {
            kind: InteractionResponseType::Modal,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .components([Component::ActionRow(ActionRow {
                        components: vec![Component::TextInput(input)],
                    })])
                    .custom_id("jump_modal")
                    .title("Go to page..")
                    .build(),
            ),
        });
    }
    // when we create the buttons, we set next and previous's custom IDs to the current page
    // plus and minus 1. This means that we don't have to store which page which
    // message is on, because the component will tell us exactly where it wants to go!
    let offset: i64 = data.custom_id.parse()?;
    Ok(InteractionResponse {
        kind: InteractionResponseType::UpdateMessage,
        data: Some(gen_leaderboard(guild_id, state.db, offset, Some(true)).await?),
    })
}
