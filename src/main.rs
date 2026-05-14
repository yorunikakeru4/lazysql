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
    connection::ActivePane,
    mode::AppMode,
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
    let mut pending_g = false;

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

        // Help overlay intercepts all input
        if state.help_visible {
            if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                state.help_visible = false;
            }
            continue;
        }

        // SQL result popup
        if state.sql_input.has_result() {
            if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                state.sql_input.dismiss_result();
                state.mode = AppMode::Normal;
            }
            continue;
        }

        // SQL editor (active)
        if state.sql_input.active {
            handle_sql_editor(key, state, terminal, router).await?;
            continue;
        }

        // Search filter
        if state.search.active {
            handle_search(key, state, router);
            continue;
        }

        // gg motion
        if pending_g {
            pending_g = false;
            if key.code == KeyCode::Char('g') {
                match router.current() {
                    Some(Screen::Connect) => state.connect.selected = 0,
                    Some(Screen::Database) => {
                        if state.active_pane == ActivePane::Schemas {
                            state.schema_selected = 0;
                        } else {
                            state.table_selected = 0;
                        }
                    }
                    Some(Screen::Records) => state.records.selected_row = 0,
                    _ => {}
                }
                continue;
            }
        }

        // Global ? opens help
        if key.code == KeyCode::Char('?') {
            state.help_visible = true;
            continue;
        }

        match router.current() {
            Some(Screen::Connect) => {
                if key.code == KeyCode::Char('q') {
                    break;
                }
                handle_connect(key, state, router, &mut pending_g).await;
            }
            Some(Screen::AddConnection) => handle_add_connection(key, state, router),
            Some(Screen::Database) => {
                handle_database(key, state, router, terminal, &mut pending_g).await?;
            }
            Some(Screen::Inspect) => handle_inspect(key, state, router, terminal).await?,
            Some(Screen::Records) => handle_records(key, state, router, &mut pending_g).await,
            None => break,
        }
    }
    Ok(())
}

// ─── SQL Editor ──────────────────────────────────────────────────────────────

async fn handle_sql_editor(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    router: &mut Router,
) -> std::io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            state.sql_input.reset();
            state.mode = AppMode::Normal;
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            execute_sql_editor_query(state, terminal, router).await?;
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            execute_sql_editor_query(state, terminal, router).await?;
        }
        KeyCode::Enter => state.sql_input.insert_newline(),
        KeyCode::Tab => state.sql_input.insert_tab(),
        KeyCode::Backspace => state.sql_input.delete_before(),
        KeyCode::Delete => state.sql_input.delete_at(),
        KeyCode::Left => state.sql_input.move_left(),
        KeyCode::Right => state.sql_input.move_right(),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sql_input.history_prev();
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sql_input.history_next();
        }
        KeyCode::Up => state.sql_input.move_up(),
        KeyCode::Down => state.sql_input.move_down(),
        KeyCode::Home => state.sql_input.move_line_start(),
        KeyCode::End => state.sql_input.move_line_end(),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sql_input.history_prev();
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sql_input.history_next();
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sql_input.move_line_start();
        }
        KeyCode::Char(c) => state.sql_input.insert_char(c),
        _ => {}
    }
    Ok(())
}

async fn execute_sql_editor_query(
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    router: &mut Router,
) -> std::io::Result<()> {
    state.sql_input.close();
    state.sql_input.push_history();
    let query = state.sql_input.query.trim().to_string();
    if query.is_empty() {
        state.sql_input.reset();
        state.mode = AppMode::Normal;
        return Ok(());
    }

    if db::postgres::tables::is_returning_query(&query) {
        let size = terminal.size()?;
        match state.execute_sql_for_records(size.height, size.width).await {
            Ok(_) => {
                state.sql_input.reset();
                state.mode = AppMode::Result;
                router.push(Screen::Records);
            }
            Err(e) => {
                state.sql_input.result = Some(state::sql_input::SqlResult::Error(e.to_string()));
                state.mode = AppMode::Result;
            }
        }
    } else {
        state.execute_sql_input().await;
        state.mode = AppMode::Result;
    }

    Ok(())
}

// ─── Search ──────────────────────────────────────────────────────────────────

fn handle_search(key: crossterm::event::KeyEvent, state: &mut AppState, router: &Router) {
    match key.code {
        KeyCode::Esc => {
            state.search.reset();
            state.schema_selected = 0;
            state.table_selected = 0;
            state.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            state.search.close();
            state.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            state.search.query.pop();
            state.clamp_search_selections();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(Screen::Database) = router.current() {
                if state.active_pane == ActivePane::Schemas {
                    let len = state.filtered_schema_names().len();
                    if len > 0 {
                        state.schema_selected = (state.schema_selected + 1) % len;
                    }
                } else {
                    let schema = state.selected_schema_name().unwrap_or_default();
                    let len = state.filtered_table_names(&schema).len();
                    if len > 0 {
                        state.table_selected = (state.table_selected + 1) % len;
                    }
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(Screen::Database) = router.current() {
                if state.active_pane == ActivePane::Schemas {
                    state.schema_selected = state.schema_selected.saturating_sub(1);
                } else {
                    state.table_selected = state.table_selected.saturating_sub(1);
                }
            }
        }
        KeyCode::Char(c) => {
            state.search.query.push(c);
            state.clamp_search_selections();
        }
        _ => {}
    }
}

// ─── Connect ─────────────────────────────────────────────────────────────────

async fn handle_connect(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    pending_g: &mut bool,
) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            state.connect.select_next(state.connections.len());
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.connect.select_prev(state.connections.len());
        }
        KeyCode::Char('G') => {
            let len = state.connections.len();
            if len > 0 {
                state.connect.selected = len - 1;
            }
        }
        KeyCode::Char('g') => *pending_g = true,
        KeyCode::Char('a') => {
            *pending_g = false;
            state.form.reset();
            state.mode = AppMode::Insert;
            router.push(Screen::AddConnection);
        }
        KeyCode::Char('e') => {
            *pending_g = false;
            if let Some(config::Connect::Postgres(cfg)) =
                state.connections.get(state.connect.selected)
            {
                state.form = state::connection::FormState::from_postgres_config(
                    cfg,
                    Some(state.connect.selected),
                );
                state.mode = AppMode::Insert;
                router.push(Screen::AddConnection);
            }
        }
        KeyCode::Char('d') => {
            *pending_g = false;
            if !state.connections.is_empty() {
                state.connections.remove(state.connect.selected);
                if state.connect.selected >= state.connections.len() {
                    state.connect.selected = state.connections.len().saturating_sub(1);
                }
                let _ = ConfigStorage::save(&state.connections);
            }
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if !state.connections.is_empty()
                && state.connect_selected().await.is_ok()
                && state.load_schemas().await.is_ok()
            {
                state.active_pane = ActivePane::Schemas;
                router.push(Screen::Database);
            }
        }
        KeyCode::Char('h') => {} // no-op at root
        _ => {}
    }
}

// ─── Add Connection ───────────────────────────────────────────────────────────

fn handle_add_connection(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    router: &mut Router,
) {
    match key.code {
        KeyCode::Esc => {
            state.form.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        KeyCode::Tab | KeyCode::Down => state.form.next_field(),
        KeyCode::BackTab | KeyCode::Up => state.form.prev_field(),
        KeyCode::Backspace => {
            state.form.current_value_mut().pop();
        }
        KeyCode::Enter => save_connection_form(state, router),
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            save_connection_form(state, router);
        }
        KeyCode::Char(c) => {
            state.form.current_value_mut().push(c);
        }
        _ => {}
    }
}

fn save_connection_form(state: &mut AppState, router: &mut Router) {
    match state.form.to_postgres_config() {
        Ok(cfg) => {
            state.form.error = None;
            if let Some(editing_index) = state.form.editing_index {
                if editing_index < state.connections.len() {
                    state.connections[editing_index] = config::Connect::Postgres(cfg);
                    state.connect.selected = editing_index;
                }
            } else {
                state.connections.push(config::Connect::Postgres(cfg));
                state.connect.selected = state.connections.len().saturating_sub(1);
            }
            let _ = ConfigStorage::save(&state.connections);
            state.form.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        Err(msg) => state.form.error = Some(msg),
    }
}

// ─── Database ─────────────────────────────────────────────────────────────────

async fn handle_database(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    pending_g: &mut bool,
) -> std::io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.search.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        KeyCode::Tab => {
            state.active_pane = if state.active_pane == ActivePane::Schemas {
                ActivePane::Tables
            } else {
                ActivePane::Schemas
            };
        }
        KeyCode::Char('h') => {
            if state.active_pane == ActivePane::Tables {
                state.active_pane = ActivePane::Schemas;
            } else {
                state.search.reset();
                router.pop();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.active_pane == ActivePane::Schemas {
                let len = state.filtered_schema_names().len();
                if len > 0 {
                    state.schema_selected = (state.schema_selected + 1) % len;
                }
            } else {
                let schema = state.selected_schema_name().unwrap_or_default();
                let len = state.filtered_table_names(&schema).len();
                if len > 0 {
                    state.table_selected = (state.table_selected + 1) % len;
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.active_pane == ActivePane::Schemas {
                state.schema_selected = state.schema_selected.saturating_sub(1);
            } else {
                state.table_selected = state.table_selected.saturating_sub(1);
            }
        }
        KeyCode::Char('G') => {
            if state.active_pane == ActivePane::Schemas {
                let len = state.filtered_schema_names().len();
                if len > 0 {
                    state.schema_selected = len - 1;
                }
            } else {
                let schema = state.selected_schema_name().unwrap_or_default();
                let len = state.filtered_table_names(&schema).len();
                if len > 0 {
                    state.table_selected = len - 1;
                }
            }
        }
        KeyCode::Char('g') => *pending_g = true,
        KeyCode::Char('l') | KeyCode::Enter => {
            if state.active_pane == ActivePane::Schemas {
                // Move to Tables pane, reset table selection
                let chosen = state
                    .filtered_schema_names()
                    .into_iter()
                    .nth(state.schema_selected);
                state.search.reset();
                state.table_selected = 0;
                if let Some(name) = chosen {
                    state.schema_selected = state
                        .schema_names()
                        .iter()
                        .position(|s| *s == name)
                        .unwrap_or(0);
                }
                state.active_pane = ActivePane::Tables;
            } else {
                // Open inspect
                let schema = state.selected_schema_name().unwrap_or_default();
                let table_name = state
                    .filtered_table_names(&schema)
                    .into_iter()
                    .nth(state.table_selected);
                if let Some(name) = table_name {
                    state.search.reset();
                    if state.load_table_details(&schema, &name).await.is_ok() {
                        router.push(Screen::Inspect);
                    }
                }
            }
        }
        KeyCode::Char('r') => {
            let schema = state.selected_schema_name().unwrap_or_default();
            let table_name = state
                .filtered_table_names(&schema)
                .into_iter()
                .nth(state.table_selected);
            if let Some(name) = table_name {
                state.search.reset();
                if state.load_table_by_name(name).await.is_ok() {
                    let size = terminal.size()?;
                    if state
                        .load_table_records(size.height, size.width)
                        .await
                        .is_ok()
                    {
                        state.mode = AppMode::Result;
                        router.push(Screen::Records);
                    }
                }
            }
        }
        KeyCode::Char('/') => {
            state.search.open();
            state.mode = AppMode::Search;
        }
        KeyCode::Char(':') | KeyCode::Char('с') => {
            if state.current_db.is_some() {
                state.sql_input.open();
                state.mode = AppMode::Command;
            }
        }
        _ => {}
    }
    Ok(())
}

// ─── Inspect ──────────────────────────────────────────────────────────────────

async fn handle_inspect(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> std::io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
            state.search.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        KeyCode::Char('/') => {
            state.search.open();
            state.mode = AppMode::Search;
        }
        KeyCode::Char('r') | KeyCode::Char('s') => {
            if let Some(details) = state.table_details.as_ref() {
                let schema = details.schema.clone();
                let name = details.name.clone();
                if state.load_table_by_name(name).await.is_ok() {
                    let size = terminal.size()?;
                    if state
                        .load_table_records(size.height, size.width)
                        .await
                        .is_ok()
                    {
                        state.mode = AppMode::Result;
                        router.push(Screen::Records);
                    }
                }
                let _ = schema; // used in load_table_by_name indirectly
            }
        }
        _ => {}
    }
    Ok(())
}

// ─── Records ──────────────────────────────────────────────────────────────────

async fn handle_records(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    pending_g: &mut bool,
) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.records.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.records.selected_row + 1 >= state.records.rows.len()
                && state.records.has_next_page()
            {
                state.records.next_page();
                let _ = state.fetch_records_page().await;
                state.records.selected_row = 0;
            } else {
                state.records.move_row_down();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.records.selected_row == 0 && state.records.has_prev_page() {
                state.records.prev_page();
                let _ = state.fetch_records_page().await;
                state.records.selected_row = state.records.rows.len().saturating_sub(1);
            } else {
                state.records.move_row_up();
            }
        }
        KeyCode::Char('l') | KeyCode::Right => state.records.move_col_right(),
        KeyCode::Char('h') | KeyCode::Left => state.records.move_col_left(),
        KeyCode::Char('G') => {
            state.records.selected_row = state.records.rows.len().saturating_sub(1);
        }
        KeyCode::Char('y') => {
            if let Some(cell) = state.records.current_cell_value() {
                let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(cell.to_string()));
            }
        }
        KeyCode::Char('Y') => {
            let tsv = state.records.current_row_tsv();
            let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(tsv));
        }
        KeyCode::Char('g') => *pending_g = true,
        _ => {}
    }
}
