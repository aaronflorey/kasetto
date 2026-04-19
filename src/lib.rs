/// Default config file in the current directory when `--config` is omitted.
pub(crate) const DEFAULT_CONFIG_FILENAME: &str = "kasetto.yaml";
/// Default config file under the Kasetto XDG config directory (`init --global` writes here).
pub(crate) const DEFAULT_GLOBAL_CONFIG_FILENAME: &str = "kasetto.yaml";
/// Kasetto preferences file under the XDG config directory.
/// May contain a `config:` key pointing to a remote or absolute config path.
pub(crate) const PREFERENCES_FILENAME: &str = "config.yaml";

#[derive(serde::Deserialize)]
struct Preferences {
    source: Option<String>,
}

/// Resolve the default config path used when `--config` is omitted.
///
/// Priority:
/// 1. `$KASETTO_CONFIG` env var
/// 2. `config:` key in `$XDG_CONFIG_HOME/kasetto/config.yaml` (saved preference)
/// 3. `./kasetto.yaml` (local project config)
/// 4. `$XDG_CONFIG_HOME/kasetto/kasetto.yaml` (global config)
/// 5. `./kasetto.yaml` fallback
pub(crate) fn default_config_path() -> String {
    if let Ok(v) = std::env::var("KASETTO_CONFIG") {
        if !v.is_empty() {
            return v;
        }
    }

    if let Ok(prefs_path) =
        crate::fsops::dirs_kasetto_config().map(|d| d.join(PREFERENCES_FILENAME))
    {
        if let Ok(text) = std::fs::read_to_string(&prefs_path) {
            if let Ok(prefs) = serde_yaml::from_str::<Preferences>(&text) {
                if let Some(cfg) = prefs.source {
                    return cfg;
                }
            }
        }
    }

    if std::path::Path::new(DEFAULT_CONFIG_FILENAME).exists() {
        return DEFAULT_CONFIG_FILENAME.to_string();
    }

    if let Ok(global) =
        crate::fsops::dirs_kasetto_config().map(|dir| dir.join(DEFAULT_GLOBAL_CONFIG_FILENAME))
    {
        if global.exists() {
            return global.to_string_lossy().to_string();
        }
    }

    DEFAULT_CONFIG_FILENAME.to_string()
}

mod app;
mod banner;
mod cli;
mod colors;
mod commands;
mod error;
mod fsops;
mod home;
mod list;
mod lock;
mod mcps;
mod model;
mod profile;
mod source;
mod tui;
mod ui;

pub use app::run;
pub use error::Result;
