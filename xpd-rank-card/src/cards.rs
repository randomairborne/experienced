use crate::customizations::{Color, Customizations};

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub enum Card {
    #[default]
    Classic,
    Vertical,
}

impl Card {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match *self {
            Self::Classic => "classic.svg",
            Self::Vertical => "vertical.svg",
        }
    }
    #[must_use]
    pub const fn template(&self) -> &'static str {
        match *self {
            Self::Classic => include_str!("resources/cards/classic.svg"),
            Self::Vertical => include_str!("resources/cards/vertical.svg"),
        }
    }
    #[must_use]
    pub fn from_name(data: &str) -> Option<Self> {
        let out = match data {
            "classic.svg" => Self::Classic,
            "vertical.svg" => Self::Vertical,
            _ => return None,
        };
        Some(out)
    }
    #[must_use]
    pub const fn default_customizations(&self) -> Customizations {
        match *self {
            Self::Classic => CLASSIC_CUSTOMIZATIONS,
            Self::Vertical => VERTICAL_CUSTOMIZATIONS,
        }
    }
}

const CLASSIC_CUSTOMIZATIONS: Customizations = Customizations {
    username: Color::new(255, 255, 255),
    rank: Color::new(255, 255, 255),
    level: Color::new(143, 202, 92),
    border: Color::new(133, 79, 43),
    background: Color::new(97, 55, 31),
    progress_foreground: Color::new(71, 122, 30),
    progress_background: Color::new(143, 202, 92),
    background_xp_count: Color::new(0, 0, 0),
    foreground_xp_count: Color::new(255, 255, 255),
    font: crate::Font::Mojang,
    toy: None,
    card: Card::Classic,
};

const VERTICAL_CUSTOMIZATIONS: Customizations = Customizations {
    username: Color::new(255, 255, 255),
    rank: Color::new(255, 255, 255),
    level: Color::new(143, 202, 92),
    border: Color::new(133, 79, 43),
    background: Color::new(97, 55, 31),
    progress_foreground: Color::new(71, 122, 30),
    progress_background: Color::new(143, 202, 92),
    background_xp_count: Color::new(0, 0, 0),
    foreground_xp_count: Color::new(255, 255, 255),
    font: crate::Font::Roboto,
    toy: None,
    card: Card::Vertical,
};
