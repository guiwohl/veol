use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_image_protocol")]
    pub image_protocol: String,
    #[serde(default = "default_mermaid")]
    pub mermaid: bool,
    #[serde(default = "default_mermaid_timeout_ms")]
    pub mermaid_timeout_ms: u64,
    #[serde(default = "default_show_statusline")]
    pub show_statusline: bool,
    #[serde(default)]
    pub show_toc: bool,
    #[serde(default = "default_wrap")]
    pub wrap: bool,
    #[serde(default = "default_frontmatter")]
    pub frontmatter: bool,
    #[serde(default = "default_watch")]
    pub watch: bool,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub keymap: KeymapConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheConfig {
    #[serde(default)]
    pub dir: Option<PathBuf>,
    #[serde(default = "default_cache_max_size_mb")]
    pub max_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeymapConfig {
    #[serde(default = "default_key_quit")]
    pub quit: String,
    #[serde(default = "default_key_search")]
    pub search: String,
    #[serde(default = "default_key_toggle_toc")]
    pub toggle_toc: String,
    #[serde(default = "default_key_toggle_mermaid")]
    pub toggle_mermaid: String,
    #[serde(default = "default_key_toggle_browser")]
    pub toggle_browser: String,
    #[serde(default = "default_key_toggle_theme")]
    pub toggle_theme: String,
}

fn default_theme() -> String {
    "reedo-dark".into()
}
fn default_image_protocol() -> String {
    "auto".into()
}
fn default_mermaid() -> bool {
    true
}
fn default_mermaid_timeout_ms() -> u64 {
    1500
}
fn default_show_statusline() -> bool {
    true
}
fn default_wrap() -> bool {
    true
}
fn default_frontmatter() -> bool {
    true
}
fn default_watch() -> bool {
    true
}
fn default_cache_max_size_mb() -> u64 {
    128
}
fn default_key_quit() -> String {
    "q".into()
}
fn default_key_search() -> String {
    "/".into()
}
fn default_key_toggle_toc() -> String {
    "t".into()
}
fn default_key_toggle_mermaid() -> String {
    "m".into()
}
fn default_key_toggle_browser() -> String {
    "Ctrl+E".into()
}
fn default_key_toggle_theme() -> String {
    "Ctrl+T".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            image_protocol: default_image_protocol(),
            mermaid: default_mermaid(),
            mermaid_timeout_ms: default_mermaid_timeout_ms(),
            show_statusline: default_show_statusline(),
            show_toc: false,
            wrap: default_wrap(),
            frontmatter: default_frontmatter(),
            watch: default_watch(),
            cache: CacheConfig::default(),
            keymap: KeymapConfig::default(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: None,
            max_size_mb: default_cache_max_size_mb(),
        }
    }
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            quit: default_key_quit(),
            search: default_key_search(),
            toggle_toc: default_key_toggle_toc(),
            toggle_mermaid: default_key_toggle_mermaid(),
            toggle_browser: default_key_toggle_browser(),
            toggle_theme: default_key_toggle_theme(),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_env_lock() -> &'static std::sync::Mutex<()> {
    use std::sync::OnceLock;
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

pub fn veol_config_dir() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "veol") {
        return dirs.config_dir().to_path_buf();
    }
    PathBuf::from(".").join(".config").join("veol")
}

pub fn config_path() -> PathBuf {
    veol_config_dir().join("config.toml")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(cfg) => {
                    tracing::info!("loaded config from {}", path.display());
                    cfg
                }
                Err(e) => {
                    tracing::warn!("failed to parse config at {}: {e}", path.display());
                    Self::default()
                }
            },
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("failed to read config at {}: {e}", path.display());
                }
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("config path has no parent"))?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
        let body = toml::to_string_pretty(self).context("serializing config")?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)
            .with_context(|| format!("creating temp file in {}", parent.display()))?;
        tmp.write_all(body.as_bytes())
            .context("writing config tempfile")?;
        tmp.flush().context("flushing config tempfile")?;
        tmp.persist(&path)
            .map_err(|e| anyhow::anyhow!("persisting config: {e}"))?;
        Ok(())
    }

    pub fn update_theme(&mut self, name: &str) -> Result<()> {
        self.theme = name.to_string();
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_config_home<R>(dir: &std::path::Path, f: impl FnOnce() -> R) -> R {
        let _guard = test_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", dir);
        let out = f();
        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        out
    }

    #[test]
    fn default_has_reedo_dark_theme() {
        let cfg = Config::default();
        assert_eq!(cfg.theme, "reedo-dark");
        assert_eq!(cfg.image_protocol, "auto");
        assert!(cfg.mermaid);
        assert_eq!(cfg.mermaid_timeout_ms, 1500);
        assert!(cfg.show_statusline);
        assert!(!cfg.show_toc);
        assert!(cfg.wrap);
        assert!(cfg.frontmatter);
        assert!(cfg.watch);
        assert_eq!(cfg.cache.max_size_mb, 128);
        assert_eq!(cfg.keymap.quit, "q");
        assert_eq!(cfg.keymap.toggle_browser, "Ctrl+E");
        assert_eq!(cfg.keymap.toggle_theme, "Ctrl+T");
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            let cfg = Config::load();
            assert_eq!(cfg, Config::default());
        });
    }

    #[test]
    fn malformed_toml_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            let path = config_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "this is = not valid [[[ toml").unwrap();
            let cfg = Config::load();
            assert_eq!(cfg, Config::default());
        });
    }

    #[test]
    fn round_trip_save_load_equal() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            let cfg = Config {
                theme: "dracula".into(),
                cache: CacheConfig {
                    max_size_mb: 256,
                    ..CacheConfig::default()
                },
                ..Config::default()
            };
            cfg.save().expect("save");
            let loaded = Config::load();
            assert_eq!(cfg, loaded);
        });
    }

    #[test]
    fn update_theme_persists() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            let mut cfg = Config::default();
            cfg.update_theme("catppuccin").expect("update theme");
            let loaded = Config::load();
            assert_eq!(loaded.theme, "catppuccin");
        });
    }

    #[test]
    fn partial_config_uses_defaults_for_missing_fields() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            let path = config_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "theme = \"gruvbox\"\n").unwrap();
            let cfg = Config::load();
            assert_eq!(cfg.theme, "gruvbox");
            assert_eq!(cfg.mermaid_timeout_ms, 1500);
            assert!(cfg.watch);
        });
    }
}
