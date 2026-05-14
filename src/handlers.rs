use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::io::Stdout;

use crate::config;
use crate::config::storage::ConfigStorage;
use crate::db;
use crate::state::app::{AppState, format_sql_error};
use crate::state::connection::{ActivePane, FormState};
use crate::state::mode::AppMode;
use crate::state::navigation::{Router, Screen};
use crate::state::sql_input::SqlResult;

/// Dispatches a key event to the appropriate screen handler.
/// Returns `false` when the application should quit.
pub async fn handle(
    key: KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> std::io::Result<bool> {
    if state.help_visible {
        if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
            state.help_visible = false;
        }
        return Ok(true);
    }

    if state.sql_input.has_result() {
        if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
            state.sql_input.dismiss_result();
            state.mode = AppMode::Normal;
        }
        return Ok(true);
    }

    if handle_connect_error_popup(key, state, router) {
        return Ok(true);
    }

    if state.sql_input.active {
        handle_sql_editor(key, state, terminal, router).await?;
        return Ok(true);
    }

    if state.search.active {
        handle_search(key, state, router);
        return Ok(true);
    }

    if key.code == KeyCode::Char('?') {
        state.help_visible = true;
        return Ok(true);
    }

    match router.current() {
        Some(Screen::Connect) => {
            if key.code == KeyCode::Char('q') {
                return Ok(false);
            }
            handle_connect(key, state, router).await;
        }
        Some(Screen::AddConnection) => handle_add_connection(key, state, router).await,
        Some(Screen::Database) => handle_database(key, state, router, terminal).await?,
        Some(Screen::Inspect) => handle_inspect(key, state, router, terminal).await?,
        Some(Screen::Records) => {
            let terminal_width = terminal.size()?.width;
            handle_records(key, state, router, terminal_width).await?;
        }
        None => return Ok(false),
    }

    Ok(true)
}

// ─── Connect error popup ──────────────────────────────────────────────────────

fn handle_connect_error_popup(key: KeyEvent, state: &mut AppState, router: &Router) -> bool {
    if !matches!(router.current(), Some(Screen::Connect)) || state.connect.error.is_none() {
        return false;
    }
    if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
        state.connect.error = None;
    }
    true
}

// ─── SQL Editor ──────────────────────────────────────────────────────────────

async fn handle_sql_editor(
    key: KeyEvent,
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
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
        KeyCode::Char(c) => {
            let max_col = sql_editor_text_width(terminal.size()?.width);
            state.sql_input.insert_char_wrapped(c, max_col);
        }
        _ => {}
    }
    Ok(())
}

async fn execute_sql_editor_query(
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    router: &mut Router,
) -> std::io::Result<()> {
    state.sql_input.close();
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
                state.sql_input.result = Some(SqlResult::Error(format_sql_error(&e)));
                state.mode = AppMode::Result;
            }
        }
    } else {
        state.execute_sql_input().await;
        state.mode = AppMode::Result;
    }

    Ok(())
}

fn sql_editor_text_width(terminal_width: u16) -> usize {
    let popup_width = terminal_width.saturating_mul(70) / 100;
    popup_width.saturating_sub(7).max(1) as usize
}

// ─── Search ──────────────────────────────────────────────────────────────────

fn handle_search(key: KeyEvent, state: &mut AppState, router: &Router) {
    match key.code {
        KeyCode::Esc => {
            state.search.reset();
            match router.current() {
                Some(Screen::Connect) => state.select_first_filtered_connection(),
                _ => {
                    state.schema_selected = 0;
                    state.table_selected = 0;
                }
            }
            state.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            state.search.close();
            state.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            state.search.query.pop();
            match router.current() {
                Some(Screen::Connect) => state.clamp_connection_selection(),
                _ => state.clamp_search_selections(),
            }
        }
        KeyCode::Down => match router.current() {
            Some(Screen::Connect) => state.select_next_filtered_connection(),
            Some(Screen::Database) => {
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
            _ => {}
        },
        KeyCode::Up => match router.current() {
            Some(Screen::Connect) => state.select_prev_filtered_connection(),
            Some(Screen::Database) => {
                if state.active_pane == ActivePane::Schemas {
                    state.select_prev_filtered_schema();
                } else {
                    let schema = state.selected_schema_name().unwrap_or_default();
                    state.select_prev_filtered_table(&schema);
                }
            }
            _ => {}
        },
        KeyCode::Char(c) => {
            state.search.query.push(c);
            match router.current() {
                Some(Screen::Connect) => state.clamp_connection_selection(),
                _ => state.clamp_search_selections(),
            }
        }
        _ => {}
    }
}

// ─── Connect ─────────────────────────────────────────────────────────────────

async fn handle_connect(key: KeyEvent, state: &mut AppState, router: &mut Router) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.search.query.is_empty() {
                state.select_next_filtered_connection();
            } else {
                state.connect.select_next(state.connections.len());
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !state.search.query.is_empty() {
                state.select_prev_filtered_connection();
            } else {
                state.connect.select_prev(state.connections.len());
            }
        }
        KeyCode::Char('/') => {
            state.search.open();
            state.mode = AppMode::Search;
        }
        KeyCode::Char('a') => {
            state.form.reset();
            state.mode = AppMode::Insert;
            router.push(Screen::AddConnection);
        }
        KeyCode::Char('r') => {
            state.refresh_connection_statuses().await;
        }
        KeyCode::Char('e') => {
            if let Some(config::Connect::Postgres(cfg)) =
                state.connections.get(state.connect.selected)
            {
                state.form = FormState::from_postgres_config(cfg, Some(state.connect.selected));
                state.mode = AppMode::Insert;
                router.push(Screen::AddConnection);
            }
        }
        KeyCode::Char('d') => {
            if !state.connections.is_empty() {
                state.remove_connection_at(state.connect.selected);
                if state.connect.selected >= state.connections.len() {
                    state.connect.selected = state.connections.len().saturating_sub(1);
                }
                state.clamp_connection_selection();
                let _ = ConfigStorage::save(&state.connections);
            }
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if state.selected_filtered_connection_position().is_none() {
                return;
            }
            if !state.connections.is_empty()
                && state.connect_selected().await.is_ok()
                && state.load_schemas().await.is_ok()
            {
                state.active_pane = ActivePane::Schemas;
                router.push(Screen::Database);
            }
        }
        KeyCode::Char('h') | KeyCode::Left => {}
        _ => {}
    }
}

// ─── Add Connection ───────────────────────────────────────────────────────────

async fn handle_add_connection(key: KeyEvent, state: &mut AppState, router: &mut Router) {
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
        KeyCode::Enter => save_connection_form(state, router).await,
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            save_connection_form(state, router).await;
        }
        KeyCode::Char(c) => {
            state.form.current_value_mut().push(c);
        }
        _ => {}
    }
}

async fn save_connection_form(state: &mut AppState, router: &mut Router) {
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
            state.sync_connection_statuses();
            let _ = ConfigStorage::save(&state.connections);
            state
                .refresh_connection_status(state.connect.selected)
                .await;
            state.form.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        Err(msg) => state.form.error = Some(msg),
    }
}

// ─── Database ─────────────────────────────────────────────────────────────────

async fn handle_database(
    key: KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
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
                state.select_prev_filtered_schema();
            } else {
                let schema = state.selected_schema_name().unwrap_or_default();
                state.select_prev_filtered_table(&schema);
            }
        }

        KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => {
            if state.active_pane == ActivePane::Schemas {
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
        KeyCode::Char(':') | KeyCode::Char('c') => {
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
    key: KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> std::io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') | KeyCode::Left => {
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
            }
        }
        _ => {}
    }
    Ok(())
}

// ─── Records ──────────────────────────────────────────────────────────────────

async fn handle_records(
    key: KeyEvent,
    state: &mut AppState,
    router: &mut Router,
    terminal_width: u16,
) -> std::io::Result<()> {
    let is_vertical = state.records.min_table_width > terminal_width;
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.records.reset();
            state.mode = AppMode::Normal;
            router.pop();
        }
        KeyCode::Down | KeyCode::Char('j') if is_vertical => state.records.move_col_right(),
        KeyCode::Up | KeyCode::Char('k') if is_vertical => state.records.move_col_left(),
        KeyCode::Char('l') | KeyCode::Right if is_vertical => state.move_record_down(true).await,
        KeyCode::Char('h') | KeyCode::Left if is_vertical => state.move_record_up(true).await,
        KeyCode::Down | KeyCode::Char('j') => state.move_record_down(false).await,
        KeyCode::Up | KeyCode::Char('k') => state.move_record_up(false).await,
        KeyCode::Char('l') | KeyCode::Right => state.records.move_col_right(),
        KeyCode::Char('h') | KeyCode::Left => state.records.move_col_left(),
        KeyCode::Char('G') => {
            state.records.selected_row = state.records.rows.len().saturating_sub(1);
        }
        KeyCode::Char('n') => {
            if state.records.has_next_page() {
                state.records.next_page();
                let _ = state.fetch_records_page().await;
                state.records.selected_row = 0;
            }
        }
        KeyCode::Char('p') => {
            if state.records.has_prev_page() {
                state.records.prev_page();
                let _ = state.fetch_records_page().await;
                state.records.selected_row = 0;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crossterm::event::KeyEvent;

    fn records_state_with_layout(min_table_width: u16) -> AppState {
        let mut state = AppState::new(vec![]);
        state.records.columns = vec![
            crate::db::repo::tables_repo::ColumnInfo {
                name: "id".to_string(),
            },
            crate::db::repo::tables_repo::ColumnInfo {
                name: "title".to_string(),
            },
        ];
        state.records.rows = vec![
            vec![Some("1".to_string()), Some("hello".to_string())],
            vec![Some("2".to_string()), Some("world".to_string())],
        ];
        state.records.rows_per_page = 2;
        state.records.total_count = 2;
        state.records.min_table_width = min_table_width;
        state
    }

    #[tokio::test]
    async fn records_j_moves_row_down_in_horizontal_layout() {
        let mut state = records_state_with_layout(20);
        let mut router = Router::new();

        handle_records(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut state,
            &mut router,
            60,
        )
        .await
        .expect("handle records");

        assert_eq!(state.records.selected_row, 1);
        assert_eq!(state.records.selected_col, 0);
    }

    #[tokio::test]
    async fn records_l_moves_field_right_in_horizontal_layout() {
        let mut state = records_state_with_layout(20);
        let mut router = Router::new();

        handle_records(
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            &mut state,
            &mut router,
            60,
        )
        .await
        .expect("handle records");

        assert_eq!(state.records.selected_row, 0);
        assert_eq!(state.records.selected_col, 1);
    }

    #[tokio::test]
    async fn records_j_moves_field_down_in_vertical_layout() {
        let mut state = records_state_with_layout(200);
        let mut router = Router::new();

        handle_records(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut state,
            &mut router,
            60,
        )
        .await
        .expect("handle records");

        assert_eq!(state.records.selected_row, 0);
        assert_eq!(state.records.selected_col, 1);
    }

    #[tokio::test]
    async fn records_k_moves_field_up_in_vertical_layout() {
        let mut state = records_state_with_layout(200);
        state.records.selected_col = 1;
        let mut router = Router::new();

        handle_records(
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            &mut state,
            &mut router,
            60,
        )
        .await
        .expect("handle records");

        assert_eq!(state.records.selected_row, 0);
        assert_eq!(state.records.selected_col, 0);
    }

    #[tokio::test]
    async fn records_l_moves_row_down_in_vertical_layout() {
        let mut state = records_state_with_layout(200);
        let mut router = Router::new();

        handle_records(
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            &mut state,
            &mut router,
            60,
        )
        .await
        .expect("handle records");

        assert_eq!(state.records.selected_row, 1);
        assert_eq!(state.records.selected_col, 0);
    }

    #[test]
    fn connect_error_popup_enter_dismisses_and_consumes_input() {
        let mut state = AppState::new(vec![]);
        state.connect.error = Some("failed".to_string());
        let router = Router::new();

        let consumed = handle_connect_error_popup(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut state,
            &router,
        );

        assert!(consumed);
        assert!(state.connect.error.is_none());
    }

    #[test]
    fn connect_error_popup_esc_dismisses_and_consumes_input() {
        let mut state = AppState::new(vec![]);
        state.connect.error = Some("failed".to_string());
        let router = Router::new();

        let consumed = handle_connect_error_popup(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &mut state,
            &router,
        );

        assert!(consumed);
        assert!(state.connect.error.is_none());
    }

    #[test]
    fn connect_error_popup_consumes_other_keys_without_dismissing() {
        let mut state = AppState::new(vec![]);
        state.connect.error = Some("failed".to_string());
        let router = Router::new();

        let consumed = handle_connect_error_popup(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut state,
            &router,
        );

        assert!(consumed);
        assert!(state.connect.error.is_some());
    }
}
