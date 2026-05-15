use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::themes::builtin;
use crate::themes::palette::{RawTheme, Theme, gruvbox};

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
pub fn theme_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("lazysql")
        .join("theme.toml")
}

/// Loads the selected or inline theme from the default theme config path.
pub fn load(themes: &[Theme]) -> LoadedTheme {
    load_from(&theme_path(), themes)
}

/// Creates a default theme config file when it does not exist.
pub fn ensure_theme_file(path: &Path) -> Result<(), std::io::Error> {
    if path.exists() {
        return Ok(());
    }

    write_theme_file(path, "gruvbox")
}

/// Loads the selected or inline theme from a specific path.
pub fn load_from(path: &Path, themes: &[Theme]) -> LoadedTheme {
    if let Err(error) = ensure_theme_file(path) {
        return fallback(Some(format!("failed to create theme config: {error}")));
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => return fallback(Some(format!("failed to read theme config: {error}"))),
    };

    match toml::from_str::<ThemeSelection>(&content) {
        Ok(selection) => {
            if let Some(name) = selection.theme {
                return load_builtin(themes, &name);
            }
        }
        Err(error) => return fallback(Some(format!("failed to parse theme config: {error}"))),
    }

    load_inline(&content)
}

/// Saves the selected built-in theme name to the default theme config path.
pub fn save_selected(name: &str) -> Result<(), std::io::Error> {
    save_selected_to(&theme_path(), name)
}

/// Saves the selected built-in theme name to a specific path.
pub fn save_selected_to(path: &Path, name: &str) -> Result<(), std::io::Error> {
    write_theme_file(path, name)
}

fn load_builtin(themes: &[Theme], name: &str) -> LoadedTheme {
    let Some(theme) = builtin::find_by_name(themes, name) else {
        return fallback(Some(format!("unknown theme: {name}")));
    };

    LoadedTheme { theme, error: None }
}

fn load_inline(content: &str) -> LoadedTheme {
    let raw = match toml::from_str::<RawTheme>(content) {
        Ok(raw) => raw,
        Err(error) => return fallback(Some(format!("failed to parse inline theme: {error}"))),
    };

    let theme = match Theme::try_from(raw) {
        Ok(theme) => theme,
        Err(error) => return fallback(Some(error.to_string())),
    };

    LoadedTheme { theme, error: None }
}

fn fallback(error: Option<String>) -> LoadedTheme {
    LoadedTheme {
        theme: gruvbox(),
        error,
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
# theme = "dracula"

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
            bg0 = "#000000"
            bg1 = "#111111"
            bg3 = "#333333"
            bg_sel = "#444444"
            fg0 = "#ffffff"
            fg3 = "#c0c0c0"
            fg4 = "#808080"
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
        assert!(content.contains("# theme = \"dracula\""));
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
    fn saves_builtin_selection_with_examples() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.toml");

        save_selected_to(&path, "solarized").unwrap();

        let content = fs::read_to_string(path).unwrap();
        assert!(content.starts_with("theme = \"solarized\""));
        assert!(content.contains("# Built-in themes"));
        assert!(content.contains("# theme = \"gruvbox\""));
        assert!(content.contains("# theme = \"dracula\""));
        assert!(content.contains("# Full custom theme example"));
        assert!(!content.starts_with("theme = \"gruvbox\""));
    }

    #[test]
    fn theme_path_ends_with_config_lazysql_theme_toml() {
        let path = theme_path();

        assert!(path.ends_with(Path::new(".config").join("lazysql").join("theme.toml")));
    }
}
