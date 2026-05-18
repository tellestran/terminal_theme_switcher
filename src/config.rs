use std::{fs, io, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::theme;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    #[serde(default)]
    pub custom_themes: Vec<CustomTheme>,
    #[serde(default)]
    pub builtin_overrides: Vec<BuiltinOverride>,
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
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub mood: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltinOverride {
    pub name: String,
    pub slug: String,
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,
    pub ansi: [String; 16],
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub mood: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTheme {
    pub name: String,
    pub slug: String,
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,
    pub ansi: [String; 16],
    pub source: ResolvedThemeSource,
    pub description: String,
    pub author: String,
    pub kind: String,
    pub mood: Vec<String>,
    pub accent: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedThemeSource {
    BuiltIn,
    BuiltInOverride,
    Custom,
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
            builtin_overrides: Vec::new(),
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

pub fn save_builtin_override(override_theme: BuiltinOverride) -> Result<()> {
    let mut config = Config::load()?;
    config
        .builtin_overrides
        .retain(|theme| theme.slug != override_theme.slug);
    config.theme = override_theme.slug.clone();
    config.builtin_overrides.push(override_theme);
    config.save()
}

pub fn delete_custom_theme(slug: &str) -> Result<bool> {
    let mut config = Config::load()?;
    let before = config.custom_themes.len();
    config.custom_themes.retain(|theme| theme.slug != slug);
    let deleted = config.custom_themes.len() != before;

    if deleted && config.theme == slug {
        config.theme = theme::default_theme().slug.to_string();
    }

    if deleted {
        config.save()?;
    }

    Ok(deleted)
}

pub fn delete_builtin_override(slug: &str) -> Result<bool> {
    let mut config = Config::load()?;
    let before = config.builtin_overrides.len();
    config.builtin_overrides.retain(|theme| theme.slug != slug);
    let deleted = config.builtin_overrides.len() != before;

    if deleted {
        config.save()?;
    }

    Ok(deleted)
}

impl Config {
    pub fn custom_theme_by_slug(&self, slug: &str) -> Option<&CustomTheme> {
        self.custom_themes.iter().find(|theme| theme.slug == slug)
    }

    pub fn builtin_override_by_slug(&self, slug: &str) -> Option<&BuiltinOverride> {
        self.builtin_overrides
            .iter()
            .find(|theme| theme.slug == slug)
    }

    pub fn resolved_themes(&self) -> Vec<ResolvedTheme> {
        let mut themes = Vec::new();

        for built_in in theme::themes() {
            if let Some(override_theme) = self.builtin_override_by_slug(built_in.slug) {
                themes.push(ResolvedTheme {
                    name: override_theme.name.clone(),
                    slug: override_theme.slug.clone(),
                    foreground: override_theme.foreground.clone(),
                    background: override_theme.background.clone(),
                    cursor: override_theme.cursor.clone(),
                    selection: override_theme.selection.clone(),
                    ansi: override_theme.ansi.clone(),
                    source: ResolvedThemeSource::BuiltInOverride,
                    description: override_theme.description.clone(),
                    author: override_theme.author.clone(),
                    kind: override_theme.kind.clone(),
                    mood: override_theme.mood.clone(),
                    accent: override_theme.accent.clone(),
                    success: override_theme.success.clone(),
                    warning: override_theme.warning.clone(),
                    error: override_theme.error.clone(),
                });
            } else {
                themes.push(ResolvedTheme {
                    name: built_in.name.to_string(),
                    slug: built_in.slug.to_string(),
                    foreground: built_in.foreground.to_string(),
                    background: built_in.background.to_string(),
                    cursor: built_in.cursor.to_string(),
                    selection: built_in.selection.to_string(),
                    ansi: built_in.ansi.map(str::to_string),
                    source: ResolvedThemeSource::BuiltIn,
                    description: built_in.description.to_string(),
                    author: built_in.author.to_string(),
                    kind: built_in.kind.to_string(),
                    mood: built_in.mood.iter().map(|s| s.to_string()).collect(),
                    accent: None,
                    success: None,
                    warning: None,
                    error: None,
                });
            }
        }

        themes.extend(self.custom_themes.iter().map(|custom| ResolvedTheme {
            name: custom.name.clone(),
            slug: custom.slug.clone(),
            foreground: custom.foreground.clone(),
            background: custom.background.clone(),
            cursor: custom.cursor.clone(),
            selection: custom.selection.clone(),
            ansi: custom.ansi.clone(),
            source: ResolvedThemeSource::Custom,
            description: custom.description.clone(),
            author: custom.author.clone(),
            kind: custom.kind.clone(),
            mood: custom.mood.clone(),
            accent: custom.accent.clone(),
            success: custom.success.clone(),
            warning: custom.warning.clone(),
            error: custom.error.clone(),
        }));

        themes
    }

    pub fn resolved_theme_by_query(&self, query: &str) -> Option<ResolvedTheme> {
        let normalized = normalize(query);
        self.resolved_themes()
            .into_iter()
            .find(|theme| theme.slug == normalized || normalize(&theme.name) == normalized)
    }
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn setup_temp_config_home() -> TempDir {
        let temp = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        std::env::set_var("HOME", temp.path());
        temp
    }

    fn test_custom(name: &str, slug: &str) -> CustomTheme {
        CustomTheme {
            name: name.to_string(),
            slug: slug.to_string(),
            foreground: "#FFFFFF".to_string(),
            background: "#101010".to_string(),
            cursor: "#DDDDDD".to_string(),
            selection: "#333333".to_string(),
            ansi: std::array::from_fn(|i| format!("#{:02X}{:02X}{:02X}", i * 17, i * 17, i * 17)),
            description: String::new(),
            author: "test".to_string(),
            kind: "custom".to_string(),
            mood: Vec::new(),
            accent: None,
            success: None,
            warning: None,
            error: None,
        }
    }

    fn test_override(name: &str, slug: &str) -> BuiltinOverride {
        BuiltinOverride {
            name: name.to_string(),
            slug: slug.to_string(),
            foreground: "#FFFFFF".to_string(),
            background: "#111111".to_string(),
            cursor: "#EEEEEE".to_string(),
            selection: "#333333".to_string(),
            ansi: std::array::from_fn(|_| "#444444".to_string()),
            description: String::new(),
            author: String::new(),
            kind: String::new(),
            mood: Vec::new(),
            accent: None,
            success: None,
            warning: None,
            error: None,
        }
    }

    #[test]
    fn config_round_trips_as_toml() {
        let _guard = env_lock().lock().unwrap();
        let config = Config {
            theme: "dracula".to_string(),
            custom_themes: vec![test_custom("My Theme", "my-theme")],
            builtin_overrides: vec![test_override("Tokyo Night Custom", "tokyo-night")],
        };

        let encoded = toml::to_string(&config).unwrap();
        let decoded: Config = toml::from_str(&encoded).unwrap();

        assert_eq!(decoded, config);
    }

    #[test]
    fn save_custom_theme_upserts_by_slug() {
        let _guard = env_lock().lock().unwrap();
        let _temp = setup_temp_config_home();

        save_custom_theme(test_custom("Ocean", "ocean")).unwrap();

        let mut v2 = test_custom("Ocean v2", "ocean");
        v2.foreground = "#FFD786".to_string();
        v2.background = "#237227".to_string();
        save_custom_theme(v2).unwrap();

        let config = Config::load().unwrap();
        assert_eq!(config.custom_themes.len(), 1);
        assert_eq!(config.custom_themes[0].name, "Ocean v2");
        assert_eq!(config.theme, "ocean");
    }

    #[test]
    fn delete_custom_theme_removes_and_resets_active_if_needed() {
        let _guard = env_lock().lock().unwrap();
        let _temp = setup_temp_config_home();
        save_custom_theme(test_custom("Ocean", "ocean")).unwrap();

        let deleted = delete_custom_theme("ocean").unwrap();
        assert!(deleted);

        let config = Config::load().unwrap();
        assert!(config
            .custom_themes
            .iter()
            .all(|theme| theme.slug != "ocean"));
        assert_ne!(config.theme, "ocean");
    }

    #[test]
    fn save_builtin_override_upserts_and_resolves() {
        let _guard = env_lock().lock().unwrap();
        let _temp = setup_temp_config_home();

        save_builtin_override(test_override("Tokyo Night Reworked", "tokyo-night")).unwrap();

        let config = Config::load().unwrap();
        let resolved = config
            .resolved_themes()
            .into_iter()
            .find(|theme| theme.slug == "tokyo-night")
            .unwrap();

        assert_eq!(resolved.name, "Tokyo Night Reworked");
        assert_eq!(resolved.source, ResolvedThemeSource::BuiltInOverride);
    }
}
