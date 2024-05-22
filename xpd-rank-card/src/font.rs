use std::fmt::Display;

use strum_macros::VariantArray;

#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray)]
pub enum Font {
    JetBrainsMono,
    Mojang,
    MontserratAlt1,
    Roboto,
}

impl Font {
    pub const JETBRAINS_MONO_PATH: &'static str = "JetBrainsMono.ttf";
    pub const JETBRAINS_MONO_STR: &'static str = "JetBrains Mono";
    pub const MOJANG_PATH: &'static str = "Mojang.ttf";
    pub const MOJANG_STR: &'static str = "Mojang";
    pub const MONTSERRAT_ALT_1_PATH: &'static str = "MontserratAlt1.ttf";
    pub const MONTSERRAT_ALT_1_STR: &'static str = "Montserrat Alt1";
    pub const PATH_PREFIX: &'static str = "./xpd-card-resources/fonts";
    pub const ROBOTO_PATH: &'static str = "Roboto.ttf";
    pub const ROBOTO_STR: &'static str = "Roboto";

    /// # Errors
    /// This function can error when the underlying filesystem read fails.
    pub fn ttf(&self) -> Result<Vec<u8>, std::io::Error> {
        let path = format!("{}/{}", Self::PATH_PREFIX, self.filename());
        std::fs::read(path)
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

    #[must_use]
    pub(crate) const fn filename(self) -> &'static str {
        match self {
            Self::JetBrainsMono => Self::JETBRAINS_MONO_PATH,
            Self::Mojang => Self::MOJANG_PATH,
            Self::MontserratAlt1 => Self::MONTSERRAT_ALT_1_PATH,
            Self::Roboto => Self::ROBOTO_PATH,
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
