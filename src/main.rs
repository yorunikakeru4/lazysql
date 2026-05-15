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
    AppState::new(connections)
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
        let state = initialize_state(vec![config::ConnectConfig::Postgres(
            config::PostgresConfig {
                name: Some("local".to_string()),
                host: "127.0.0.1".to_string(),
                user: "postgres".to_string(),
                db_name: "postgres".to_string(),
                port: 1,
                password: None,
            },
        )]);

        assert_eq!(
            state.connection_status(0),
            state::connection::ConnectionStatus::Unknown
        );
    }
}
