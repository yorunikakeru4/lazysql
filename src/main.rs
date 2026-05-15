mod config;
mod db;
mod handlers;
mod state;
mod themes;
mod ui;

use config::storage::ConfigStorage;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, prelude::CrosstermBackend};
use state::{app::AppState, navigation::Router};
use std::path::Path;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let connections = ConfigStorage::load();
    let mut state = initialize_state(connections);
    state.refresh_connection_statuses().await;

    let mut router = Router::new();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut state, &mut router).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}

/// Builds startup state without blocking on connection reachability probes.
fn initialize_state(connections: Vec<config::ConnectConfig>) -> AppState {
    let theme_path = themes::storage::theme_path();
    initialize_state_with_theme_loader(connections, themes::builtin::load, &theme_path)
}

fn initialize_state_with_theme_loader(
    connections: Vec<config::ConnectConfig>,
    load_themes: impl FnOnce() -> Result<Vec<themes::palette::Theme>, themes::palette::ThemeError>,
    theme_path: &Path,
) -> AppState {
    let (available_themes, builtin_error) = match load_themes() {
        Ok(themes) => (themes, None),
        Err(error) => (vec![themes::palette::gruvbox()], Some(error.to_string())),
    };

    let loaded_theme = themes::storage::load_from(theme_path, &available_themes);
    let theme_error = combine_theme_errors(builtin_error, loaded_theme.error);

    AppState::new_with_theme(
        connections,
        loaded_theme.theme,
        available_themes,
        theme_error,
    )
}

fn combine_theme_errors(
    builtin_error: Option<String>,
    storage_error: Option<String>,
) -> Option<String> {
    match (builtin_error, storage_error) {
        (Some(builtin), Some(storage)) => Some(format!("{builtin}; {storage}")),
        (Some(builtin), None) => Some(builtin),
        (None, storage) => storage,
    }
}

#[cfg(test)]
fn initialize_state_from_theme_path(
    connections: Vec<config::ConnectConfig>,
    theme_path: &Path,
) -> AppState {
    initialize_state_with_theme_loader(connections, themes::builtin::load, theme_path)
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    router: &mut Router,
) -> std::io::Result<()> {
    let mut events = EventStream::new();

    loop {
        terminal.draw(|frame| ui::render(frame, state, router))?;

        let event = match events.next().await {
            Some(Ok(e)) => e,
            Some(Err(e)) => return Err(e),
            None => break,
        };
        let Event::Key(key) = event else { continue };

        // Ctrl-C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            break;
        }

        if !handlers::handle(key, state, router, terminal).await? {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn initialize_state_does_not_refresh_connection_statuses_on_startup() {
        let dir = tempfile::tempdir().unwrap();
        let state = initialize_state_from_theme_path(
            vec![config::ConnectConfig::Postgres(config::PostgresConfig {
                name: Some("local".to_string()),
                host: "127.0.0.1".to_string(),
                user: "postgres".to_string(),
                db_name: "postgres".to_string(),
                port: 1,
                password: None,
            })],
            &dir.path().join("theme.toml"),
        );

        assert_eq!(
            state.connection_status(0),
            state::connection::ConnectionStatus::Unknown
        );
    }

    #[test]
    fn initialize_state_has_default_theme() {
        let dir = tempfile::tempdir().unwrap();
        let state = initialize_state_from_theme_path(Vec::new(), &dir.path().join("theme.toml"));

        assert_eq!(state.theme.name, "gruvbox");
    }

    #[test]
    fn initialize_state_preserves_builtin_theme_load_error() {
        let dir = tempfile::tempdir().unwrap();
        let state = initialize_state_with_theme_loader(
            Vec::new(),
            || {
                Err(themes::palette::ThemeError(
                    "builtin themes unavailable".to_string(),
                ))
            },
            &dir.path().join("theme.toml"),
        );

        assert_eq!(state.theme.name, "gruvbox");
        assert_eq!(
            state.theme_error,
            Some("builtin themes unavailable".to_string())
        );
    }
}
