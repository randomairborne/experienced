use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(CommandModel, CreateCommand)]
#[command(name = "delete", desc = "Delete all of your data from Experienced")]
pub struct GdprCommandDelete {
    #[command(desc = "Your @username (for confirmation)")]
    pub username: String,
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "download",
    desc = "Download all of your data stored by Experienced"
)]
pub struct GdprCommandDownload;
