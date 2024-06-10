use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::InteractionChannel;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "levels",
    desc = "Configure level-up behavior",
    dm_permission = false
)]
pub struct ConfigCommandLevels {
    #[command(
        desc = "Message to send when a user levels up. https://xp.valk.sh/docs/#level_up_msg",
        max_length = 512,
        min_length = 1
    )]
    pub level_up_message: Option<String>,
    #[command(desc = "Where to send level up messages", channel_types = "guild_text")]
    pub level_up_channel: Option<InteractionChannel>,
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
