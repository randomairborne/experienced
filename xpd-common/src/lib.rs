#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
    borrow::Cow, fmt::{Debug, Display, Formatter}
};

use simpleinterpolation::Interpolation;
use twilight_cache_inmemory::ResourceType;
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::Intents,
    guild::Member,
    id::{
        marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
        Id,
    },
    user::User,
    util::ImageHash,
};

pub const CURRENT_GIT_SHA: &str = env!("GIT_HASH_EXPERIENCED");
pub const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;
pub const DISCORD_EPOCH_SECS: i64 = DISCORD_EPOCH_MS / 1000;

pub trait DisplayName {
    #[must_use]
    fn display_name(&self) -> &str;
}

impl DisplayName for User {
    fn display_name(&self) -> &str {
        self.global_name.as_ref().unwrap_or(&self.name)
    }
}

impl DisplayName for Member {
    fn display_name(&self) -> &str {
        self.nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }
}

impl DisplayName for MemberDisplayInfo {
    fn display_name(&self) -> &str {
        self.nick.as_ref().map_or_else(
            || {
                self.global_name
                    .as_ref()
                    .map_or(self.name.as_str(), |global| global.as_str())
            },
            |nick| nick.as_str(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberDisplayInfo {
    pub id: Id<UserMarker>,
    pub name: String,
    pub global_name: Option<String>,
    pub nick: Option<String>,
    pub avatar: Option<ImageHash>,
    pub local_avatar: Option<ImageHash>,
    pub bot: bool,
}

impl From<User> for MemberDisplayInfo {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
            global_name: value.global_name,
            nick: None,
            avatar: value.avatar,
            local_avatar: None,
            bot: value.bot,
        }
    }
}

impl From<Member> for MemberDisplayInfo {
    fn from(value: Member) -> Self {
        Self {
            id: value.user.id,
            name: value.user.name,
            global_name: value.user.global_name,
            nick: value.nick,
            avatar: value.user.avatar,
            local_avatar: value.avatar,
            bot: value.user.bot,
        }
    }
}

impl MemberDisplayInfo {
    #[must_use]
    pub fn with_nick(self, nick: Option<String>) -> Self {
        Self { nick, ..self }
    }
}

pub const TEMPLATE_VARIABLES: [&str; 9] = [
    "user_id",
    "user_mention",
    "user_username",
    "user_display_name",
    "user_nickname",
    "old_level",
    "level",
    "old_xp",
    "xp",
];
pub const DEFAULT_MAX_XP_PER_MESSAGE: i16 = 25;
pub const DEFAULT_MIN_XP_PER_MESSAGE: i16 = 15;
pub const DEFAULT_MESSAGE_COOLDOWN: i16 = 60;
pub const MAX_MESSAGE_COOLDOWN: i16 = 28800;

#[derive(Default, Debug)]
pub struct GuildConfig {
    pub one_at_a_time: Option<bool>,
    pub level_up_message: Option<Interpolation>,
    pub level_up_channel: Option<Id<ChannelMarker>>,
    pub ping_on_level_up: Option<bool>,
    pub min_xp_per_message: Option<i16>,
    pub max_xp_per_message: Option<i16>,
    pub cooldown: Option<i16>,
}

impl Display for GuildConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "One reward role at a time: {}",
            match self.one_at_a_time {
                None => "unset",
                Some(true) => "true",
                Some(false) => "false"
            }
        )?;
        writeln!(
            f,
            "Level-up message: {}",
                self.level_up_message
                    .as_ref()
                    .map(Interpolation::input_value).map_or(Cow::Borrowed("unset"), |v| Cow::Owned(format!("`{v}`")))
        )?;
        writeln!(
            f,
            "Level-up channel: {}",
            self.level_up_channel.map_or(Cow::Borrowed("unset"), |v| {
                Cow::Owned(format!("`<#{v}>`"))
            })
        )?;
        writeln!(
            f,
            "Maximum XP per message: {}",
            self.max_xp_per_message
                .unwrap_or(DEFAULT_MAX_XP_PER_MESSAGE)
        )?;
        writeln!(
            f,
            "Minimum XP per message: {}",
            self.min_xp_per_message
                .unwrap_or(DEFAULT_MIN_XP_PER_MESSAGE)
        )?;
        write!(
            f,
            "Cooldown (seconds): {}",
            self.cooldown.unwrap_or(DEFAULT_MESSAGE_COOLDOWN)
        )?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct UserStatus {
    pub id: Id<UserMarker>,
    pub guild: Id<GuildMarker>,
    pub xp: i64,
}

#[derive(Debug)]
pub struct RoleReward {
    pub id: Id<RoleMarker>,
    pub requirement: i64,
}

#[inline]
#[must_use]
pub fn compare_rewards_requirement(a: &RoleReward, b: &RoleReward) -> std::cmp::Ordering {
    a.requirement.cmp(&b.requirement)
}

pub trait RequiredDiscordResources {
    fn required_intents() -> Intents;
    fn required_events() -> EventTypeFlags;
    fn required_cache_types() -> ResourceType;
}

pub enum EventBusMessage {
    InvalidateRewards(Id<GuildMarker>),
    UpdateConfig(Id<GuildMarker>, GuildConfig),
}
