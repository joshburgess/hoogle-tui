use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub backend: BackendConfig,
    pub ui: UiConfig,
    pub theme: String,
    pub cache: CacheConfig,
    pub keybinds: KeybindOverrides,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendConfig::default(),
            ui: UiConfig::default(),
            theme: "dracula".into(),
            cache: CacheConfig::default(),
            keybinds: KeybindOverrides::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackendConfig {
    pub mode: BackendMode,
    pub hoogle_path: Option<PathBuf>,
    pub database_path: Option<PathBuf>,
    pub web_url: String,
    pub timeout_secs: u64,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            mode: BackendMode::Auto,
            hoogle_path: None,
            database_path: None,
            web_url: "https://hoogle.haskell.org".into(),
            timeout_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub max_results: usize,
    pub preview_enabled: bool,
    pub layout: LayoutMode,
    pub mouse_enabled: bool,
    pub debounce_ms: u64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            max_results: 50,
            preview_enabled: true,
            layout: LayoutMode::Auto,
            mouse_enabled: true,
            debounce_ms: 150,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    pub enabled: bool,
    pub dir: Option<PathBuf>,
    pub ttl_hours: u64,
    pub max_size_mb: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: None,
            ttl_hours: 168,
            max_size_mb: 500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendMode {
    Auto,
    Local,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    Auto,
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeybindOverrides {
    #[serde(default)]
    pub overrides: HashMap<String, String>,
}

impl CacheConfig {
    pub fn cache_dir(&self) -> PathBuf {
        self.dir.clone().unwrap_or_else(|| {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hoogle-tui")
        })
    }
}

impl Config {
    /// Load configuration with the following precedence:
    /// 1. Compiled-in defaults
    /// 2. Config file overrides defaults
    /// 3. CLI args override file config (applied separately by caller)
    pub fn load(config_path: Option<&PathBuf>) -> Self {
        let path = config_path
            .cloned()
            .or_else(|| dirs::config_dir().map(|d| d.join("hoogle-tui").join("config.toml")));

        let Some(path) = path else {
            tracing::debug!("no config path found, using defaults");
            return Config::default();
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => {
                    tracing::info!("loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    tracing::warn!("failed to parse config at {}: {}", path.display(), e);
                    Config::default()
                }
            },
            Err(_) => {
                tracing::debug!("no config file at {}, using defaults", path.display());
                Config::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.theme, "dracula");
        assert_eq!(config.backend.mode, BackendMode::Auto);
        assert_eq!(config.ui.max_results, 50);
        assert!(config.cache.enabled);
        assert_eq!(config.cache.ttl_hours, 168);
    }

    #[test]
    fn toml_deserialization_partial() {
        let toml_str = r#"
            theme = "nord"

            [backend]
            mode = "web"
            timeout_secs = 10

            [ui]
            max_results = 25
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.theme, "nord");
        assert_eq!(config.backend.mode, BackendMode::Web);
        assert_eq!(config.backend.timeout_secs, 10);
        assert_eq!(config.backend.web_url, "https://hoogle.haskell.org");
        assert_eq!(config.ui.max_results, 25);
        assert!(config.ui.preview_enabled); // default
    }

    #[test]
    fn toml_deserialization_full_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.theme, config.theme);
        assert_eq!(deserialized.backend.mode, config.backend.mode);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let toml_str = r#"
            theme = "catppuccin_mocha"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.theme, "catppuccin_mocha");
        assert_eq!(config.ui.debounce_ms, 150);
        assert_eq!(config.cache.max_size_mb, 500);
    }

    #[test]
    fn cache_dir_default() {
        let config = CacheConfig::default();
        let dir = config.cache_dir();
        assert!(dir.to_string_lossy().contains("hoogle-tui"));
    }

    #[test]
    fn cache_dir_custom() {
        let config = CacheConfig {
            dir: Some(PathBuf::from("/tmp/my-cache")),
            ..Default::default()
        };
        assert_eq!(config.cache_dir(), PathBuf::from("/tmp/my-cache"));
    }
}
