use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "leaderboard",
    desc = "See the leaderboard for this server",
    dm_permission = false
)]
pub struct LeaderboardCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
    #[command(desc = "Page to jump to", min_value = 1)]
    pub page: Option<i64>,
    #[command(desc = "Want to show this off to everyone?")]
    pub show_off: Option<bool>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rank",
    desc = "Check someone's rank and level",
    dm_permission = false
)]
pub struct RankCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
    #[command(desc = "Show off this card publicly")]
    pub show_off: Option<bool>,
}
