use crate::config::Connect;
use crate::db::repo::tables_repo::TableField;
use crate::state::app::AppState;
use crate::state::connection::FIELD_LABELS;
use crate::state::navigation::{Router, Screen};
use crate::state::records::{MAX_CELL_LEN, RecordsState};
use crate::state::sql_input::SqlResult;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, state: &AppState, router: &Router) {
    match router.current() {
        Some(Screen::Connect) => render_connect(frame, state),
        Some(Screen::AddConnection) => render_add_connection(frame, state),
        Some(Screen::Schemas) => render_schemas(frame, state),
        Some(Screen::Tables) => render_tables(frame, state),
        Some(Screen::TableView) => render_table_view(frame, state),
        Some(Screen::Records) => render_records(frame, state),
        None => {}
    }

    // SQL input bar (overlay at bottom)
    if state.sql_input.active {
        render_sql_input_bar(frame, state);
    }

    // SQL result popup (centered overlay, always on top)
    if state.sql_input.has_result() {
        render_sql_result_popup(frame, state);
    }
}

fn render_add_connection(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Outer wrapper with title and hint
    let outer = Block::bordered()
        .title(" Add Connection (j/k or Tab navigate fields, Enter save, Esc cancel) ");

    let inner_area = outer.inner(area);
    frame.render_widget(outer, area);

    // 5 field rows (3 lines each) + 1 status row (1 line, no border)
    let field_height = 3u16;
    let constraints: Vec<Constraint> = (0..FIELD_LABELS.len())
        .map(|_| Constraint::Length(field_height))
        .chain(std::iter::once(Constraint::Length(1)))
        .collect();

    let chunks = Layout::vertical(constraints).split(inner_area);

    for (i, label) in FIELD_LABELS.iter().enumerate() {
        let is_focused = i == state.form.focused;

        let display_value = if i == 4 {
            // password: mask with asterisks
            "*".repeat(state.form.values[i].len())
        } else {
            state.form.values[i].clone()
        };

        let block = if is_focused {
            Block::bordered().title(format!(" {label} ")).bold()
        } else {
            Block::bordered().title(format!(" {label} "))
        };

        let paragraph = Paragraph::new(display_value.as_str()).block(block);
        frame.render_widget(paragraph, chunks[i]);

        // Place cursor at end of focused field (not for password field)
        if is_focused && i != 4 {
            let inner = chunks[i];
            // content area starts 1 inside the border
            let cursor_x = inner.x + 1 + state.form.values[i].len() as u16;
            let cursor_y = inner.y + 1;
            // clamp to field width
            let max_x = inner.x + inner.width.saturating_sub(2);
            frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
        }
    }

    // Status row: validation error (no border, red text)
    if let Some(err) = &state.form.error {
        frame.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            chunks[FIELD_LABELS.len()],
        );
    }
}

fn render_connect(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(area);

    let items: Vec<ListItem> = state
        .connections
        .iter()
        .map(|c| {
            let label = match c {
                Connect::Postgres(cfg) => {
                    format!("{}@{}:{}/{}", cfg.user, cfg.host, cfg.port, cfg.db_name)
                }
            };
            ListItem::new(label)
        })
        .collect();

    let mut list_state = ListState::default().with_selected(Some(state.connect.selected));
    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" lazysql — Connections (j/k navigate, l/Enter connect, a add, q quit) "),
        )
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let hint = state
        .connect
        .error
        .as_deref()
        .unwrap_or("No connections saved. Edit ~/.config/lazysql/config.toml to add one.");
    frame.render_widget(
        Paragraph::new(hint).block(Block::bordered().title(" Status ")),
        chunks[1],
    );
}

/// Renders the search bar at `area`. Shows cursor when active.
fn render_search_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let (title, text) = if state.search.active {
        (" / ", format!("{}_", state.search.query))
    } else {
        (" Filter ", format!("/{}", state.search.query))
    };

    frame.render_widget(
        Paragraph::new(text.as_str()).block(Block::bordered().title(title)),
        area,
    );

    if state.search.active {
        // cursor sits after the query text, inside the border
        let cursor_x = (area.x + 1 + state.search.query.len() as u16)
            .min(area.x + area.width.saturating_sub(2));
        frame.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

fn render_schemas(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_bar = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_bar {
        vec![Constraint::Fill(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Fill(1)]
    })
    .split(area);

    let names = state.filtered_schema_names();
    let items: Vec<ListItem> = names.iter().map(|n| ListItem::new(n.as_str())).collect();
    let mut list_state = ListState::default().with_selected(Some(state.schema_selected));
    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" Schemas (j/k navigate, l/Enter select, / search, h/Esc back) "),
        )
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    if show_bar {
        render_search_bar(frame, chunks[1], state);
    }
}

fn render_tables(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_bar = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_bar {
        vec![Constraint::Fill(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Fill(1)]
    })
    .split(area);

    let schema = state.selected_schema_name().unwrap_or_default();
    let names = state.filtered_table_names(&schema);

    let items: Vec<ListItem> = names.iter().map(|n| ListItem::new(n.as_str())).collect();
    let mut list_state = ListState::default().with_selected(Some(state.table_selected));
    let list = List::new(items)
        .block(Block::bordered().title(format!(
            " Tables in '{schema}' (j/k navigate, l/Enter view, / search, h/Esc back) "
        )))
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    if show_bar {
        render_search_bar(frame, chunks[1], state);
    }
}

fn render_table_view(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_bar = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_bar {
        vec![Constraint::Fill(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Fill(1)]
    })
    .split(area);

    let Some(table) = &state.loaded_table else {
        frame.render_widget(
            Paragraph::new("No table loaded.").block(Block::bordered()),
            chunks[0],
        );
        return;
    };

    let fields: Vec<&TableField> = table
        .fields
        .iter()
        .filter(|f| state.search.matches(&f.name))
        .collect();

    let content = format_fields(&fields);
    frame.render_widget(
        Paragraph::new(content).block(
            Block::bordered().title(format!(" Table: {} (/ search, h/Esc back) ", table.name)),
        ),
        chunks[0],
    );

    if show_bar {
        render_search_bar(frame, chunks[1], state);
    }
}

/// Renders the SQL input bar at the bottom of the screen.
fn render_sql_input_bar(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    // 3 lines for bordered input at the bottom
    let bar_area = Rect {
        x: area.x,
        y: area.height.saturating_sub(3),
        width: area.width,
        height: 3,
    };

    frame.render_widget(Clear, bar_area);

    let display_query = state.sql_input.query.replace('\n', " ↵ ");
    let text = format!(":{}", display_query);
    let paragraph = Paragraph::new(text.as_str())
        .block(Block::bordered().title(" SQL (Shift+Enter newline, Esc cancel) "));
    frame.render_widget(paragraph, bar_area);

    // Cursor position: after the colon and query
    let cursor_x = (bar_area.x + 1 + 1 + display_query.len() as u16)
        .min(bar_area.x + bar_area.width.saturating_sub(2));
    frame.set_cursor_position(Position::new(cursor_x, bar_area.y + 1));
}

/// Renders a centered popup showing SQL execution result.
fn render_sql_result_popup(frame: &mut Frame, state: &AppState) {
    let Some(result) = &state.sql_input.result else {
        return;
    };

    let (title, body, color) = match result {
        SqlResult::Success { rows_affected } => (
            " Success ",
            format!("Rows Affected: {}", rows_affected),
            Color::Green,
        ),
        SqlResult::Rows { count } => (
            " Success ",
            format!("Rows Returned: {}", count),
            Color::Green,
        ),
        SqlResult::Error(msg) => (" Error ", msg.clone(), Color::Red),
    };

    let popup_area = centered_rect(50, 5, frame.area());
    frame.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(format!("{}\n\nPress Enter to dismiss", body)).block(
        Block::bordered()
            .title(title)
            .style(Style::default().fg(color)),
    );
    frame.render_widget(paragraph, popup_area);
}

/// Returns a centered Rect of given percentage width and fixed height.
fn centered_rect(percent_width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_width / 100;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height,
    }
}

fn render_records(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    // Layout: content + status bar (3 lines)
    let chunks = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(area);

    let records = &state.records;
    let title = match &records.source {
        Some(crate::state::records::RecordsSource::Table { table, .. }) => {
            format!(" Records: {} ", table)
        }
        Some(crate::state::records::RecordsSource::Query { .. }) => " Query Results ".to_string(),
        None => " Records ".to_string(),
    };

    let content = format_records_table(records);

    frame.render_widget(
        Paragraph::new(content)
            .block(Block::bordered().title(format!("{} (h/← prev, l/→ next, Esc back)", title))),
        chunks[0],
    );

    // Status bar: pagination info
    let start_row = records.offset + 1;
    let end_row = (records.offset + records.rows.len() as u64).min(records.total_count);
    let status = format!(
        "Page {}/{} | Rows {}-{} of {}",
        records.current_page(),
        records.total_pages(),
        start_row,
        end_row,
        records.total_count
    );
    frame.render_widget(
        Paragraph::new(status).block(Block::bordered().title(" Status ")),
        chunks[1],
    );
}

/// Truncates a cell value to MAX_CELL_LEN characters, appending "..." if cut.
fn truncate_cell(s: &str) -> String {
    if s.chars().count() <= MAX_CELL_LEN {
        return s.to_string();
    }
    let boundary = s
        .char_indices()
        .nth(MAX_CELL_LEN - 3)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    format!("{}...", &s[..boundary])
}

/// Formats records as a table with aligned columns.
fn format_records_table(records: &RecordsState) -> String {
    const COL_GAP: &str = "   ";

    if records.columns.is_empty() {
        return "No data".to_string();
    }

    // Calculate column widths (capped at MAX_CELL_LEN)
    let mut widths: Vec<usize> = records.columns.iter().map(|c| c.name.len()).collect();
    for row in &records.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                let cell_len = cell
                    .as_ref()
                    .map(|s| s.chars().count().min(MAX_CELL_LEN))
                    .unwrap_or(4); // "NULL"
                widths[i] = widths[i].max(cell_len);
            }
        }
    }

    // Header
    let header: String = records
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{:<width$}", col.name, width = widths[i]))
        .collect::<Vec<_>>()
        .join(COL_GAP);

    // Separator
    let separator = "-".repeat(widths.iter().sum::<usize>() + COL_GAP.len() * (widths.len() - 1));

    // Rows
    let rows: String = records
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, cell)| {
                    let raw = cell.as_ref().map(|s| s.as_str()).unwrap_or("NULL");
                    let val = truncate_cell(raw);
                    let w = widths.get(i).copied().unwrap_or(val.chars().count());
                    format!("{:<width$}", val, width = w)
                })
                .collect::<Vec<_>>()
                .join(COL_GAP)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}\n{}\n{}", header, separator, rows)
}

fn format_fields(fields: &[&TableField]) -> String {
    const COLUMN_GAP: &str = "   ";

    let mut rows: Vec<(&str, &str, &str, String, String)> = Vec::with_capacity(fields.len());
    for f in fields {
        let constraint = f
            .constraint_type
            .as_ref()
            .map(|c| format!("{c:?}"))
            .unwrap_or_default();
        let default = f.default.clone().unwrap_or_default();
        rows.push((&f.name, &f.data_type, &f.is_nullable, constraint, default));
    }

    let name_w = rows
        .iter()
        .map(|(name, _, _, _, _)| name.len())
        .max()
        .unwrap_or(0)
        .max("Column".len());
    let type_w = rows
        .iter()
        .map(|(_, data_type, _, _, _)| data_type.len())
        .max()
        .unwrap_or(0)
        .max("Type".len());
    let nullable_w = rows
        .iter()
        .map(|(_, _, nullable, _, _)| nullable.len())
        .max()
        .unwrap_or(0)
        .max("Nullable".len());
    let constraint_w = rows
        .iter()
        .map(|(_, _, _, constraint, _)| constraint.len())
        .max()
        .unwrap_or(0)
        .max("Constraint".len());

    let header = format!(
        "{:<name_w$}{COLUMN_GAP}{:<type_w$}{COLUMN_GAP}{:<nullable_w$}{COLUMN_GAP}{:<constraint_w$}{COLUMN_GAP}{}\n",
        "Column", "Type", "Nullable", "Constraint", "Default",
    );
    let separator = format!(
        "{}\n",
        "-".repeat(
            name_w + type_w + nullable_w + constraint_w + "Default".len() + COLUMN_GAP.len() * 4,
        )
    );
    let body: String = rows
        .iter()
        .map(|(name, data_type, nullable, constraint, default)| {
            format!(
                "{:<name_w$}{COLUMN_GAP}{:<type_w$}{COLUMN_GAP}{:<nullable_w$}{COLUMN_GAP}{:<constraint_w$}{COLUMN_GAP}{}\n",
                name, data_type, nullable, constraint, default
            )
        })
        .collect();

    format!("{header}{separator}{body}")
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::repo::tables_repo::{ColumnInfo, TableField};

    #[test]
    fn nullable_column_stays_aligned_with_long_type_values() {
        let short = TableField {
            name: "id".to_string(),
            data_type: "integer".to_string(),
            is_nullable: "NO".to_string(),
            constraint_type: None,
            default: None,
        };
        let long = TableField {
            name: "payload".to_string(),
            data_type: "character varying with very very long suffix".to_string(),
            is_nullable: "YES".to_string(),
            constraint_type: None,
            default: None,
        };

        let rendered = format_fields(&[&short, &long]);
        let lines: Vec<&str> = rendered.lines().collect();

        let nullable_short = lines[2].find("NO").unwrap();
        let nullable_long = lines[3].find("YES").unwrap();
        assert_eq!(nullable_short, nullable_long);
    }

    #[test]
    fn columns_have_wider_gap_between_each_other() {
        let field = TableField {
            name: "id".to_string(),
            data_type: "integer".to_string(),
            is_nullable: "NO".to_string(),
            constraint_type: None,
            default: None,
        };

        let rendered = format_fields(&[&field]);
        let header = rendered.lines().next().unwrap();
        let column_end = header.find("Column").unwrap() + "Column".len();
        let type_start = header.find("Type").unwrap();
        let nullable_start = header.find("Nullable").unwrap();
        let constraint_start = header.find("Constraint").unwrap();
        let default_start = header.find("Default").unwrap();

        assert!(type_start - column_end >= 3);
        assert!(nullable_start - (type_start + "Type".len()) >= 3);
        assert!(constraint_start - (nullable_start + "Nullable".len()) >= 3);
        assert!(default_start - (constraint_start + "Constraint".len()) >= 3);
    }

    #[test]
    fn format_records_table_aligns_columns() {
        let records = RecordsState {
            columns: vec![
                ColumnInfo {
                    name: "id".into(),
                    data_type: "int4".into(),
                },
                ColumnInfo {
                    name: "name".into(),
                    data_type: "text".into(),
                },
            ],
            rows: vec![
                vec![Some("1".into()), Some("Alice".into())],
                vec![Some("2".into()), Some("Bob".into())],
            ],
            ..Default::default()
        };
        let output = format_records_table(&records);
        assert!(output.contains("id"));
        assert!(output.contains("name"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn format_records_table_handles_null() {
        let records = RecordsState {
            columns: vec![ColumnInfo {
                name: "val".into(),
                data_type: "text".into(),
            }],
            rows: vec![vec![None]],
            ..Default::default()
        };
        let output = format_records_table(&records);
        assert!(output.contains("NULL"));
    }

    #[test]
    fn format_records_table_empty_returns_no_data() {
        let records = RecordsState::default();
        let output = format_records_table(&records);
        assert_eq!(output, "No data");
    }
}
