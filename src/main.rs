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
    app::AppState,
    navigation::{Router, Screen},
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
                    Some(Screen::Database) => {
                        state.schema_selected = 0;
                        state.table_selected = 0;
                    }
                    _ => {}
                }
                continue;
            }
            // Not a gg sequence — fall through to normal handling below.
        }

        // SQL result popup: Enter to dismiss
        if state.sql_input.has_result() {
            if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                state.sql_input.dismiss_result();
            }
            continue;
        }

        // SQL input mode
        if state.sql_input.active {
            match key.code {
                KeyCode::Esc => state.sql_input.reset(),
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    state.sql_input.query.push('\n');
                }
                KeyCode::Enter => {
                    state.sql_input.close();
                    let query = state.sql_input.query.trim().to_string();
                    if crate::db::postgres::tables::is_returning_query(&query) {
                        let size = terminal.size()?;
                        match state.execute_sql_for_records(size.height, size.width).await {
                            Ok(_) => {
                                state.sql_input.reset();
                                router.push(Screen::Records);
                            }
                            Err(e) => {
                                state.sql_input.result =
                                    Some(state::sql_input::SqlResult::Error(e.to_string()));
                            }
                        }
                    } else {
                        state.execute_sql_input().await;
                    }
                }
                KeyCode::Backspace => {
                    state.sql_input.query.pop();
                }
                KeyCode::Char(c) => state.sql_input.query.push(c),
                _ => {}
            }
            continue;
        }

        // Search input mode: capture all keys for the query.
        if state.search.active {
            match key.code {
                KeyCode::Esc => {
                    state.search.reset();
                    state.schema_selected = 0;
                    state.table_selected = 0;
                }
                KeyCode::Enter => state.search.close(),
                KeyCode::Backspace => {
                    state.search.query.pop();
                    state.clamp_search_selections();
                }
                KeyCode::Down | KeyCode::Char('j') => match router.current() {
                    Some(Screen::Database) => {
                        let len = state.filtered_schema_names().len();
                        if len > 0 {
                            state.schema_selected = (state.schema_selected + 1) % len;
                        }
                    }
                    _ => {}
                },
                KeyCode::Up | KeyCode::Char('k') => match router.current() {
                    Some(Screen::Database) => {
                        state.schema_selected = state.schema_selected.saturating_sub(1);
                    }
                    _ => {}
                },
                KeyCode::Char(c) => {
                    state.search.query.push(c);
                    state.clamp_search_selections();
                }
                _ => {}
            }
            continue;
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
                        router.push(Screen::Database);
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
                KeyCode::Esc => {
                    state.form.reset();
                    router.pop();
                }
                KeyCode::Tab | KeyCode::Down => state.form.next_field(),
                KeyCode::BackTab | KeyCode::Up => state.form.prev_field(),
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

            Some(Screen::Database) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                    state.search.reset();
                    router.pop();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let len = state.filtered_schema_names().len();
                    if len > 0 {
                        state.schema_selected = (state.schema_selected + 1) % len;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.schema_selected = state.schema_selected.saturating_sub(1);
                }
                KeyCode::Char('G') => {
                    let len = state.filtered_schema_names().len();
                    if len > 0 {
                        state.schema_selected = len - 1;
                    }
                }
                KeyCode::Char('g') => pending_g = true,
                KeyCode::Char('/') => state.search.open(),
                KeyCode::Char(':') | KeyCode::Char('с') => {
                    if state.current_db.is_some() {
                        state.sql_input.open();
                    }
                }
                _ => {}
            },

            Some(Screen::Inspect) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                    state.search.reset();
                    router.pop();
                }
                KeyCode::Char('/') => state.search.open(),
                KeyCode::Char(':') | KeyCode::Char('с') => {
                    if state.current_db.is_some() {
                        state.sql_input.open();
                    }
                }
                _ => {}
            },

            Some(Screen::Records) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    state.records.reset();
                    router.pop();
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    if state.records.has_prev_page() {
                        state.records.prev_page();
                        let _ = state.fetch_records_page().await;
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    if state.records.has_next_page() {
                        state.records.next_page();
                        let _ = state.fetch_records_page().await;
                    }
                }
                _ => {}
            },

            None => break,
        }
    }

    Ok(())
}
