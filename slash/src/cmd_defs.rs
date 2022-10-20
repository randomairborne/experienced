use twilight_model::application::command::CommandType;
use twilight_util::builder::command::{CommandBuilder, UserBuilder};
pub async fn register(http: twilight_http::client::InteractionClient<'_>) {
    let cmds = vec![
        CommandBuilder::new("level", "Check someone's level", CommandType::ChatInput)
            .dm_permission(false)
            .option(UserBuilder::new("user", "User to check level of").required(false))
            .validate()
            .expect("Slash command is invalid!")
            .build(),
        CommandBuilder::new("Get level", "", CommandType::User).build(),
        CommandBuilder::new("Get level", "", CommandType::Message).build(),
    ];
    http.set_global_commands(&cmds)
        .exec()
        .await
        .expect("Failed to set global commands for bot!");
}
