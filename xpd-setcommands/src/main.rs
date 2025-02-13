use std::env::VarError;

use twilight_http::Client;
use twilight_model::id::{marker::GuildMarker, Id};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let token = valk_utils::get_var("DISCORD_TOKEN");
    let control_guild: Option<Id<GuildMarker>> = match std::env::var("CONTROL_GUILD") {
        Ok(v) => Some(v.parse().expect("Could not parse guild ID")),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => panic!("Non-UTF-8 CONTROL_GUILD"),
    };

    let client = Client::new(token);
    let app_id = client
        .current_user_application()
        .await
        .expect("Failed to contact discord for app ID")
        .model()
        .await
        .expect("Failed to get app ID from discord")
        .id;

    let cmds = xpd_slash_defs::get_commands();
    let admin_commands = xpd_slash_defs::admin_commands();

    client
        .interaction(app_id)
        .set_global_commands(&cmds)
        .await
        .expect("Failed to set global commands for bot!");
    if let Some(control_guild) = control_guild {
        client
            .interaction(app_id)
            .set_guild_commands(control_guild, &admin_commands)
            .await
            .expect("Failed to set admin commands");
    }
}
