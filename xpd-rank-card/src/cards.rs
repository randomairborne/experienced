#[derive(Clone, Copy, Debug, Default)]
pub enum Card {
    #[default]
    Classic,
}

impl Card {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match *self {
            Self::Classic => "classic.svg",
        }
    }
    #[must_use]
    pub const fn template(&self) -> &'static str {
        match *self {
            Self::Classic => include_str!("resources/cards/classic.svg"),
        }
    }
    #[must_use]
    pub fn from_name(data: &str) -> Option<Self> {
        let out = match data {
            "classic.svg" => Self::Classic,
            _ => return None,
        };
        Some(out)
    }
}
