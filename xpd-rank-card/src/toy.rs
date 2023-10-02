use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toy {
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
    Airplane,
}

impl Toy {
    #[must_use]
    pub const fn png(&self) -> &'static [u8] {
        match self {
            Self::Bee => include_bytes!("resources/icons/CEa_TIde/bee.png"),
            Self::Biscuit => include_bytes!("resources/icons/CEa_TIde/biscuit.png"),
            Self::Chicken => include_bytes!("resources/icons/CEa_TIde/chicken.png"),
            Self::Cow => include_bytes!("resources/icons/CEa_TIde/cow.png"),
            Self::Fox => include_bytes!("resources/icons/CEa_TIde/fox.png"),
            Self::GrassBlock => include_bytes!("resources/icons/CEa_TIde/grassblock.png"),
            Self::Parrot => include_bytes!("resources/icons/CEa_TIde/parrot.png"),
            Self::Pickaxe => include_bytes!("resources/icons/CEa_TIde/pickaxe.png"),
            Self::Pig => include_bytes!("resources/icons/CEa_TIde/pig.png"),
            Self::PotionBlue => include_bytes!("resources/icons/CEa_TIde/potion_blue.png"),
            Self::PotionPurple => include_bytes!("resources/icons/CEa_TIde/potion_purple.png"),
            Self::PotionRed => include_bytes!("resources/icons/CEa_TIde/potion_red.png"),
            Self::Sheep => include_bytes!("resources/icons/CEa_TIde/sheep.png"),
            Self::SteveHeart => include_bytes!("resources/icons/CEa_TIde/steveheart.png"),
            Self::Tree => include_bytes!("resources/icons/CEa_TIde/tree.png"),
            Self::Airplane => include_bytes!("resources/icons/valkyrie_pilot/airplane.png"),
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
