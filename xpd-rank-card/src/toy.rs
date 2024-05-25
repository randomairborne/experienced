use std::fmt::Display;

use strum_macros::{EnumCount, VariantArray};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumCount, VariantArray)]
pub enum Toy {
    Airplane = 0,
    Bee,
    Biscuit,
    Chicken,
    Cow,
    Fox,
    GrassBlock,
    Parrot,
    Pickaxe,
    Pig,
    PotionBlue,
    PotionPurple,
    PotionRed,
    Sheep,
    SteveHeart,
    Tree,
}

impl Toy {
    const BASE_PATH: &'static str = "./xpd-card-resources/icons";

    /// # Errors
    /// This function can error when the underlying filesystem read fails.
    pub fn load_png(&self) -> Result<Vec<u8>, std::io::Error> {
        let path = format!("{}/{}/{}", Self::BASE_PATH, self.author(), self.filename());
        std::fs::read(path)
    }

    #[must_use]
    pub const fn author(&self) -> &'static str {
        match self {
            Self::Airplane => "valkyrie_pilot",
            Self::Bee
            | Self::Biscuit
            | Self::Chicken
            | Self::Cow
            | Self::Fox
            | Self::GrassBlock
            | Self::Parrot
            | Self::Pickaxe
            | Self::Pig
            | Self::PotionBlue
            | Self::PotionPurple
            | Self::PotionRed
            | Self::Sheep
            | Self::SteveHeart
            | Self::Tree => "Cyana",
        }
    }

    #[must_use]
    pub const fn filename(&self) -> &'static str {
        match self {
            Self::Bee => "bee.png",
            Self::Biscuit => "biscuit.png",
            Self::Chicken => "chicken.png",
            Self::Cow => "cow.png",
            Self::Fox => "fox.png",
            Self::GrassBlock => "grassblock.png",
            Self::Parrot => "parrot.png",
            Self::Pickaxe => "pickaxe.png",
            Self::Pig => "pig.png",
            Self::PotionBlue => "potion_blue.png",
            Self::PotionPurple => "potion_purple.png",
            Self::PotionRed => "potion_red.png",
            Self::Sheep => "sheep.png",
            Self::SteveHeart => "steveheart.png",
            Self::Tree => "tree.png",
            Self::Airplane => "airplane.png",
        }
    }

    #[must_use]
    pub fn from_filename(data: &str) -> Option<Self> {
        let out = match data {
            "bee.png" => Self::Bee,
            "biscuit.png" => Self::Biscuit,
            "chicken.png" => Self::Chicken,
            "cow.png" => Self::Cow,
            "fox.png" => Self::Fox,
            "grassblock.png" => Self::GrassBlock,
            "parrot.png" => Self::Parrot,
            "pickaxe.png" => Self::Pickaxe,
            "pig.png" => Self::Pig,
            "potion_blue.png" => Self::PotionBlue,
            "potion_purple.png" => Self::PotionPurple,
            "potion_red.png" => Self::PotionRed,
            "sheep.png" => Self::Sheep,
            "steveheart.png" => Self::SteveHeart,
            "tree.png" => Self::Tree,
            "airplane.png" => Self::Airplane,
            _ => return None,
        };
        Some(out)
    }
}

impl Display for Toy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Bee => "Bee",
            Self::Biscuit => "Biscuit",
            Self::Chicken => "Chicken",
            Self::Cow => "Cow",
            Self::Fox => "Fox",
            Self::GrassBlock => "Grass Block",
            Self::Parrot => "Parrot",
            Self::Pickaxe => "Pickaxe",
            Self::Pig => "Pig",
            Self::PotionBlue => "Blue Potion Bottle",
            Self::PotionPurple => "Purple Potion Bottle",
            Self::PotionRed => "Red Potion Bottle",
            Self::Sheep => "Sheep",
            Self::SteveHeart => "Steve Heart",
            Self::Tree => "Tree",
            Self::Airplane => "Airplane",
        };
        f.write_str(name)
    }
}

impl serde::Serialize for Toy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.filename())
    }
}
