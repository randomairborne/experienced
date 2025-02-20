use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::id::{Id, marker::UserMarker};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "gdpr",
    desc = "Exercise your rights under the GDPR",
    dm_permission = true
)]
pub enum GdprCommand {
    #[command(name = "delete")]
    Delete(GdprCommandDelete),
    #[command(name = "download")]
    Download(GdprCommandDownload),
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "delete", desc = "Delete all of your data from Experienced")]
pub struct GdprCommandDelete {
    #[command(desc = "Your @username (for confirmation)")]
    pub user: Id<UserMarker>,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "download",
    desc = "Download all of your data stored by Experienced"
)]
pub struct GdprCommandDownload;
