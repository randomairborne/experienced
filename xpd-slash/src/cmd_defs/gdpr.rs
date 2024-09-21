use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::id::{marker::UserMarker, Id};

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
