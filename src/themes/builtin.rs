use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::themes::palette::{RawTheme, Theme, ThemeError, dracula, gruvbox};

/// Loads built-in themes from the repository themes directory.
pub fn load() -> Result<Vec<Theme>, ThemeError> {
    load_from_dir(Path::new("themes"))
}

/// Loads all TOML themes from a directory.
pub fn load_from_dir(path: &Path) -> Result<Vec<Theme>, ThemeError> {
    let mut themes = default_themes_by_name();
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(themes.into_values().collect());
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

fn default_themes_by_name() -> BTreeMap<String, Theme> {
    [dracula(), gruvbox()]
        .into_iter()
        .map(|theme| (theme.name.clone(), theme))
        .collect()
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

        let themes = load_from_dir(dir.path()).unwrap();
        let names = themes
            .iter()
            .map(|theme| theme.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["alpha", "dracula", "gruvbox", "zeta"]);
    }

    #[test]
    fn default_list_contains_gruvbox_when_directory_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing_path = dir.path().join("missing");

        let themes = load_from_dir(&missing_path).unwrap();

        assert!(find_by_name(&themes, "gruvbox").is_some());
    }

    #[test]
    fn default_list_contains_gruvbox_and_dracula_when_directory_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing_path = dir.path().join("missing");

        let themes = load_from_dir(&missing_path).unwrap();
        let names = themes
            .iter()
            .map(|theme| theme.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["dracula", "gruvbox"]);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("broken.toml"), "name = [").unwrap();

        let result = load_from_dir(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn invalid_color_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("bad-color.toml"),
            r##"
            name = "bad-color"

            [colors]
            bg0 = "not-a-color"
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
            "##,
        )
        .unwrap();

        let result = load_from_dir(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn find_by_name_matches_exact_case_only() {
        let themes = load_from_dir(std::path::Path::new("missing")).unwrap();

        assert!(find_by_name(&themes, "dracula").is_some());
        assert!(find_by_name(&themes, "gruvbox").is_some());
        assert!(find_by_name(&themes, "Gruvbox").is_none());
    }
}
