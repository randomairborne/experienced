use twilight_model::application::command::CommandType;
use twilight_util::builder::command::CommandBuilder;

use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};

#[derive(CommandModel, CreateCommand)]
#[command(name = "leaderboard", desc = "See the leaderboard for this server")]
pub struct LeaderboardCommand;

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "rank",
    desc = "Check someone's rank and level",
    dm_permission = false
)]
pub struct RankCommand {
    #[command(desc = "User to check level of")]
    pub user: Option<ResolvedUser>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "toy", desc = "Pick a toy image to use in your card")]
pub struct ToyCommand {
    #[command(desc = "What toy image to use in the card")]
    pub toy_image: crate::toy::Toy,
}

pub async fn register(http: twilight_http::client::InteractionClient<'_>) {
    let cmds = vec![
        RankCommand::create_command().into(),
        ToyCommand::create_command().into(),
        LeaderboardCommand::create_command().into(),
        CommandBuilder::new("Get level", "", CommandType::User).build(),
        CommandBuilder::new("Get author level", "", CommandType::Message).build(),
    ];
    http.set_global_commands(&cmds)
        .await
        .expect("Failed to set global commands for bot!");
}
