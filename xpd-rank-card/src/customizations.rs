use crate::Error;

pub const DEFAULT_USERNAME: Color = Color::new(255, 255, 255);
pub const DEFAULT_RANK: Color = Color::new(255, 255, 255);
pub const DEFAULT_LEVEL: Color = Color::new(143, 202, 92);
pub const DEFAULT_BORDER: Color = Color::new(133, 79, 43);
pub const DEFAULT_BACKGROUND: Color = Color::new(97, 55, 31);
pub const DEFAULT_PROGRESS_FOREGROUND: Color = Color::new(71, 122, 30);
pub const DEFAULT_PROGRESS_BACKGROUND: Color = Color::new(143, 202, 92);
pub const DEFAULT_BACKGROUND_XP_COUNT: Color = Color::new(0, 0, 0);
pub const DEFAULT_FOREGROUND_XP_COUNT: Color = Color::new(255, 255, 255);

#[derive(serde::Serialize, Debug, Clone, Copy)]
pub struct Colors {
    pub username: Color,
    pub rank: Color,
    pub level: Color,
    pub border: Color,
    pub background: Color,
    pub progress_foreground: Color,
    pub progress_background: Color,
    pub background_xp_count: Color,
    pub foreground_xp_count: Color,
}

impl std::fmt::Display for Colors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::add_output!(f, "Important text", self.username, DEFAULT_USERNAME);
        crate::add_output!(f, "Rank", self.rank, DEFAULT_RANK);
        crate::add_output!(f, "Level", self.level, DEFAULT_LEVEL);
        crate::add_output!(f, "Border", self.border, DEFAULT_BORDER);
        crate::add_output!(f, "Background", self.background, DEFAULT_BACKGROUND);
        crate::add_output!(
            f,
            "Progress bar completed",
            self.progress_foreground,
            DEFAULT_PROGRESS_FOREGROUND
        );
        crate::add_output!(
            f,
            "Progress bar remaining",
            self.progress_background,
            DEFAULT_PROGRESS_BACKGROUND
        );
        crate::add_output!(
            f,
            "Progress bar foreground overlay",
            self.foreground_xp_count,
            DEFAULT_FOREGROUND_XP_COUNT
        );
        crate::add_output!(
            f,
            "Progress bar background overlay",
            self.background_xp_count,
            DEFAULT_BACKGROUND_XP_COUNT
        );
        Ok(())
    }
}

#[macro_export]
macro_rules! add_output {
    ($f:expr, $name:expr, $val:expr, $default:expr) => {
        write!($f, "{}: `{}`", $name, $val)?;
        if $val == $default {
            writeln!($f, " (default)")?;
        } else {
            writeln!($f)?;
        };
    };
}

#[macro_export]
macro_rules! from_maybe_hex {
    ($val:expr, $default:expr) => {
        $val.map_or($default, |color| {
            Color::from_hex(&color).unwrap_or($default)
        })
    };
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            username: DEFAULT_USERNAME,
            rank: DEFAULT_RANK,
            level: DEFAULT_LEVEL,
            border: DEFAULT_BORDER,
            background: DEFAULT_BACKGROUND,
            progress_foreground: DEFAULT_PROGRESS_FOREGROUND,
            progress_background: DEFAULT_PROGRESS_BACKGROUND,
            background_xp_count: DEFAULT_BACKGROUND_XP_COUNT,
            foreground_xp_count: DEFAULT_FOREGROUND_XP_COUNT,
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
    /// # Errors
    /// Errors if the hex color is invalid
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
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
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
