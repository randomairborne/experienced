use twilight_model::{application::command::CommandType, guild::Permissions};
use twilight_util::builder::command::{
    CommandBuilder, IntegerBuilder, RoleBuilder, StringBuilder, SubCommandBuilder,
    SubCommandGroupBuilder, UserBuilder,
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
        CommandBuilder::new("card", "Set hex codes for different color schemes in your rank card.", CommandType::ChatInput)
            .dm_permission(true)
            .option(SubCommandBuilder::new("reset", "Reset your card to defaults.").build())
            .option(SubCommandBuilder::new("fetch", "Get your current card settings, including defaults.").build())
            .option(
                SubCommandBuilder::new("edit", "Edit card colors by specifying hex codes for values you would like to change.")
                    .option(StringBuilder::new(
                        "background",
                        "What background color to use",
                    ))
                    .option(StringBuilder::new("border", "What border color to use"))
                    .option(StringBuilder::new(
                        "important",
                        "What color to use for important informational text",
                    ))
                    .option(StringBuilder::new(
                        "secondary",
                        "What color to use for secondary informational text",
                    ))
                    .option(StringBuilder::new(
                        "rank",
                        "What color to use for rank display",
                    ))
                    .option(StringBuilder::new(
                        "level",
                        "What color to use for level display",
                    ))
                    .option(StringBuilder::new(
                        "progress_background",
                        "What color to use for the empty part of the level up progress bar",
                    ))
                    .option(StringBuilder::new(
                        "progress_foreground",
                        "What color to use for the filled part of the level up progress baray",
                    ))
                    .build(),
            )
            .validate()
            .expect("Card slash command is invalid!")
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
