# Theme System Design

## Summary

Move lazysql theme colors out of hardcoded Rust constants and into TOML theme files.
Ship built-in `gruvbox` and `dracula` themes, support user-defined themes in `~/.config/lazysql/theme.toml`, and add a keyboard-only theme picker on the Connections screen.

The implementation will use a full runtime theme model: render code reads colors from `AppState.theme.colors`, not from static `ui::theme` constants. This keeps the app ready for immediate theme switching without restart.

## Goals

- Add root-level `themes/` directory with built-in `.toml` theme files.
- Add `~/.config/lazysql/theme.toml` next to the existing `config.toml`.
- Support selecting a built-in theme by name.
- Support defining a complete custom theme inline in `theme.toml`.
- Create a documented starter `theme.toml` on first launch.
- Add `Ctrl+T` theme picker on the main Connections screen.
- Apply selected theme immediately and persist the selection.

## Non-Goals

- No per-connection themes.
- No live file watching.
- No partial theme inheritance in the first version.
- No theme editor screen.
- No mouse support for picker.

## Theme Files

Built-in themes live in the repository root:

```text
themes/
  gruvbox.toml
  dracula.toml
```

Each theme file uses the same format as inline custom themes:

```toml
name = "gruvbox"

[colors]
bg0 = "#1d2021"
bg1 = "#282828"
bg3 = "#3c3836"
bg_sel = "#504945"
fg0 = "#ebdbb2"
fg3 = "#a89984"
fg4 = "#7c6f64"
red = "#fb4934"
green = "#b8bb26"
yellow = "#fabd2f"
blue = "#83a598"
aqua = "#8ec07c"
orange = "#fe8019"
purple = "#d3869b"
```

`dracula.toml` uses the same keys with Dracula palette values.

## User Theme Config

The user theme config lives at:

```text
~/.config/lazysql/theme.toml
```

Two forms are valid.

Built-in theme selection:

```toml
theme = "gruvbox"
```

Inline custom theme:

```toml
name = "my-theme"

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
```

On first launch, if `theme.toml` does not exist, create it with active default `theme = "gruvbox"` plus commented documentation:

```toml
theme = "gruvbox"

# Built-in themes:
# theme = "gruvbox"
# theme = "dracula"
#
# Custom theme example:
# name = "my-theme"
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
```

## Rust Modules

Add:

```text
src/themes.rs
src/themes/
  builtin.rs
  palette.rs
  picker.rs
  storage.rs
```

Responsibilities:

- `themes::palette`: public runtime theme structs and color parsing.
- `themes::builtin`: discovery and loading of built-in themes from `themes/*.toml`.
- `themes::storage`: `theme.toml` path, first-run file creation, load, save selected theme.
- `themes::picker`: filterable keyboard picker state.

Core types:

```rust
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

pub struct ThemeColors {
    pub bg0: Color,
    pub bg1: Color,
    pub bg3: Color,
    pub bg_sel: Color,
    pub fg0: Color,
    pub fg3: Color,
    pub fg4: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub aqua: Color,
    pub orange: Color,
    pub purple: Color,
}
```

All public structs, enums, traits, and functions get concise doc comments.

## App State

Extend `AppState`:

```rust
pub theme: Theme,
pub theme_picker: ThemePickerState,
```

`initialize_state` loads the active theme during startup. If loading fails, it uses built-in `gruvbox` and records a user-visible theme error for status display or picker feedback.

The current `src/ui/theme.rs` static constants are removed or reduced to non-runtime helpers only. Screen and widget render functions receive or access `state.theme.colors` and use direct fields:

```rust
Style::new().fg(state.theme.colors.blue)
Style::new().bg(state.theme.colors.bg_sel)
```

This is a deliberate full migration. No long-term mixed model with both `theme::BLUE` constants and `state.theme.colors.blue`.

## Theme Loading Flow

Startup:

1. Ensure `~/.config/lazysql/theme.toml` exists.
2. Load built-in themes from `themes/*.toml`.
3. Parse `theme.toml`.
4. If it contains `theme = "<name>"`, find matching built-in theme.
5. If it contains `name` and `[colors]`, parse it as inline custom theme.
6. If parsing fails, use `gruvbox`.

Fallback rules:

- Unknown built-in theme: fallback to `gruvbox`.
- Invalid hex color: fallback to `gruvbox`.
- Missing required color key: fallback to `gruvbox`.
- Missing built-in files: use compiled gruvbox fallback inside Rust so app still starts.

Persistence:

- Picker selection writes `theme = "<name>"` to `theme.toml`.
- Inline custom themes are preserved until user selects a built-in theme; selecting built-in replaces file content with documented built-in selection.

## Picker UX

Open key:

- `Ctrl+T`

Available only when:

- current screen is `Screen::Connect`;
- app mode is `AppMode::Normal`;
- add/edit connection form is closed;
- connection error popup is closed;
- help/search/sql/result overlays are inactive.

Keyboard behavior:

- `Ctrl+T`: open theme picker overlay.
- printable chars: update filter query.
- `Backspace`: delete query char.
- `Down` or `j`: next visible theme.
- `Up` or `k`: previous visible theme.
- `Enter`: select highlighted theme, save config, apply immediately, close picker.
- `Esc`: cancel and close picker.

Initial list:

- `gruvbox`
- `dracula`

If more valid files exist in `themes/`, picker lists them sorted by name.

Overlay copy:

```text
Select theme
› gruvbox

▶ gruvbox
  dracula

2 themes · type to filter
↵:select  esc:cancel
```

Statusbar while picker is open:

```text
PICK theme · type to filter · ↵ select · esc cancel
```

Connections screen hint adds:

```text
^t:theme
```

## Error Handling

Theme load errors must not prevent app startup.

The app stores the latest theme error as display-only state. The UI may show it in the picker footer or status area. Errors should be concise:

- `unknown theme: solarized`
- `invalid color for fg0`
- `missing color: bg_sel`

No connection strings or unrelated config data are logged or displayed.

## Testing

Unit tests:

- parse valid `#rrggbb` hex values;
- reject malformed hex values;
- load `theme = "gruvbox"`;
- load a complete inline custom theme;
- fallback to `gruvbox` for unknown built-in theme;
- fallback to `gruvbox` for invalid inline color;
- create default `theme.toml` with commented examples;
- save selected built-in theme;
- picker filters by query;
- picker selection clamps after filtering;
- picker cancel does not alter active theme;
- `Ctrl+T` opens picker only on Connections screen in Normal mode;
- `Ctrl+T` does not open picker while add/edit form is open.

Render tests should be updated where hardcoded theme constants were asserted. Assertions should compare against `state.theme.colors.*`.

Verification after implementation:

```bash
cargo fmt
cargo clippy -- -D warnings
just test
```

`just test` remains the supported integration-test path because DB tests require the compose-managed PostgreSQL environment.

## Open Decisions

- Theme names are case-sensitive in config and picker.
- Built-in theme file names must match their `name` field.
- `gruvbox` is the default theme.
- Inline custom themes do not appear in picker unless later saved as a file under `themes/`.
