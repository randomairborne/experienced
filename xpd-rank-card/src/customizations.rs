use serde::{de::Visitor, Deserialize, Deserializer};

use crate::Error;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Customizations {
    pub username: Color,
    pub rank: Color,
    pub level: Color,
    pub border: Color,
    pub background: Color,
    pub progress_foreground: Color,
    pub progress_background: Color,
    pub background_xp_count: Color,
    pub foreground_xp_count: Color,
    pub font: String,
    pub toy: Option<String>,
    pub internal_name: String,
}

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

impl Customizations {
    /// Generate a user representation of their customizations
    /// # Errors
    /// Never, realistically. Just uses write! internally.
    pub fn display(&self, defaults: &Self) -> Result<String, std::fmt::Error> {
        use std::fmt::Write;
        let mut f = String::with_capacity(256);
        add_output!(f, "Important text", self.username, defaults.username);
        add_output!(f, "Rank", self.rank, defaults.rank);
        add_output!(f, "Level", self.level, defaults.level);
        add_output!(f, "Border", self.border, defaults.border);
        add_output!(f, "Background", self.background, defaults.background);
        add_output!(
            f,
            "Progress bar completed",
            self.progress_foreground,
            defaults.progress_foreground
        );
        add_output!(
            f,
            "Progress bar remaining",
            self.progress_background,
            defaults.progress_background
        );
        add_output!(
            f,
            "Progress bar foreground overlay",
            self.foreground_xp_count,
            defaults.foreground_xp_count
        );
        add_output!(
            f,
            "Progress bar background overlay",
            self.background_xp_count,
            defaults.background_xp_count
        );
        add_output!(f, "Font", self.font, defaults.font);
        writeln!(
            f,
            "Toy: `{}`",
            self.toy
                .as_ref()
                .map_or_else(|| "None".to_owned(), ToString::to_string)
        )?;
        add_output!(f, "Card", self.internal_name, defaults.internal_name);
        Ok(f)
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
    pub fn from_hex(hex: &impl AsRef<str>) -> Result<Self, Error> {
        let hex = hex.as_ref();
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

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ColorVisitor)
    }
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a hex string or a 3-byte array")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let [r, g, b] = v else {
            return Err(E::invalid_length(
                v.len(),
                &"must deserialize to a length 3 byte array",
            ));
        };
        Ok(Color::new(*r, *g, *b))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(3));
        while let Some(value) = seq.next_element::<u8>()? {
            bytes.push(value);
        }
        Self::visit_bytes(self, &bytes)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v.len() == 7 && v.len() == 6 {
            Err(E::invalid_length(
                v.len(),
                &"hex str must be 6 or 7 bytes long",
            ))
        } else {
            Color::from_hex(&v).map_err(|_| {
                E::invalid_value(
                    serde::de::Unexpected::Other("String was not valid hex"),
                    &"String must be valid hex with an optional # at the start",
                )
            })
        }
    }
}

#[cfg(test)]
mod test {
    use serde::Serialize;

    use super::*;
    #[derive(serde::Serialize, serde::Deserialize)]
    struct TestsStruct {
        color: Color,
    }

    #[test]
    fn basic_roundtrip() {
        let mut shared_serialize_string = String::with_capacity(32);
        for red in u8::MIN..=u8::MAX {
            eprintln!("red progress: {red}");
            for green in u8::MIN..=u8::MAX {
                for blue in u8::MIN..=u8::MAX {
                    let color = Color::new(red, green, blue);
                    let test_struct = TestsStruct { color };
                    let serializer = toml::ser::Serializer::new(&mut shared_serialize_string);
                    test_struct.serialize(serializer).unwrap();
                    let rt_struct: TestsStruct = toml::from_str(&shared_serialize_string).unwrap();
                    assert_eq!(color, rt_struct.color);
                    shared_serialize_string.clear();
                }
            }
        }
    }
}
