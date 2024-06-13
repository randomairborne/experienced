use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    pub fonts: Vec<ConfigItem>,
    pub toys: Vec<ConfigItem>,
    pub cards: Vec<ConfigItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConfigItem {
    pub file: PathBuf,
    pub internal_name: String,
    pub display_name: String,
}
