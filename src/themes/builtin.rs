use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::themes::palette::{RawTheme, Theme, ThemeError};

include!(concat!(env!("OUT_DIR"), "/bundled_themes.rs"));

/// Returns the embedded gruvbox theme used as a last-resort fallback.
pub fn fallback_theme() -> Theme {
    let (_, content) = BUNDLED
        .iter()
        .find(|(name, _)| *name == "gruvbox.toml")
        .expect("gruvbox.toml missing from bundled themes");
    let raw: RawTheme = toml::from_str(content)
        .unwrap_or_else(|e| panic!("embedded gruvbox.toml is invalid TOML: {e}"));
    Theme::try_from(raw).unwrap_or_else(|e| panic!("embedded gruvbox.toml has invalid colors: {e}"))
}

/// Loads built-in themes from the given directory path.
pub fn load(path: &Path) -> Result<Vec<Theme>, ThemeError> {
    let mut themes = BTreeMap::new();

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![]);
        }
        Err(error) => {
            return Err(ThemeError(format!(
                "failed to read themes directory: {error}"
            )));
        }
    };

    for entry in entries {
        let entry =
            entry.map_err(|error| ThemeError(format!("failed to read theme entry: {error}")))?;
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("toml") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .map_err(|error| ThemeError(format!("failed to read theme file: {error}")))?;
        let raw = toml::from_str::<RawTheme>(&content).map_err(|error| {
            ThemeError(format!("failed to parse theme {}: {error}", path.display()))
        })?;
        let theme = Theme::try_from(raw).map_err(|error| {
            ThemeError(format!("failed to load theme {}: {error}", path.display()))
        })?;
        themes.insert(theme.name.clone(), theme);
    }

    Ok(themes.into_values().collect())
}

/// Finds a theme by exact case-sensitive name.
pub fn find_by_name(themes: &[Theme], name: &str) -> Option<Theme> {
    themes.iter().find(|theme| theme.name == name).cloned()
}

#[cfg(test)]
mod test {
    use super::*;

    fn write_theme(path: &std::path::Path, name: &str) {
        std::fs::write(
            path,
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
            ),
        )
        .unwrap();
    }

    #[test]
    fn loads_themes_from_directory_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        write_theme(&dir.path().join("zeta.toml"), "zeta");
        write_theme(&dir.path().join("alpha.toml"), "alpha");

        let themes = load(dir.path()).unwrap();
        let names = themes
            .iter()
            .map(|theme| theme.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["alpha", "zeta"]);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("broken.toml"), "name = [").unwrap();

        let result = load(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn fallback_theme_returns_gruvbox() {
        assert_eq!(fallback_theme().name, "gruvbox");
    }
}
