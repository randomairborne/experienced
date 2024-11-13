use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{application::interaction::InteractionChannel, guild::Permissions};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "config",
    desc = "Configure the behavior of the bot in your server",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
#[allow(clippy::large_enum_variant)]
pub enum ConfigCommand {
    #[command(name = "reset")]
    Reset(ConfigCommandReset),
    #[command(name = "get")]
    Get(ConfigCommandGet),
    #[command(name = "rewards")]
    Rewards(ConfigCommandRewards),
    #[command(name = "levels")]
    Levels(ConfigCommandLevels),
    #[command(name = "perms_checkup")]
    PermsCheckup(ConfigCommandPermsCheckup),
}

impl ConfigCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "levels",
    desc = "Configure level-up behavior",
    dm_permission = false
)]
pub struct ConfigCommandLevels {
    #[command(
        desc = "Message to send when a user levels up. https://xp.valk.sh/docs/",
        max_length = 512,
        min_length = 1
    )]
    pub level_up_message: Option<String>,
    #[command(desc = "Where to send level up messages", channel_types = "guild_text")]
    pub level_up_channel: Option<InteractionChannel>,
    #[command(desc = "Enable push notifications to users when they level up and are mentioned")]
    pub ping_users: Option<bool>,
    #[command(
        desc = "Maximum amount of XP per message (Default 25)",
        min_value = 0,
        max_value = 32767
    )]
    pub max_xp_per_message: Option<i64>,
    #[command(
        desc = "Minimum amount of XP per message (Default 15)",
        min_value = 0,
        max_value = 32767
    )]
    pub min_xp_per_message: Option<i64>,
    #[command(
        desc = "How many seconds users must wait between messages that are able to earn XP",
        min_value = 0,
        max_value = 28800
    )]
    pub message_cooldown: Option<i64>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rewards",
    desc = "Configure role reward behavior",
    dm_permission = false
)]
pub struct ConfigCommandRewards {
    #[command(desc = "Remove all existing Experienced-managed roles when assigning a new one")]
    pub one_at_a_time: Option<bool>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "reset", desc = "Reset your guild's configuration")]
pub struct ConfigCommandReset;

#[derive(CommandModel, CreateCommand)]
#[command(name = "get", desc = "Get your guild's configuration")]
pub struct ConfigCommandGet;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "perms_checkup",
    desc = "See if Experienced has the proper permissions in your server"
)]
pub struct ConfigCommandPermsCheckup;
