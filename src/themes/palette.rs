use ratatui::style::Color;
use serde::Deserialize;

/// Runtime TUI theme.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

/// Runtime color palette used by screens and widgets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    // primary colors
    pub primary: Color,
    pub secondary: Color,

    // background colors
    pub bg0: Color,
    pub bg1: Color,
    pub bg2: Color,
    pub bg_sel: Color,

    // foreground colors
    pub fg0: Color,
    pub fg1: Color,
    pub fg2: Color,

    // accent colors
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub aqua: Color,
    pub orange: Color,
    pub purple: Color,
}

/// TOML representation of one theme.
#[derive(Debug, Clone, Deserialize)]
pub struct RawTheme {
    pub name: String,
    pub colors: RawThemeColors,
}

/// TOML representation of theme colors.
#[derive(Debug, Clone, Deserialize)]
pub struct RawThemeColors {
    pub primary: String,
    pub secondary: String,

    pub bg0: String,
    pub bg1: String,
    pub bg2: String,
    pub bg_sel: String,

    pub fg0: String,
    pub fg1: String,
    pub fg2: String,

    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub aqua: String,
    pub orange: String,
    pub purple: String,
}

/// Theme parsing error suitable for user-facing display.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeError(pub String);

impl std::fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ThemeError {}

/// Parses a `#rrggbb` color string.
pub fn parse_hex_color(value: &str) -> Result<Color, ThemeError> {
    let Some(hex) = value.strip_prefix('#') else {
        return Err(invalid_color(value));
    };

    if hex.len() != 6 || !hex.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return Err(invalid_color(value));
    }

    let red = parse_hex_pair(value, &hex[0..2])?;
    let green = parse_hex_pair(value, &hex[2..4])?;
    let blue = parse_hex_pair(value, &hex[4..6])?;

    Ok(Color::Rgb(red, green, blue))
}

impl TryFrom<RawTheme> for Theme {
    type Error = ThemeError;

    fn try_from(raw: RawTheme) -> Result<Self, Self::Error> {
        Ok(Self {
            name: raw.name,
            colors: ThemeColors {
                primary: parse_theme_color("primary", &raw.colors.primary)?,
                secondary: parse_theme_color("secondary", &raw.colors.secondary)?,
                bg0: parse_theme_color("bg0", &raw.colors.bg0)?,
                bg1: parse_theme_color("bg1", &raw.colors.bg1)?,
                bg2: parse_theme_color("bg3", &raw.colors.bg2)?,
                bg_sel: parse_theme_color("bg_sel", &raw.colors.bg_sel)?,

                fg0: parse_theme_color("fg0", &raw.colors.fg0)?,
                fg1: parse_theme_color("fg3", &raw.colors.fg1)?,
                fg2: parse_theme_color("fg4", &raw.colors.fg2)?,

                red: parse_theme_color("red", &raw.colors.red)?,
                green: parse_theme_color("green", &raw.colors.green)?,
                yellow: parse_theme_color("yellow", &raw.colors.yellow)?,
                blue: parse_theme_color("blue", &raw.colors.blue)?,
                aqua: parse_theme_color("aqua", &raw.colors.aqua)?,
                orange: parse_theme_color("orange", &raw.colors.orange)?,
                purple: parse_theme_color("purple", &raw.colors.purple)?,
            },
        })
    }
}

fn invalid_color(value: &str) -> ThemeError {
    ThemeError(format!("invalid color: {value}"))
}

fn parse_hex_pair(value: &str, pair: &str) -> Result<u8, ThemeError> {
    u8::from_str_radix(pair, 16).map_err(|_| invalid_color(value))
}

fn parse_theme_color(field: &str, value: &str) -> Result<Color, ThemeError> {
    parse_hex_color(value).map_err(|_| ThemeError(format!("invalid color for {field}")))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_valid_hex_color() {
        assert_eq!(
            parse_hex_color("#1d2021").unwrap(),
            Color::Rgb(0x1d, 0x20, 0x21)
        );
    }

    #[test]
    fn rejects_malformed_hex_color() {
        assert!(parse_hex_color("1d2021").is_err());
        assert!(parse_hex_color("#12345").is_err());
        assert!(parse_hex_color("#xx2021").is_err());
    }

    #[test]
    fn converts_complete_toml_theme() {
        let raw = toml::from_str::<RawTheme>(
            r##"
            name = "test"

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
            "##,
        )
        .unwrap();

        let theme = Theme::try_from(raw).unwrap();
        assert_eq!(theme.name, "test");
        assert_eq!(theme.colors.bg_sel, Color::Rgb(0x44, 0x44, 0x44));
    }
}
