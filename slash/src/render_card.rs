use std::{collections::HashMap, sync::Arc};

use resvg::{
    usvg::{ImageKind, ImageRendering},
    usvg_text_layout::TreeTextToPath,
};

use crate::AppState;

#[derive(serde::Serialize)]
pub struct Context {
    pub level: u64,
    pub rank: i64,
    pub name: String,
    pub discriminator: String,
    pub width: u64,
    pub current: u64,
    pub needed: u64,
    pub font: String,
    pub colors: crate::colors::Colors,
    pub icon: String,
}

pub async fn render(state: AppState, context: Context) -> Result<Vec<u8>, RenderingError> {
    let context = tera::Context::from_serialize(context)?;
    tokio::task::spawn_blocking(move || do_render(&state.svg, &context)).await?
}

fn do_render(state: &SvgState, context: &tera::Context) -> Result<Vec<u8>, RenderingError> {
    let svg = state.tera.render("svg", context)?;
    let resolve_data =
        Box::new(|mime: &str, data: std::sync::Arc<Vec<u8>>, _: &resvg::usvg::Options|
        match mime {
            "image/png" => Some(ImageKind::PNG(data)),
            "image/jpg" | "image/jpeg" => Some(ImageKind::JPEG(data)),
            _ => None
        });
    let resolve_string_state = state.clone();
    let resolve_string = Box::new(move |href: &str, _: &resvg::usvg::Options| {
        Some(ImageKind::PNG(
            resolve_string_state.images.get(href)?.clone(),
        ))
    });
    let opt = resvg::usvg::Options {
        image_href_resolver: resvg::usvg::ImageHrefResolver {
            resolve_data,
            resolve_string,
        },
        image_rendering: ImageRendering::OptimizeSpeed,
        ..Default::default()
    };
    let mut tree = resvg::usvg::Tree::from_str(&svg, &opt)?;
    tree.convert_text(&state.fonts, opt.keep_named_groups);
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

#[derive(Clone)]
pub struct SvgState {
    fonts: Arc<resvg::usvg_text_layout::fontdb::Database>,
    tera: Arc<tera::Tera>,
    images: Arc<HashMap<String, Arc<Vec<u8>>>>,
}

impl SvgState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for SvgState {
    fn default() -> Self {
        let mut fonts = resvg::usvg_text_layout::fontdb::Database::new();
        fonts.load_font_data(include_bytes!("resources/fonts/Mojang.ttf").to_vec());
        fonts.load_font_data(include_bytes!("resources/fonts/Roboto.ttf").to_vec());
        fonts.load_font_data(include_bytes!("resources/fonts/JetBrainsMono.ttf").to_vec());
        fonts.load_font_data(include_bytes!("resources/fonts/MontserratAlt1.ttf").to_vec());
        let mut tera = tera::Tera::default();
        tera.autoescape_on(vec!["svg", "html", "xml", "htm"]);
        tera.add_raw_template("svg", include_str!("resources/card.svg"))
            .expect("Failed to build card.svg template!");
        let images = HashMap::from([
            (
                "parrot.png".to_string(),
                Arc::new(include_bytes!("resources/icons/parrot.png").to_vec()),
            ),
            (
                "fox.png".to_string(),
                Arc::new(include_bytes!("resources/icons/fox.png").to_vec()),
            ),
            (
                "grassblock.png".to_string(),
                Arc::new(include_bytes!("resources/icons/grassblock.png").to_vec()),
            ),
            (
                "pickaxe.png".to_string(),
                Arc::new(include_bytes!("resources/icons/pickaxe.png").to_vec()),
            ),
            (
                "steveheart.png".to_string(),
                Arc::new(include_bytes!("resources/icons/steveheart.png").to_vec()),
            ),
            (
                "tree.png".to_string(),
                Arc::new(include_bytes!("resources/icons/tree.png").to_vec()),
            ),
        ]);
        Self {
            fonts: Arc::new(fonts),
            tera: Arc::new(tera),
            images: Arc::new(images),
        }
    }
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

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::{colors::Colors, levels::get_percentage_bar_as_pixels};

    use super::*;

    #[test]
    fn test_renderer() -> Result<(), RenderingError> {
        let state = SvgState::new();
        let xp = rand::thread_rng().gen_range(0..=10_000_000);
        let data = mee6::LevelInfo::new(xp);
        #[allow(clippy::cast_precision_loss)]
        let context = Context {
            level: data.level(),
            rank: rand::thread_rng().gen_range(0..=1_000_000),
            name: "Testy McTestington<span>".to_string(),
            discriminator: "0000".to_string(),
            width: get_percentage_bar_as_pixels(data.percentage()),
            current: xp,
            needed: mee6::xp_needed_for_level(data.level() + 1),
            font: "Roboto".to_string(),
            colors: Colors::default(),
            icon: "parrot.png".to_string(),
        };
        let output = do_render(&state, &tera::Context::from_serialize(context)?)?;
        std::fs::write("renderer_test.png", output).unwrap();
        Ok(())
    }
}
