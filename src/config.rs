use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub preserve_header: Option<bool>,
    pub output_dir: Option<String>,
    pub prefix: Option<String>,
    pub dry_run: Option<bool>,
}

impl Config {
    pub fn load(path: &str) -> Self {
        if !Path::new(path).exists() {
            return Config::default();
        }
        let content = std::fs::read_to_string(path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }
}
