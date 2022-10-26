use twilight_model::{application::command::CommandType, guild::Permissions};
use twilight_util::builder::command::{
    CommandBuilder, NumberBuilder, RoleBuilder, SubCommandBuilder, SubCommandGroupBuilder,
    UserBuilder,
};
pub async fn register(http: twilight_http::client::InteractionClient<'_>) {
    let cmds = [
        CommandBuilder::new("level", "Check someone's level", CommandType::ChatInput)
            .dm_permission(false)
            .option(UserBuilder::new("user", "User to check level of").required(false))
            .validate()
            .expect("Level slash command is invalid!")
            .build(),
        CommandBuilder::new("rank", "Check someone's level", CommandType::ChatInput)
            .dm_permission(false)
            .option(UserBuilder::new("user", "User to check level of").required(false))
            .validate()
            .expect("Rank slash command is invalid!")
            .build(),
        CommandBuilder::new("anvil", "Manage anvil functions", CommandType::ChatInput)
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .dm_permission(false)
            .option(
                SubCommandGroupBuilder::new("rewards", "Manage leveling role rewards").subcommands(
                    [
                        SubCommandBuilder::new("add", "Add a new leveling reward")
                            .option(
                                NumberBuilder::new("level", "What level to grant the role at")
                                    .min_value(1.0)
                                    .required(true),
                            )
                            .option(RoleBuilder::new("role", "Role to grant").required(true)),
                        SubCommandBuilder::new("remove", "Remove a leveling reward").option(
                            NumberBuilder::new("level", "What level of role reward to remove")
                                .min_value(1.0)
                                .required(true)
                                .autocomplete(true),
                        ),
                        SubCommandBuilder::new("list", "Show a list of leveling rewards"),
                    ],
                ),
            )
            .validate()
            .expect("Anvil slash command is invalid")
            .build(),
        CommandBuilder::new("Get level", "", CommandType::User).build(),
        CommandBuilder::new("Get author level", "", CommandType::Message).build(),
    ];
    http.set_global_commands(&cmds)
        .exec()
        .await
        .expect("Failed to set global commands for bot!");
}
