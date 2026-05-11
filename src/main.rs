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

    loop {
        terminal.draw(|frame| ui::render(frame, state, router))?;

        let Some(Ok(event)) = events.next().await else {
            break;
        };

        if let Event::Key(key) = event {
            if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                break;
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
                    KeyCode::Enter => {
                        if !state.connections.is_empty()
                            && state.connect_selected().await.is_ok()
                            && state.load_schemas().await.is_ok()
                        {
                            router.push(Screen::Schemas);
                        }
                    }
                    _ => {}
                },
                Some(Screen::Schemas) => {
                    let len = state.schema_names().len();
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
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
                        KeyCode::Enter => {
                            state.table_selected = 0;
                            router.push(Screen::Tables);
                        }
                        _ => {}
                    }
                }
                Some(Screen::Tables) => {
                    let schema = state.selected_schema_name().unwrap_or_default();
                    let len = state.table_names_in_schema(&schema).len();
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
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
                        KeyCode::Enter => {
                            if state.load_table().await.is_ok() {
                                router.push(Screen::TableView);
                            }
                        }
                        _ => {}
                    }
                }
                Some(Screen::TableView) => {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                        router.pop();
                    }
                }
                None => break,
            }
        }
    }

    Ok(())
}
