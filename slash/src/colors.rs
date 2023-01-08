const DEFAULT_IMPORTANT: Color = Color::new(255, 255, 255);
const DEFAULT_SECONDARY: Color = Color::new(204, 204, 204);
const DEFAULT_RANK: Color = Color::new(255, 255, 255);
const DEFAULT_LEVEL: Color = Color::new(143, 202, 92);
const DEFAULT_BORDER: Color = Color::new(133, 79, 43);
const DEFAULT_BACKGROUND: Color = Color::new(97, 55, 31);
const DEFAULT_PROGRESS_FOREGROUND: Color = Color::new(71, 122, 30);
const DEFAULT_PROGRESS_BACKGROUND: Color = Color::new(143, 202, 92);

#[derive(serde::Serialize, Debug, Clone, Copy)]
pub struct Colors {
    pub important: Color,
    pub secondary: Color,
    pub rank: Color,
    pub level: Color,
    pub border: Color,
    pub background: Color,
    pub progress_foreground: Color,
    pub progress_background: Color,
}

impl Colors {
    pub async fn for_user(
        db: &sqlx::MySqlPool,
        id: twilight_model::id::Id<twilight_model::id::marker::UserMarker>,
    ) -> Self {
        let Ok(rec) = sqlx::query!("SELECT * FROM custom_colors WHERE id = ?", id.get())
            .fetch_one(db)
            .await else {return Self::default();};
        let important = rec.important.map_or(DEFAULT_IMPORTANT, |color| {
            Color::from_hex(&color).unwrap_or(DEFAULT_IMPORTANT)
        });
        let secondary = rec.secondary.map_or(DEFAULT_SECONDARY, |color| {
            Color::from_hex(&color).unwrap_or(DEFAULT_SECONDARY)
        });
        let rank = rec.rank.map_or(DEFAULT_RANK, |color| {
            Color::from_hex(&color).unwrap_or(DEFAULT_RANK)
        });
        let level = rec.level.map_or(DEFAULT_LEVEL, |color| {
            Color::from_hex(&color).unwrap_or(DEFAULT_LEVEL)
        });
        let border = rec.border.map_or(DEFAULT_BORDER, |color| {
            Color::from_hex(&color).unwrap_or(DEFAULT_BORDER)
        });
        let background = rec.background.map_or(DEFAULT_BACKGROUND, |color| {
            Color::from_hex(&color).unwrap_or(DEFAULT_BACKGROUND)
        });
        let progress_foreground = rec
            .progress_foreground
            .map_or(DEFAULT_PROGRESS_FOREGROUND, |color| {
                Color::from_hex(&color).unwrap_or(DEFAULT_PROGRESS_FOREGROUND)
            });
        let progress_background = rec
            .progress_background
            .map_or(DEFAULT_PROGRESS_BACKGROUND, |color| {
                Color::from_hex(&color).unwrap_or(DEFAULT_PROGRESS_BACKGROUND)
            });

        Self {
            important,
            secondary,
            rank,
            level,
            border,
            background,
            progress_foreground,
            progress_background,
        }
    }
}
impl std::fmt::Display for Colors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Important text: {}", self.important)?;
        writeln!(f, "Secondary text: {}", self.secondary)?;
        writeln!(f, "Rank: {}", self.rank)?;
        writeln!(f, "Level: {}", self.level)?;
        writeln!(f, "Border: {}", self.border)?;
        writeln!(f, "Background: {}", self.background)?;
        writeln!(f, "Progress bar completed: {}", self.progress_foreground)?;
        writeln!(f, "Progress bar remaining: {}", self.progress_background)
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            important: DEFAULT_IMPORTANT,
            secondary: DEFAULT_SECONDARY,
            rank: DEFAULT_RANK,
            level: DEFAULT_LEVEL,
            border: DEFAULT_BORDER,
            background: DEFAULT_BACKGROUND,
            progress_foreground: DEFAULT_PROGRESS_FOREGROUND,
            progress_background: DEFAULT_PROGRESS_BACKGROUND,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    /// Takes hex-color input and converts it to a Color.
    pub fn from_hex(hex: &impl ToString) -> Result<Self, Error> {
        let hex = hex.to_string();
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(Error::InvalidLength);
        }
        Ok(Self {
            red: u8::from_str_radix(&hex[0..=1], 16)?,
            green: u8::from_str_radix(&hex[2..=3], 16)?,
            blue: u8::from_str_radix(&hex[4..=5], 16)?,
        })
    }
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid length! Hex data length must be exactly 6 characters!")]
    InvalidLength,
    #[error("Integer parsing error: {0}!")]
    ParseInt(#[from] std::num::ParseIntError),
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }
}

impl serde::Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
