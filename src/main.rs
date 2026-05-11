mod config;
mod db;
mod state;
mod ui;

use config::storage::ConfigStorage;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, prelude::CrosstermBackend};
use state::{
    app_state::AppState,
    router::{Router, Screen},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let connections = ConfigStorage::load();
    let mut state = AppState::new(connections);
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

async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    router: &mut Router,
) -> std::io::Result<()> {
    let mut events = EventStream::new();
    // Tracks first 'g' keypress for the gg (go-to-first) motion.
    let mut pending_g = false;

    loop {
        terminal.draw(|frame| ui::render(frame, state, router))?;

        let event = match events.next().await {
            Some(Ok(e)) => e,
            Some(Err(e)) => return Err(e),
            None => break,
        };

        let Event::Key(key) = event else {
            continue;
        };

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            break;
        }

        // Handle second key of gg sequence (go to first item in any list screen).
        if pending_g {
            pending_g = false;
            if key.code == KeyCode::Char('g') {
                match router.current() {
                    Some(Screen::Connect) => state.connect.selected = 0,
                    Some(Screen::Schemas) => state.schema_selected = 0,
                    Some(Screen::Tables) => state.table_selected = 0,
                    _ => {}
                }
                continue;
            }
            // Not a gg sequence — fall through to normal handling below.
        }

        match router.current() {
            Some(Screen::Connect) => match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down | KeyCode::Char('j') => {
                    state.connect.select_next(state.connections.len());
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.connect.select_prev(state.connections.len());
                }
                // l = select (same as Enter)
                KeyCode::Char('l') | KeyCode::Enter => {
                    if !state.connections.is_empty()
                        && state.connect_selected().await.is_ok()
                        && state.load_schemas().await.is_ok()
                    {
                        router.push(Screen::Schemas);
                    }
                }
                // h at root screen is a no-op (nowhere to go back to)
                KeyCode::Char('h') => {}
                KeyCode::Char('G') => {
                    let len = state.connections.len();
                    if len > 0 {
                        state.connect.selected = len - 1;
                    }
                }
                KeyCode::Char('g') => pending_g = true,
                KeyCode::Char('a') => {
                    pending_g = false;
                    state.form.reset();
                    router.push(Screen::AddConnection);
                }
                _ => {}
            },

            Some(Screen::AddConnection) => match key.code {
                KeyCode::Esc | KeyCode::Char('h') => {
                    state.form.reset();
                    router.pop();
                }
                KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => state.form.next_field(),
                KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => state.form.prev_field(),
                KeyCode::Backspace => {
                    state.form.current_value_mut().pop();
                }
                KeyCode::Enter => match state.form.to_postgres_config() {
                    Ok(cfg) => {
                        state.form.error = None;
                        state.connections.push(config::Connect::Postgres(cfg));
                        let _ = ConfigStorage::save(&state.connections);
                        state.form.reset();
                        router.pop();
                    }
                    Err(msg) => state.form.error = Some(msg),
                },
                KeyCode::Char(c) => {
                    state.form.current_value_mut().push(c);
                }
                _ => {}
            },

            Some(Screen::Schemas) => {
                let len = state.schema_names().len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                        router.pop();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if len > 0 {
                            state.schema_selected = (state.schema_selected + 1) % len;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.schema_selected = state.schema_selected.saturating_sub(1);
                    }
                    KeyCode::Char('l') | KeyCode::Enter => {
                        state.table_selected = 0;
                        router.push(Screen::Tables);
                    }
                    KeyCode::Char('G') => {
                        if len > 0 {
                            state.schema_selected = len - 1;
                        }
                    }
                    KeyCode::Char('g') => pending_g = true,
                    _ => {}
                }
            }

            Some(Screen::Tables) => {
                let schema = state.selected_schema_name().unwrap_or_default();
                let len = state.table_names_in_schema(&schema).len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                        router.pop();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if len > 0 {
                            state.table_selected = (state.table_selected + 1) % len;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.table_selected = state.table_selected.saturating_sub(1);
                    }
                    KeyCode::Char('l') | KeyCode::Enter => {
                        if state.load_table().await.is_ok() {
                            router.push(Screen::TableView);
                        }
                    }
                    KeyCode::Char('G') => {
                        if len > 0 {
                            state.table_selected = len - 1;
                        }
                    }
                    KeyCode::Char('g') => pending_g = true,
                    _ => {}
                }
            }

            Some(Screen::TableView) => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h')
                ) {
                    router.pop();
                }
            }

            None => break,
        }
    }

    Ok(())
}
