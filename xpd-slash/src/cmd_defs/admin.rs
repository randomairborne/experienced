use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    guild::Permissions,
    id::{marker::UserMarker, Id},
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "admin",
    desc = "Globally manage the bot",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
#[allow(clippy::large_enum_variant)]
pub enum AdminCommand {
    #[command(name = "leave")]
    Leave(AdminCommandLeave),
    #[command(name = "resetguild")]
    ResetGuild(AdminCommandResetGuild),
    #[command(name = "resetuser")]
    ResetUser(AdminCommandResetUser),
    #[command(name = "setnick")]
    SetNick(AdminCommandSetNick),
    #[command(name = "banguild")]
    BanGuild(AdminCommandBanGuild),
    #[command(name = "pardonguild")]
    PardonGuild(AdminCommandPardonGuild),
    #[command(name = "guildstats")]
    GuildStats(AdminCommandGuildStats),
    #[command(name = "stats")]
    Stats(AdminCommandStats),
}

impl AdminCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "leave", desc = "Leave a guild")]
pub struct AdminCommandLeave {
    #[command(desc = "Guild to leave")]
    pub guild: String,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "resetguild", desc = "Reset the stats of a guild")]
pub struct AdminCommandResetGuild {
    #[command(desc = "Guild to reset")]
    pub guild: String,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "guildstats",
    desc = "Get some basic info about a guild the bot is in"
)]
pub struct AdminCommandGuildStats {
    #[command(desc = "Guild to fetch stats of")]
    pub guild: String,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "stats", desc = "Get some basic stats about the bot in general")]
pub struct AdminCommandStats;

#[derive(CommandModel, CreateCommand)]
#[command(name = "resetuser", desc = "Reset the stats & custom card of a user")]
pub struct AdminCommandResetUser {
    #[command(desc = "User to reset")]
    pub user: Id<UserMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "banguild", desc = "Ban a guild from using the bot")]
pub struct AdminCommandBanGuild {
    #[command(desc = "Guild to ban")]
    pub guild: String,
    #[command(desc = "How many days to ban for")]
    pub duration: Option<f64>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "pardonguild", desc = "Unban a guild from using the bot")]
pub struct AdminCommandPardonGuild {
    #[command(desc = "Guild to pardon")]
    pub guild: String,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "setnick", desc = "Set the bot's nickname in a guild")]
pub struct AdminCommandSetNick {
    #[command(desc = "Guild to set nick in")]
    pub guild: String,
    #[command(desc = "Name to set", max_length = 32, min_length = 1)]
    pub name: Option<String>,
}
