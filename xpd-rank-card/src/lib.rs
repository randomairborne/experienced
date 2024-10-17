#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#[allow(clippy::module_name_repetitions)]
mod config;
pub mod customizations;

use std::{collections::HashMap, ops::Deref, path::Path, sync::Arc, time::Instant};

use rayon::ThreadPoolBuilder;
use resvg::usvg::{
    fontdb::{Database, Family, Query},
    ImageKind, ImageRendering,
};
use tera::{Tera, Value};
use tracing::debug;

pub use crate::config::{Config, ConfigItem};

/// Context is the main argument of [`InnerSvgState::render`], and takes parameters for what to put on
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

#[derive(Clone)]
pub struct SvgState(pub Arc<InnerSvgState>);

impl SvgState {
    /// Create a new [`SvgState`]
    ///
    /// # Errors
    /// This function usually fails when your manifest.toml is invalid.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, NewSvgStateError> {
        Ok(Self(Arc::new(InnerSvgState::new(path.as_ref())?)))
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
}

impl Deref for SvgState {
    type Target = InnerSvgState;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// This struct should be constructed with [`InnerSvgState::new`] to begin rendering rank cards
pub struct InnerSvgState {
    fontdb: Arc<Database>,
    tera: Tera,
    threads: rayon::ThreadPool,
    images: HashMap<String, Arc<Vec<u8>>>,
    config: Config,
}

impl InnerSvgState {
    /// Create a new [`InnerSvgState`]
    ///
    /// # Errors
    /// This function will error if your manifest lies or is invalid
    pub fn new(data_dir: &Path) -> Result<Self, NewSvgStateError> {
        let config_path = data_dir.join("manifest.toml");
        let config = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config)?;

        let mut fonts = Database::new();
        for font in &config.fonts {
            fonts.load_font_file(data_dir.join(&font.file))?;
            let test_query = Query {
                families: &[Family::Name(&font.internal_name)],
                ..Query::default()
            };
            fonts
                .query(&test_query)
                .ok_or_else(|| NewSvgStateError::WrongFontName(font.internal_name.clone()))?;
        }

        let mut tera = Tera::default();
        tera.autoescape_on(vec!["svg", "html", "xml", "htm"]);
        tera.register_filter("integerhumanize", int_humanize);
        let template_files = config
            .cards
            .clone()
            .into_iter()
            .map(|v| (data_dir.join(&v.file), Some(v.internal_name)));
        tera.add_template_files(template_files)?;

        let threads = ThreadPoolBuilder::new()
            .thread_name(|i| format!("svg-renderer-{i}"))
            .build()?;

        let images = config
            .toys
            .clone()
            .into_iter()
            .map(|v| ConfigItem {
                file: data_dir.join(&v.file),
                ..v
            })
            .map(config_item_tuple)
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self {
            fontdb: Arc::new(fonts),
            tera,
            threads,
            images,
            config,
        })
    }

    #[must_use]
    /// A config file which is guaranteed to have been successfully loaded.
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// This function is very fast. It does not need to be async.
    /// # Errors
    /// Errors if tera has a problem
    pub fn render_svg(&self, context: &Context) -> Result<String, Error> {
        let ctx = tera::Context::from_serialize(context)?;
        Ok(self.tera.render(&context.customizations.card, &ctx)?)
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
            debug!(href, "fetching toy image");
            images_clone.get(href).cloned().map(ImageKind::PNG)
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

fn config_item_tuple(ci: ConfigItem) -> Result<(String, Arc<Vec<u8>>), NewSvgStateError> {
    let data = std::fs::read(&ci.file)?;
    Ok((ci.internal_name, Arc::new(data)))
}

#[allow(clippy::unnecessary_wraps)]
fn int_humanize(v: &Value, _hm: &HashMap<String, Value>) -> tera::Result<Value> {
    let num = if let Value::Number(num) = v {
        if let Some(num) = num.as_f64() {
            num
        } else {
            return Ok(v.clone());
        }
    } else {
        return Ok(v.clone());
    };
    let (suffix, xp, precision) = if (1_000.0..1_000_000.0).contains(&num) {
        ("k", num / 1_000.0, 1)
    } else if (1_000_000.0..1_000_000_000.0).contains(&num) {
        ("m", num / 1_000_000.0, 3)
    } else if (1_000_000_000.0..1_000_000_000_000.0).contains(&num) {
        ("b", num / 1_000_000_000.0, 3)
    } else {
        ("", num, 0)
    };
    let xp_untrim = format!("{xp:.precision$}");
    let xp_trim = xp_untrim.trim_end_matches('0').trim_end_matches('.');
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
    #[error("Render result fetching error: {0}")]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("Pixmap Creation error!")]
    PixmapCreation,
    #[error("Invalid length! Color hex data length must be exactly 6 characters!")]
    InvalidLength,
}

#[derive(Debug, thiserror::Error)]
pub enum NewSvgStateError {
    #[error("Rayon error: {0:?}")]
    Rayon(#[from] rayon::ThreadPoolBuildError),
    #[error("File read error: {0:?}")]
    FileRead(#[from] std::io::Error),
    #[error("TOML deserialize error: {0:?}")]
    Deserialize(#[from] toml::de::Error),
    #[error("Template build error: {0:?}")]
    Tera(#[from] tera::Error),
    #[error("Unknown font name `{0}! Are you sure that the file-name font pairs match?")]
    WrongFontName(String),
}
