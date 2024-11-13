use twilight_http::Client;
use twilight_model::id::{marker::GuildMarker, Id};

#[tokio::main]
pub async fn main() {
    let token = valk_utils::get_var("DISCORD_TOKEN");
    let control_guild: Id<GuildMarker> = valk_utils::parse_var("CONTROL_GUILD");

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
    client
        .interaction(app_id)
        .set_guild_commands(control_guild, &admin_commands)
        .await
        .expect("Failed to set admin commands");
}
