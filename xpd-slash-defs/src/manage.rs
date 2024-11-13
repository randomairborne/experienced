use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{channel::Attachment, guild::Permissions};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "manage",
    desc = "Do bulk actions on user XP in your server",
    dm_permission = false,
    default_permissions = "Self::default_permissions"
)]
#[allow(clippy::large_enum_variant)]
pub enum ManageCommand {
    #[command(name = "reset")]
    ResetGuild(ManageCommandResetGuild),
    #[command(name = "import")]
    Import(ManageCommandImport),
    #[command(name = "export")]
    Export(ManageCommandExport),
}

impl ManageCommand {
    #[inline]
    const fn default_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

pub const CONFIRMATION_STRING: &str = "I Understand The Risks";

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "reset",
    desc = "DANGER: Reset ALL the leveling data for your guild! This is IRREVERSIBLE!",
    dm_permission = false
)]
pub struct ManageCommandResetGuild {
    #[command(
        desc = "\"I Understand The Risks\", to ensure you know this will delete ALL YOUR DATA"
    )]
    pub confirm_message: String,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "import",
    desc = "Import leveling data from another Discord bot or other source",
    dm_permission = false
)]
pub struct ManageCommandImport {
    #[command(desc = "Leveling JSON file")]
    pub levels: Attachment,
    #[command(desc = "Overwrite, rather then summing with previous leveling data")]
    pub overwrite: Option<bool>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "export",
    desc = "Export this server's leveling data into a JSON file",
    dm_permission = false
)]
pub struct ManageCommandExport;
