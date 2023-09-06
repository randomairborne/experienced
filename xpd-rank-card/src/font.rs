use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    JetBrainsMono,
    Mojang,
    MontserratAlt1,
    Roboto,
}

impl Font {
    pub const JETBRAINS_MONO_STR: &str = "JetBrains Mono";
    pub const MOJANG_STR: &str = "Mojang";
    pub const MONTSERRAT_ALT_1_STR: &str = "Montserrat Alt1";
    pub const ROBOTO_STR: &str = "Roboto";
    #[must_use]
    pub const fn ttf(&self) -> &'static [u8] {
        match self {
            Self::JetBrainsMono => include_bytes!("resources/fonts/JetBrainsMono.ttf"),
            Self::Mojang => include_bytes!("resources/fonts/Mojang.ttf"),
            Self::MontserratAlt1 => include_bytes!("resources/fonts/MontserratAlt1.ttf"),
            Self::Roboto => include_bytes!("resources/fonts/Roboto.ttf"),
        }
    }
    #[must_use]
    pub fn from_name(data: &str) -> Option<Self> {
        let out = match data {
            Self::JETBRAINS_MONO_STR => Self::JetBrainsMono,
            Self::MOJANG_STR => Self::Mojang,
            Self::MONTSERRAT_ALT_1_STR => Self::MontserratAlt1,
            Self::ROBOTO_STR => Self::Roboto,
            _ => return None,
        };
        Some(out)
    }
    #[must_use]
    pub(crate) const fn family(self) -> &'static str {
        match self {
            Self::JetBrainsMono => "'JetBrains Mono'",
            Self::Mojang => "Mojang",
            Self::MontserratAlt1 => "'Montserrat-Alt1'",
            Self::Roboto => "Roboto",
        }
    }
}

impl Display for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::JetBrainsMono => Self::JETBRAINS_MONO_STR,
            Self::Mojang => Self::MOJANG_STR,
            Self::MontserratAlt1 => Self::MONTSERRAT_ALT_1_STR,
            Self::Roboto => Self::ROBOTO_STR,
        };
        f.write_str(name)
    }
}

impl serde::Serialize for Font {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.family())
    }
}
