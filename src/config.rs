use std::{fs, io, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::theme;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    #[serde(default)]
    pub custom_themes: Vec<CustomTheme>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTheme {
    pub name: String,
    pub slug: String,
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,
    pub ansi: [String; 16],
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
            custom_themes: Vec::new(),
        }
    }
}

pub fn save_selected_theme(theme: &theme::Theme) -> Result<()> {
    let mut config = Config::load()?;
    config.theme = theme.slug.to_string();
    config.save()
}

pub fn save_custom_theme(custom: CustomTheme) -> Result<()> {
    let mut config = Config::load()?;
    config
        .custom_themes
        .retain(|theme| theme.slug != custom.slug);
    config.theme = custom.slug.clone();
    config.custom_themes.push(custom);
    config.save()
}

impl Config {
    pub fn custom_theme_by_slug(&self, slug: &str) -> Option<&CustomTheme> {
        self.custom_themes.iter().find(|theme| theme.slug == slug)
    }
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
    use tempfile::TempDir;

    fn setup_temp_config_home() -> TempDir {
        let temp = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        std::env::set_var("HOME", temp.path());
        temp
    }

    #[test]
    fn config_round_trips_as_toml() {
        let config = Config {
            theme: "dracula".to_string(),
            custom_themes: vec![CustomTheme {
                name: "My Theme".to_string(),
                slug: "my-theme".to_string(),
                foreground: "#FFFFFF".to_string(),
                background: "#101010".to_string(),
                cursor: "#DDDDDD".to_string(),
                selection: "#333333".to_string(),
                ansi: [
                    "#000000".to_string(),
                    "#111111".to_string(),
                    "#222222".to_string(),
                    "#333333".to_string(),
                    "#444444".to_string(),
                    "#555555".to_string(),
                    "#666666".to_string(),
                    "#777777".to_string(),
                    "#888888".to_string(),
                    "#999999".to_string(),
                    "#AAAAAA".to_string(),
                    "#BBBBBB".to_string(),
                    "#CCCCCC".to_string(),
                    "#DDDDDD".to_string(),
                    "#EEEEEE".to_string(),
                    "#FFFFFF".to_string(),
                ],
            }],
        };

        let encoded = toml::to_string(&config).unwrap();
        let decoded: Config = toml::from_str(&encoded).unwrap();

        assert_eq!(decoded, config);
    }

    #[test]
    fn save_custom_theme_upserts_by_slug() {
        let _temp = setup_temp_config_home();

        save_custom_theme(CustomTheme {
            name: "Ocean".to_string(),
            slug: "ocean".to_string(),
            foreground: "#FFFFFF".to_string(),
            background: "#001122".to_string(),
            cursor: "#EEEEEE".to_string(),
            selection: "#223344".to_string(),
            ansi: std::array::from_fn(|_| "#111111".to_string()),
        })
        .unwrap();

        save_custom_theme(CustomTheme {
            name: "Ocean v2".to_string(),
            slug: "ocean".to_string(),
            foreground: "#FFD786".to_string(),
            background: "#237227".to_string(),
            cursor: "#FFF0C2".to_string(),
            selection: "#2F6E5A".to_string(),
            ansi: std::array::from_fn(|_| "#222222".to_string()),
        })
        .unwrap();

        let config = Config::load().unwrap();
        assert_eq!(config.custom_themes.len(), 1);
        assert_eq!(config.custom_themes[0].name, "Ocean v2");
        assert_eq!(config.theme, "ocean");
    }
}
