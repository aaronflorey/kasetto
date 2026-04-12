/// Default config file in the current directory when `--config` is omitted.
pub(crate) const DEFAULT_CONFIG_FILENAME: &str = "kasetto.yaml";
/// Default config file under the Kasetto XDG config directory.
pub(crate) const DEFAULT_GLOBAL_CONFIG_FILENAME: &str = "config.yaml";

/// Resolve the default config path used when `--config` is omitted.
///
/// Priority: local `kasetto.yaml` in CWD, then `$XDG_CONFIG_HOME/kasetto/config.yaml`
/// (or `$HOME/.config/kasetto/config.yaml`), then local `kasetto.yaml` fallback.
pub(crate) fn default_config_path() -> String {
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
