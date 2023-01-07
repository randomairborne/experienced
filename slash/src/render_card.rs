use resvg::usvg_text_layout::TreeTextToPath;

use crate::AppState;

#[derive(serde::Serialize)]
struct Context {
    level: String,
    rank: String,
    name: String,
    discriminator: String,
    width: u64,
}

pub async fn render(
    state: AppState,
    name: String,
    discriminator: String,
    level: String,
    rank: String,
    percentage: u8,
) -> Result<Vec<u8>, RenderingError> {
    let context = tera::Context::from_serialize(Context {
        level,
        rank,
        name,
        discriminator,
        width: 40 + (u64::from(percentage) * 7),
    })?;
    tokio::task::spawn_blocking(move || do_render(&state, &context)).await?
}

fn do_render(state: &AppState, context: &tera::Context) -> Result<Vec<u8>, RenderingError> {
    let opt = resvg::usvg::Options::default();
    let svg = state.svg.tera.render("svg", context)?;
    let mut tree = resvg::usvg::Tree::from_str(&svg, &opt)?;
    tree.convert_text(&state.svg.fonts, opt.keep_named_groups);
    let pixmap_size = tree.size.to_screen_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or(RenderingError::PixmapCreation)?;
    resvg::render(
        &tree,
        resvg::usvg::FitTo::Original,
        resvg::tiny_skia::Transform::default(),
        pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}

#[derive(Debug, thiserror::Error)]
pub enum RenderingError {
    #[error("Tera error: {0}")]
    Template(#[from] tera::Error),
    #[error("Tokio JoinError: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("uSVG error: {0}")]
    Usvg(#[from] resvg::usvg::Error),
    #[error("Pixmap error: {0}")]
    Pixmap(#[from] png::EncodingError),
    #[error("Pixmap Creation error!")]
    PixmapCreation,
}
