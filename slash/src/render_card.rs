use resvg::usvg_text_layout::TreeTextToPath;

use crate::AppState;

#[derive(serde::Serialize)]
pub struct Context {
    pub level: u64,
    pub rank: i64,
    pub name: String,
    pub discriminator: String,
    pub width: u64,
    pub current: u64,
    pub needed: f64,
    pub colors: crate::colors::Colors,
}

pub async fn render(state: AppState, context: Context) -> Result<Vec<u8>, RenderingError> {
    let context = tera::Context::from_serialize(context)?;
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
