use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_FILENAME: &str = "lucius_config.toml";

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Config {
    pub ollama_url: Option<String>,
    pub selected_model: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        log::info!("Loading config from: {}", config_path.display());
        match fs::read_to_string(&config_path) {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
                log::error!("Failed to parse config file: {}. Using default config. Error: {}", config_path.display(), e);
                Self::default()
            }),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::error!("Failed to read config file: {}. Using default config. Error: {}", config_path.display(), e);
                } else {
                    log::info!("Config file not found. Using default config.");
                }
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        let config_path = Self::get_config_path();
        log::info!("Saving config to: {}", config_path.display());
        let toml_string = toml::to_string_pretty(self).expect("Failed to serialize config to TOML");
        if let Err(e) = fs::write(&config_path, toml_string) {
            log::error!("Failed to write config file: {}. Error: {}", config_path.display(), e);
        }
    }

    fn get_config_path() -> PathBuf {
        let mut path = match dirs::config_dir() {
            Some(dir) => dir,
            None => {
                log::warn!("Could not find config directory, falling back to current directory.");
                PathBuf::from(".")
            }
        };
        path.push("lucius"); // Create a lucius subdirectory in config_dir
        fs::create_dir_all(&path).ok(); // Ensure the directory exists
        path.push(CONFIG_FILENAME);
        log::info!("Config path resolved to: {}", path.display());
        path
    }
}
