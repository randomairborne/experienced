use twilight_model::{application::command::CommandType, guild::Permissions};
use twilight_util::builder::command::{
    CommandBuilder, IntegerBuilder, RoleBuilder, SubCommandBuilder, SubCommandGroupBuilder,
    UserBuilder,
};

pub async fn register(http: twilight_http::client::InteractionClient<'_>) {
    let cmds = [
        CommandBuilder::new(
            "level",
            "Check someone's level and rank",
            CommandType::ChatInput,
        )
        .dm_permission(false)
        .option(UserBuilder::new("user", "User to check level of").required(false))
        .validate()
        .expect("Level slash command is invalid!")
        .build(),
        CommandBuilder::new(
            "rank",
            "Check someone's rank and level",
            CommandType::ChatInput,
        )
        .dm_permission(false)
        .option(UserBuilder::new("user", "User to check level of").required(false))
        .validate()
        .expect("Rank slash command is invalid!")
        .build(),
        CommandBuilder::new("xp", "Manage Experienced functions", CommandType::ChatInput)
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .dm_permission(false)
            .option(
                SubCommandGroupBuilder::new("rewards", "Manage leveling role rewards").subcommands(
                    [
                        SubCommandBuilder::new("add", "Add a new leveling reward")
                            .option(
                                IntegerBuilder::new("level", "What level to grant the role at")
                                    .min_value(1)
                                    .required(true),
                            )
                            .option(RoleBuilder::new("role", "Role to grant").required(true)),
                        SubCommandBuilder::new("remove", "Remove a leveling reward")
                            .option(
                                IntegerBuilder::new("level", "What level of role reward to remove")
                                    .min_value(1),
                            )
                            .option(RoleBuilder::new("role", "What role reward to remove")),
                        SubCommandBuilder::new("list", "Show a list of leveling rewards"),
                    ],
                ),
            )
            .validate()
            .expect("XP slash command is invalid")
            .build(),
        CommandBuilder::new("Get level", "", CommandType::User).build(),
        CommandBuilder::new("Get author level", "", CommandType::Message).build(),
    ];
    http.set_global_commands(&cmds)
        .await
        .expect("Failed to set global commands for bot!");
}
