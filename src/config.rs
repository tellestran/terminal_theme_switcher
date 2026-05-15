use std::{fs, io, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::theme;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let contents = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, contents)
            .with_context(|| format!("failed to write {}", path.display()))?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: theme::default_theme().slug.to_string(),
        }
    }
}

pub fn save_selected_theme(theme: &theme::Theme) -> Result<()> {
    Config {
        theme: theme.slug.to_string(),
    }
    .save()
}

pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not find user config directory",
        )
    })?;

    Ok(config_dir.join("switch-theme").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_as_toml() {
        let config = Config {
            theme: "dracula".to_string(),
        };

        let encoded = toml::to_string(&config).unwrap();
        let decoded: Config = toml::from_str(&encoded).unwrap();

        assert_eq!(decoded, config);
    }
}
