use std::{convert::TryInto, fmt::Write};

use twilight_model::{
    application::interaction::{
        message_component::MessageComponentInteractionData, modal::ModalInteractionData,
    },
    channel::{
        message::{
            component::{ActionRow, Button, ButtonStyle, TextInput, TextInputStyle},
            AllowedMentions, Component, MessageFlags, ReactionType,
        },
        Message,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;
use xpd_slash_defs::levels::LeaderboardCommand;

use crate::{Error, SlashState};

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
        data: Some(gen_leaderboard(&state, guild_id, zpage, guild_command.show_off).await?),
        kind: InteractionResponseType::ChannelMessageWithSource,
    })
}

const USERS_PER_PAGE_USIZE: usize = 10;
#[allow(clippy::cast_possible_wrap)]
const USERS_PER_PAGE: i64 = USERS_PER_PAGE_USIZE as i64;

async fn gen_leaderboard(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
    zpage: i64,
    show_off: Option<bool>,
) -> Result<InteractionResponseData, Error> {
    if zpage.is_negative() {
        return Err(Error::PageDoesNotExist);
    }
    let is_ephemeral = !show_off.is_some_and(|v| v);
    let users = xpd_database::get_leaderboard_page(
        &state.db,
        guild_id,
        USERS_PER_PAGE + 1,
        zpage * USERS_PER_PAGE,
    )
    .await?;

    if users.is_empty() {
        return Err(if zpage == 0 {
            Error::NoUsersForPage
        } else {
            Error::NoRanksYet
        });
    }

    let one_more_page_bro = users.len() >= (USERS_PER_PAGE_USIZE + 1);
    let last_user_idx = users.len().clamp(0, USERS_PER_PAGE_USIZE);
    let users = &users[0..last_user_idx];
    // this is kinda the only way to do this
    // It's designed to only allocate once, at the start here
    let mut description = String::with_capacity(256 + users.len() * 128);
    writeln!(description, "### Leaderboard")?;
    for (i, user) in users.iter().enumerate() {
        let level = mee6::LevelInfo::new(user.xp.try_into().unwrap_or(0)).level();
        let rank: i64 = i
            .try_into()
            .map_or(-1, |v: i64| v + (zpage * USERS_PER_PAGE) + 1);
        writeln!(description, "**#{rank}.** <@{}> - Level {level}", user.id)?;
    }

    let control_options = control_options(zpage, one_more_page_bro);

    let (components, flags) = if is_ephemeral {
        let second_last_idx = control_options.len() - 2;
        (
            &control_options[..=second_last_idx],
            MessageFlags::EPHEMERAL,
        )
    } else {
        (control_options.as_slice(), MessageFlags::empty())
    };

    let components = Component::ActionRow(ActionRow {
        components: components.to_vec(),
    });

    Ok(InteractionResponseDataBuilder::new()
        .allowed_mentions(AllowedMentions::default())
        .components([components])
        .content(description)
        .flags(flags)
        .build())
}

fn control_options(zpage: i64, next_page_exists: bool) -> [Component; 5] {
    [
        Button {
            custom_id: Some("page_indicator".to_string()),
            disabled: true,
            emoji: None,
            label: Some(format!("Page {}", zpage + 1)),
            style: ButtonStyle::Secondary,
            url: None,
        },
        Button {
            custom_id: Some((zpage - 1).to_string()),
            disabled: zpage == 0,
            emoji: Some(ReactionType::Unicode {
                name: "⬅".to_string(),
            }),
            label: Some("Previous".to_string()),
            style: ButtonStyle::Primary,
            url: None,
        },
        Button {
            custom_id: Some("jump_modal".to_string()),
            disabled: !next_page_exists && zpage == 0,
            emoji: None,
            label: Some("Go to page".to_string()),
            style: ButtonStyle::Primary,
            url: None,
        },
        Button {
            custom_id: Some((zpage + 1).to_string()),
            disabled: !next_page_exists,
            emoji: Some(ReactionType::Unicode {
                name: "➡️".to_string(),
            }),
            label: Some("Next".to_string()),
            style: ButtonStyle::Primary,
            url: None,
        },
        Button {
            custom_id: Some("delete_leaderboard".to_string()),
            disabled: false,
            emoji: Some(ReactionType::Unicode {
                name: "🗑️".to_string(),
            }),
            label: Some("Delete".to_string()),
            style: ButtonStyle::Danger,
            url: None,
        },
    ]
    .map(Component::Button)
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
        data: Some(gen_leaderboard(&state, guild_id, zpage, Some(true)).await?),
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
    match data.custom_id.as_str() {
        "jump_modal" => {
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
            Ok(InteractionResponse {
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
            })
        }
        "delete_leaderboard" => {
            state
                .client
                .delete_message(original_message.channel_id, original_message.id)
                .await?;
            Ok(InteractionResponse {
                kind: InteractionResponseType::DeferredUpdateMessage,
                data: Some(InteractionResponseDataBuilder::new().build()),
            })
        }
        offset_str => {
            // when we create the buttons, we set next and previous's custom IDs to the current page
            // plus and minus 1. This means that we don't have to store which page which
            // message is on, because the component will tell us exactly where it wants to go!
            let offset: i64 = offset_str.parse()?;
            let show_delete_btn = original_message
                .flags
                .is_none_or(|f| !f.contains(MessageFlags::EPHEMERAL));
            Ok(InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: Some(gen_leaderboard(&state, guild_id, offset, Some(show_delete_btn)).await?),
            })
        }
    }
}
