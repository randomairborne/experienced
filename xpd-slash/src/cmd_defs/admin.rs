use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::id::marker::UserMarker;
use twilight_model::id::{marker::GuildMarker, Id};

#[derive(CommandModel, CreateCommand)]
#[command(name = "leave", desc = "Leave a guild")]
pub struct AdminCommandLeave {
    #[command(desc = "Guild to leave")]
    pub guild: Id<GuildMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "resetguild", desc = "Reset the stats of a guild")]
pub struct AdminCommandResetGuild {
    #[command(desc = "Guild to reset")]
    pub guild: Id<GuildMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "resetuser", desc = "Reset the stats of a user")]
pub struct AdminCommandResetUser {
    #[command(desc = "User to reset")]
    pub user: Id<UserMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "banguild", desc = "Ban a guild from using the bot")]
pub struct AdminCommandBanGuild {
    #[command(desc = "Guild to reset")]
    pub guild: Id<GuildMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "banuser", desc = "Ban a user from using the bot")]
pub struct AdminCommandBanUser {
    #[command(desc = "User to reset")]
    pub user: Id<UserMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "leave", desc = "Leave a guild")]
pub struct AdminCommandSetNick {
    #[command(desc = "User to fetch settings of")]
    pub guild: Id<GuildMarker>,
    #[command(desc = "Nick to change to", max_length = 32, min_length = 2)]
    pub nick: String,
}
