#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod cards;
pub mod customizations;
mod font;
mod toy;

use std::{sync::Arc, time::Instant};

pub use font::Font;
use rayon::ThreadPoolBuilder;
use resvg::usvg::{fontdb::Database, ImageKind, ImageRendering};
use strum::{EnumCount, VariantArray};
use tera::{Tera, Value};
pub use toy::Toy;
use tracing::debug;

/// Context is the main argument of [`SvgState::render`], and takes parameters for what to put on
/// the card.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// Level of the user for display
    pub level: u64,
    /// Rank of the user for display
    pub rank: i64,
    /// Username
    pub name: String,
    /// Percentage of the way to the next level, out of 100
    pub percentage: u64,
    /// Current XP count
    pub current: u64,
    /// Total XP needed to complete this level
    pub needed: u64,
    /// Customization data
    pub customizations: customizations::Customizations,
    /// Base64-encoded PNG string.
    pub avatar: String,
}

/// This struct should be constructed with [`SvgState::new`] to begin rendering rank cards
#[derive(Clone)]
pub struct SvgState {
    fontdb: Arc<Database>,
    tera: Arc<Tera>,
    threads: Arc<rayon::ThreadPool>,
    images: Arc<[Arc<Vec<u8>>; Toy::COUNT]>,
}

impl SvgState {
    /// Create a new [`SvgState`]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// this function renders an SVG on the internal thread pool, and returns PNG-encoded image
    /// data on completion.
    /// # Errors
    /// Errors on [`resvg`](https://docs.rs/resvg) library failure. This will almost always be a library bug.
    pub async fn render(&self, data: Context) -> Result<Vec<u8>, Error> {
        let cloned_self = self.clone();
        let (send, recv) = tokio::sync::oneshot::channel();
        debug!("starting async render of SVG");
        self.threads.spawn(move || {
            send.send(cloned_self.sync_render(&data)).ok();
        });
        recv.await?
    }

    /// This function is very fast. It does not need to be async.
    /// # Errors
    /// Errors if tera has a problem
    pub fn render_svg(&self, context: &Context) -> Result<String, Error> {
        let name = context.customizations.card.name();
        let ctx = tera::Context::from_serialize(context)?;
        Ok(self.tera.render(name, &ctx)?)
    }

    /// Render the PNG for a card.
    /// # Errors
    /// Errors if tera has a problem, or resvg does.
    pub fn sync_render(&self, context: &Context) -> Result<Vec<u8>, Error> {
        let start = Instant::now();
        let svg = self.render_svg(context)?;
        let resolve_data =
            Box::new(
                |mime: &str, data: Arc<Vec<u8>>, _: &resvg::usvg::Options| match mime {
                    "image/png" => Some(ImageKind::PNG(data)),
                    "image/jpg" | "image/jpeg" => Some(ImageKind::JPEG(data)),
                    _ => None,
                },
            );
        let images_clone = self.images.clone();
        let resolve_string = Box::new(move |href: &str, _: &resvg::usvg::Options| {
            let toy = Toy::from_filename(href)?;
            images_clone.get(toy as usize).cloned().map(ImageKind::PNG)
        });
        let opt = resvg::usvg::Options {
            image_href_resolver: resvg::usvg::ImageHrefResolver {
                resolve_data,
                resolve_string,
            },
            image_rendering: ImageRendering::OptimizeSpeed,
            font_family: context.customizations.font.to_string(),
            fontdb: self.fontdb.clone(),
            ..Default::default()
        };
        let tree = resvg::usvg::Tree::from_str(&svg, &opt)?;
        let pixmap_size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
            .ok_or(Error::PixmapCreation)?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::default(),
            &mut pixmap.as_mut(),
        );
        let png = pixmap.encode_png()?;
        debug!(
            micros_taken = start.elapsed().as_micros(),
            "Rendered SVG image"
        );
        Ok(png)
    }
}

impl Default for SvgState {
    fn default() -> Self {
        let mut fonts = Database::new();
        for variant in Font::VARIANTS {
            fonts.load_font_data(
                variant.ttf().unwrap_or_else(|e| {
                    panic!("Failed to load font `{}`: {e}", variant.filename())
                }),
            );
        }
        let mut tera =
            Tera::new("xpd-card-resources/cards/**/*.svg").expect("Failed to build card templates");
        tera.autoescape_on(vec!["svg", "html", "xml", "htm"]);
        tera.register_filter("integerhumanize", int_humanize);
        let threads = ThreadPoolBuilder::new().build().unwrap();
        let images = Toy::VARIANTS
            .iter()
            .map(|v| {
                v.load_png()
                    .unwrap_or_else(|e| panic!("Failed to load toy PNG `{}`: {e}", v.filename()))
            })
            .map(Arc::new)
            .collect::<Vec<Arc<Vec<u8>>>>()
            .try_into()
            .unwrap();
        Self {
            fontdb: Arc::new(fonts),
            tera: Arc::new(tera),
            threads: Arc::new(threads),
            images: Arc::new(images),
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn int_humanize(v: &Value, _hm: &std::collections::HashMap<String, Value>) -> tera::Result<Value> {
    let num = if let Value::Number(num) = v {
        if let Some(num) = num.as_f64() {
            num
        } else {
            return Ok(v.clone());
        }
    } else {
        return Ok(v.clone());
    };
    let (suffix, xp) = if (1_000.0..1_000_000.0).contains(&num) {
        ("k", num / 1_000.0)
    } else if (1_000_000.0..1_000_000_000.0).contains(&num) {
        ("m", num / 1_000_000.0)
    } else if (1_000_000_000.0..1_000_000_000_000.0).contains(&num) {
        ("b", num / 1_000_000_000.0)
    } else {
        ("", num)
    };
    let xp_untrim = format!("{xp:.1}");
    let xp_trim = xp_untrim.trim_end_matches(".0");
    Ok(Value::String(format!("{xp_trim}{suffix}")))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tera error: {0}")]
    Template(#[from] tera::Error),
    #[error("uSVG error: {0}")]
    Usvg(#[from] resvg::usvg::Error),
    #[error("Integer parsing error: {0}!")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Pixmap error: {0}")]
    Pixmap(#[from] png::EncodingError),
    #[error("Rayon error: {0}")]
    Rayon(#[from] rayon::ThreadPoolBuildError),
    #[error("Render result fetching error: {0}")]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("Pixmap Creation error!")]
    PixmapCreation,
    #[error("Invalid length! Color hex data length must be exactly 6 characters!")]
    InvalidLength,
}
