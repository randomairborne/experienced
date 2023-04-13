use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum Font {
    JetBrainsMono,
    Mojang,
    MontserratAlt1,
    Roboto,
}

impl Font {
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
            "JetBrains Mono" => Self::JetBrainsMono,
            "Mojang" => Self::Mojang,
            "Monsterrat Alt1" => Self::MontserratAlt1,
            "Roboto" => Self::Roboto,
            _ => return None,
        };
        Some(out)
    }
}

impl Display for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::JetBrainsMono => "JetBrains Mono",
            Self::Mojang => "Mojang",
            Self::MontserratAlt1 => "Montserrat Alt1",
            Self::Roboto => "Roboto",
        };
        f.write_str(name)
    }
}

impl serde::Serialize for Font {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
