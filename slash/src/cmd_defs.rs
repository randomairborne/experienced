use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::{CommandBuilder, UserBuilder};
pub async fn register<'a>(http: twilight_http::client::InteractionClient<'a>) {
    let mut cmds: Vec<Command> = Vec::with_capacity(3);
    cmds.push(
        CommandBuilder::new("level", "Check someone's level", CommandType::ChatInput)
            .dm_permission(false)
            .option(UserBuilder::new("user", "User to check level of").required(false))
            .build(),
    );
    cmds.push(CommandBuilder::new("level", "Check level", CommandType::User).build());
    http.set_global_commands(&cmds)
        .exec()
        .await
        .expect("Failed to set global commands for bot!");
}
