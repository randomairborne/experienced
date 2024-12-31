use std::path::PathBuf;

use serde::Deserialize;

use crate::customizations::Customizations;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    pub defaults: Defaults,
    pub fonts: Vec<ConfigItem>,
    pub toys: Vec<ConfigItem>,
    pub cards: Vec<CardItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Defaults {
    pub card: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConfigItem {
    pub file: PathBuf,
    pub internal_name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CardItem {
    pub file: PathBuf,
    pub display_name: String,
    #[serde(flatten)]
    pub customizations: Customizations,
}

pub trait NameableItem {
    fn display_name(&self) -> &str;
    fn internal_name(&self) -> &str;
}

impl NameableItem for ConfigItem {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn internal_name(&self) -> &str {
        &self.internal_name
    }
}

impl NameableItem for CardItem {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn internal_name(&self) -> &str {
        &self.customizations.internal_name
    }
}
