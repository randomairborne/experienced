use std::{convert::TryInto, fmt::Write};

use twilight_model::{
    application::interaction::{
        message_component::MessageComponentInteractionData,
        modal::{ModalInteractionComponent, ModalInteractionData, ModalInteractionLabel},
    },
    channel::{
        Message,
        message::{
            AllowedMentions, Component, EmojiReactionType, MessageFlags,
            component::{ActionRow, Button, ButtonStyle, Label, TextInput, TextInputStyle},
        },
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};
use xpd_common::UserStatus;
use xpd_slash_defs::levels::LeaderboardCommand;

use crate::{
    Error, SlashState, XpdInteractionData, dispatch::Respondable, response::XpdInteractionResponse,
};

pub async fn leaderboard(
    state: SlashState,
    guild_id: Id<GuildMarker>,
    guild_command: LeaderboardCommand,
) -> Result<XpdInteractionResponse, Error> {
    // "zpage" means "zero-indexed page", which is how this is represented internally.
    // We add one whenever we show it to the user, and subtract one every time we get it from the user.
    let zpage = if let Some(pick) = guild_command.page {
        pick - 1
    } else if let Some(pick) = guild_command.user {
        state.get_user_stats(pick.resolved.id, guild_id).await?.rank / 10
    } else {
        0
    };
    Ok(XpdInteractionResponse::new(
        InteractionResponseType::ChannelMessageWithSource,
        gen_leaderboard(
            &state,
            guild_id,
            zpage.try_into().map_err(|_| Error::PageDoesNotExist)?,
            guild_command.show_off,
        )
        .await?,
    ))
}

const USERS_PER_PAGE: usize = 10;

async fn gen_leaderboard(
    state: &SlashState,
    guild_id: Id<GuildMarker>,
    zpage: usize,
    show_off: Option<bool>,
) -> Result<XpdInteractionData, Error> {
    let is_ephemeral = !(show_off.unwrap_or(true));
    let users_in_guild = xpd_database::get_guild_leaderboard(&state.db, guild_id).await?;
    let cache = state.cache.clone();
    let users = tokio::task::spawn_blocking(move || {
        let mut users: Box<[UserStatus]> = users_in_guild
            .into_iter()
            .filter(|v| cache.member(guild_id, v.id).is_some())
            .collect();
        users.sort_unstable_by_key(|v| std::cmp::Reverse(v.xp));
        users
    })
    .await?;

    if users.is_empty() {
        return Err(Error::NoRanksYet);
    }

    let first_user_idx = zpage * USERS_PER_PAGE;
    let last_user_idx = (first_user_idx + USERS_PER_PAGE).clamp(1, users.len()) - 1;
    let next_page_exists = users.get(first_user_idx + USERS_PER_PAGE).is_some();

    let Some(page_users) = users.get(first_user_idx..=last_user_idx) else {
        return Err(Error::PageDoesNotExist);
    };

    // this is kinda the only way to do this
    // It's designed to only allocate once, at the start here
    let mut description = String::with_capacity(256 + users.len() * 128);
    writeln!(description, "### Leaderboard")?;
    for (i, user) in page_users.iter().enumerate() {
        let level = mee6::LevelInfo::new(user.xp.try_into().unwrap_or(0)).level();
        // first_user_idx is zero-indexed, so we need to add 1
        let rank = first_user_idx + i + 1;
        writeln!(description, "**#{rank}.** <@{}> - Level {level}", user.id)?;
    }

    let control_options = control_options(zpage, next_page_exists);

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
        id: None,
    });

    Ok(XpdInteractionData::new()
        .allowed_mentions(AllowedMentions::default())
        .components([components])
        .content(description)
        .flags(flags))
}

fn control_options(zpage: usize, next_page_exists: bool) -> [Component; 5] {
    [
        Button {
            custom_id: Some("page_indicator".to_string()),
            disabled: true,
            emoji: None,
            label: Some(format!("Page {}", zpage + 1)),
            style: ButtonStyle::Secondary,
            url: None,
            sku_id: None,
            id: None,
        },
        Button {
            custom_id: Some((zpage - 1).to_string()),
            disabled: zpage == 0,
            emoji: Some(EmojiReactionType::Unicode {
                name: "⬅".to_string(),
            }),
            label: Some("Previous".to_string()),
            style: ButtonStyle::Primary,
            url: None,
            sku_id: None,
            id: None,
        },
        Button {
            custom_id: Some("jump_modal".to_string()),
            disabled: !next_page_exists && zpage == 0,
            emoji: None,
            label: Some("Go to page".to_string()),
            style: ButtonStyle::Primary,
            url: None,
            sku_id: None,
            id: None,
        },
        Button {
            custom_id: Some((zpage + 1).to_string()),
            disabled: !next_page_exists,
            emoji: Some(EmojiReactionType::Unicode {
                name: "➡️".to_string(),
            }),
            label: Some("Next".to_string()),
            style: ButtonStyle::Primary,
            url: None,
            sku_id: None,
            id: None,
        },
        Button {
            custom_id: Some("delete_leaderboard".to_string()),
            disabled: false,
            emoji: Some(EmojiReactionType::Unicode {
                name: "🗑️".to_string(),
            }),
            label: Some("Delete".to_string()),
            style: ButtonStyle::Danger,
            url: None,
            sku_id: None,
            id: None,
        },
    ]
    .map(Component::Button)
}

pub async fn process_modal_submit(
    data: ModalInteractionData,
    guild_id: Id<GuildMarker>,
    state: SlashState,
) -> Result<XpdInteractionResponse, Error> {
    // You can't get this modal unless you are the triggering user
    let ModalInteractionComponent::Label(ModalInteractionLabel { id: _id, component }) =
        data.components.first().ok_or(Error::NoModalActionRow)?
    else {
        return Err(Error::NoFormLabel);
    };
    let ModalInteractionComponent::TextInput(field) = component.as_ref() else {
        return Err(Error::NoFormInput);
    };
    let choice: i64 = field.value.parse()?;
    let zpage = choice - 1;
    Ok(XpdInteractionResponse::new(
        InteractionResponseType::UpdateMessage,
        gen_leaderboard(
            &state,
            guild_id,
            zpage.try_into().map_err(|_| Error::PageDoesNotExist)?,
            Some(true),
        )
        .await?,
    ))
}

pub async fn process_message_component(
    data: MessageComponentInteractionData,
    original_message: Message,
    guild_id: Id<GuildMarker>,
    invoker_id: Id<UserMarker>,
    state: SlashState,
    respondable: Respondable,
) -> Result<XpdInteractionResponse, Error> {
    if original_message
        .interaction_metadata
        .ok_or(Error::NoInteractionInvocationOnInteractionMessage)?
        .user
        .id
        != invoker_id
    {
        return Err(Error::NotYourLeaderboard);
    }
    match data.custom_id.as_str() {
        "jump_modal" => Ok(XpdInteractionResponse::new(
            InteractionResponseType::Modal,
            XpdInteractionData::new()
                .components([Component::Label(Label {
                    id: None,
                    label: "Jump Destination".into(),
                    description: None,
                    component: Box::new(Component::TextInput(TextInput {
                        custom_id: "jump_modal_input".to_string(),
                        #[expect(deprecated)]
                        label: None,
                        max_length: Some(8),
                        min_length: Some(1),
                        placeholder: Some("What page to jump to".to_string()),
                        required: Some(true),
                        style: TextInputStyle::Short,
                        value: None,
                        id: None,
                    })),
                })])
                .custom_id("jump_modal".to_string())
                .title("Go to page..".to_string()),
        )),
        "delete_leaderboard" => {
            let deferred_update = InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: None,
            };
            state
                .client
                .interaction(state.app_id)
                .create_response(respondable.id(), respondable.token(), &deferred_update)
                .await?;
            state
                .client
                .interaction(state.app_id)
                .delete_response(respondable.token())
                .await?;
            Ok(XpdInteractionResponse::inhibited())
        }
        offset_str => {
            // when we create the buttons, we set next and previous's custom IDs to the current page
            // plus and minus 1. This means that we don't have to store which page which
            // message is on, because the component will tell us exactly where it wants to go!
            let offset: usize = offset_str.parse()?;
            let show_delete_btn = original_message
                .flags
                .is_none_or(|f| !f.contains(MessageFlags::EPHEMERAL));
            Ok(XpdInteractionResponse::new(
                InteractionResponseType::UpdateMessage,
                gen_leaderboard(&state, guild_id, offset, Some(show_delete_btn)).await?,
            ))
        }
    }
}
