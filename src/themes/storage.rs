use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::themes::builtin;
use crate::themes::palette::{RawTheme, Theme};

#[derive(Debug, Deserialize)]
struct ThemeSelection {
    theme: Option<String>,
}

/// Loaded theme plus an optional user-facing load error.
pub struct LoadedTheme {
    /// Theme selected for rendering.
    pub theme: Theme,
    /// User-facing load error when storage falls back to the compiled theme.
    pub error: Option<String>,
}

/// Returns the default theme config path.
fn theme_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("lazysql")
        .join("theme.toml")
}

/// Creates a default theme config file when it does not exist.
fn ensure_theme_file(path: &Path) -> Result<(), std::io::Error> {
    if path.exists() {
        return Ok(());
    }

    write_theme_file(path, "gruvbox")
}

pub fn load(themes: &[Theme]) -> LoadedTheme {
    load_from(&theme_path(), themes)
}

/// Loads the selected or inline theme from the given path.
fn load_from(path: &Path, themes: &[Theme]) -> LoadedTheme {
    if let Err(error) = ensure_theme_file(path) {
        return LoadedTheme {
            theme: builtin::fallback_theme(),
            error: Some(format!("failed to create theme config: {error}")),
        };
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            return LoadedTheme {
                theme: builtin::fallback_theme(),
                error: Some(format!("failed to read theme config: {error}")),
            };
        }
    };

    match toml::from_str::<ThemeSelection>(&content) {
        Ok(selection) => {
            if let Some(name) = selection.theme {
                return load_builtin(themes, &name);
            }
        }
        Err(error) => {
            return LoadedTheme {
                theme: builtin::fallback_theme(),
                error: Some(format!("failed to parse theme config: {error}")),
            };
        }
    }

    load_inline(&content)
}

/// Saves the selected built-in theme name to the default theme config path.
pub fn save_selected(name: &str) -> Result<(), std::io::Error> {
    let path: &Path = &theme_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, theme_file_content(name))
}

/// Attempts to load a built-in theme by name. If the name is unknown, falls back to a built-in theme with an error message.
fn load_builtin(themes: &[Theme], name: &str) -> LoadedTheme {
    let Some(theme) = builtin::find_by_name(themes, name) else {
        return LoadedTheme {
            theme: builtin::fallback_theme(),
            error: Some(format!("unknown theme: {name}")),
        };
    };

    LoadedTheme { theme, error: None }
}

/// Attempts to load an inline custom theme from the config content. If parsing or validation fails, falls back to a built-in theme with an error message.
fn load_inline(content: &str) -> LoadedTheme {
    let raw = match toml::from_str::<RawTheme>(content) {
        Ok(raw) => raw,
        Err(error) => {
            return LoadedTheme {
                theme: builtin::fallback_theme(),
                error: Some(format!("failed to parse custom theme: {error}")),
            };
        }
    };

    let theme = match Theme::try_from(raw) {
        Ok(theme) => theme,
        Err(error) => {
            return LoadedTheme {
                theme: builtin::fallback_theme(),
                error: Some(format!("invalid custom theme: {error}")),
            };
        }
    };

    LoadedTheme { theme, error: None }
}

/// Merges builtin load error and storage load error into a single user-facing message.
pub fn combine_errors(builtin: Option<String>, storage: Option<String>) -> Option<String> {
    match (builtin, storage) {
        (Some(b), Some(s)) => Some(format!("{b}; {s}")),
        (Some(b), None) => Some(b),
        (None, s) => s,
    }
}

fn write_theme_file(path: &Path, selected: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, theme_file_content(selected))
}

fn theme_file_content(selected: &str) -> String {
    format!(
        r##"theme = "{selected}"

# Built-in themes
# Set `theme` to one of the built-in theme names.
# theme = "gruvbox"
# theme = "gruvbox-material"
# theme = "gruvbox-baby"
# theme = "everforest"
# theme = "dracula"
# theme = "tokyo-night"
# theme = "catppuccin-mocha"

# Full custom theme example
# Remove the `theme` line above and uncomment this block to use an inline theme.
# name = "custom"
#
# [colors]
# bg0 = "#1d2021"
# bg1 = "#282828"
# bg3 = "#3c3836"
# bg_sel = "#504945"
# fg0 = "#ebdbb2"
# fg3 = "#a89984"
# fg4 = "#7c6f64"
# red = "#fb4934"
# green = "#b8bb26"
# yellow = "#fabd2f"
# blue = "#83a598"
# aqua = "#8ec07c"
# orange = "#fe8019"
# purple = "#d3869b"
"##
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::themes::palette::{RawTheme, Theme};
    use std::fs;
    use std::path::Path;

    fn custom_theme_toml(name: &str) -> String {
        format!(
            r##"
            name = "{name}"

            [colors]
            primary = "#282828"
            secondary = "#ebdbb2"
            bg0 = "#000000"
            bg1 = "#111111"
            bg2 = "#333333"
            bg_sel = "#444444"
            fg0 = "#ffffff"
            fg1 = "#c0c0c0"
            fg2 = "#808080"
            red = "#ff5555"
            green = "#50fa7b"
            yellow = "#f1fa8c"
            blue = "#8be9fd"
            aqua = "#8be9fd"
            orange = "#ffb86c"
            purple = "#bd93f9"
            "##
        )
    }

    fn custom_theme(name: &str) -> Theme {
        let raw = toml::from_str::<RawTheme>(&custom_theme_toml(name)).unwrap();
        Theme::try_from(raw).unwrap()
    }

    #[test]
    fn creates_default_theme_config_with_examples() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.toml");

        ensure_theme_file(&path).unwrap();

        let content = fs::read_to_string(path).unwrap();
        assert!(content.starts_with("theme = \"gruvbox\""));
        assert!(content.contains("# Built-in themes"));
        assert!(content.contains("# theme = \"gruvbox\""));
        assert!(content.contains("# theme = \"gruvbox-material\""));
        assert!(content.contains("# theme = \"gruvbox-baby\""));
        assert!(content.contains("# theme = \"everforest\""));
        assert!(content.contains("# theme = \"dracula\""));
        assert!(content.contains("# theme = \"tokyo-night\""));
        assert!(content.contains("# theme = \"catppuccin-mocha\""));
        assert!(content.contains("# Full custom theme example"));
        assert!(content.contains("# [colors]"));
    }

    #[test]
    fn loads_selected_builtin_theme() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.toml");
        fs::write(&path, "theme = \"solarized\"\n").unwrap();
        let themes = vec![custom_theme("gruvbox"), custom_theme("solarized")];

        let loaded = load_from(&path, &themes);

        assert_eq!(loaded.theme.name, "solarized");
        assert_eq!(loaded.error, None);
    }

    #[test]
    fn falls_back_for_unknown_builtin_theme_with_error_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.toml");
        fs::write(&path, "theme = \"missing\"\n").unwrap();
        let themes = vec![custom_theme("gruvbox")];

        let loaded = load_from(&path, &themes);

        assert_eq!(loaded.theme.name, "gruvbox");
        assert_eq!(loaded.error, Some("unknown theme: missing".to_string()));
    }

    #[test]
    fn loads_inline_custom_theme() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.toml");
        fs::write(&path, custom_theme_toml("midnight")).unwrap();
        let themes = vec![custom_theme("gruvbox")];

        let loaded = load_from(&path, &themes);

        assert_eq!(loaded.theme.name, "midnight");
        assert_eq!(loaded.error, None);
    }

    #[test]
    fn invalid_inline_custom_theme_falls_back_with_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.toml");
        fs::write(
            &path,
            r##"
            name = "broken"

            [colors]
            bg0 = "bad"
            "##,
        )
        .unwrap();
        let themes = vec![custom_theme("gruvbox")];

        let loaded = load_from(&path, &themes);

        assert_eq!(loaded.theme.name, "gruvbox");
        assert!(loaded.error.is_some());
    }

    #[test]
    fn theme_path_ends_with_config_lazysql_theme_toml() {
        let path = theme_path();

        assert!(path.ends_with(Path::new(".config").join("lazysql").join("theme.toml")));
    }
}
