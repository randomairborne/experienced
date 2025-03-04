#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
};

use simpleinterpolation::Interpolation;
use strum_macros::FromRepr;
use twilight_cache_inmemory::ResourceType;
use twilight_gateway::EventTypeFlags;
use twilight_model::{
    gateway::Intents,
    guild::{Member, PartialMember},
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
    },
    user::User,
    util::ImageHash,
};

pub const CURRENT_GIT_SHA: &str = env!("GIT_HASH_EXPERIENCED");
pub const CURRENT_GIT_REV_COUNT: &str = env!("GIT_REV_COUNT_EXPERIENCED");
pub const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;
pub const DISCORD_EPOCH_SECS: i64 = DISCORD_EPOCH_MS / 1000;

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

    #[must_use]
    pub fn from_partial_member(member: PartialMember) -> Option<Self> {
        let user = member.user?;
        Some(Self {
            id: user.id,
            name: user.name,
            global_name: user.global_name,
            nick: member.nick,
            avatar: user.avatar,
            local_avatar: member.avatar,
            bot: user.bot,
        })
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
    pub guild_card_default_show_off: bool,
}

impl Display for GuildConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "One reward role at a time: {}",
            match self.one_at_a_time {
                None => "unset",
                Some(true) => "true",
                Some(false) => "false",
            }
        )?;
        writeln!(
            f,
            "Level-up message: {}",
            self.level_up_message
                .as_ref()
                .map(Interpolation::input_value)
                .map_or(Cow::Borrowed("unset"), |v| Cow::Owned(format!("`{v}`")))
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
        writeln!(
            f,
            "Cooldown (seconds): {}",
            self.cooldown.unwrap_or(DEFAULT_MESSAGE_COOLDOWN)
        )?;
        write!(
            f,
            "Show off guild card by default: {}",
            self.guild_card_default_show_off
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AuditLogEvent {
    pub guild: Id<GuildMarker>,
    pub target: Id<UserMarker>,
    pub moderator: Id<UserMarker>,
    pub timestamp: i64,
    pub previous: i64,
    pub delta: i64,
    pub kind: AuditLogEventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, FromRepr)]
#[non_exhaustive]
#[repr(i16)]
pub enum AuditLogEventKind {
    AddOrSub = 0,
    Reset = 1,
    Set = 2,
    KickReset = 3,
    BanReset = 4,
}

impl AuditLogEventKind {
    #[must_use]
    pub fn from_i64(t: i64) -> Option<Self> {
        let Ok(disc) = t.try_into() else {
            return None;
        };
        Self::from_repr(disc)
    }

    #[must_use]
    pub const fn to_i64(self) -> i64 {
        self as i64
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct UserStatus {
    pub id: Id<UserMarker>,
    pub guild: Id<GuildMarker>,
    pub xp: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserInGuild {
    pub guild: Id<GuildMarker>,
    pub user: Id<UserMarker>,
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
