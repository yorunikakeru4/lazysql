use crate::state::app::AppState;
use crate::state::records::{MAX_CELL_LEN, RecordsSource};
use crate::ui::widgets;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};

const HINTS: &[(&str, &str)] = &[
    ("j/k", "row/field"),
    ("h/l", "field/row"),
    ("n/p", "page"),
    ("q", "close"),
];

/// Renders the record paging screen with vim navigation.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    let records = &state.records;
    let col_name = records.current_col_name().unwrap_or("—");
    let context = match &records.source {
        Some(RecordsSource::Table { table, .. }) => format!("records · {table}"),
        Some(RecordsSource::Query { .. }) => "query results".into(),
        None => "records".into(),
    };
    let status_hints = format!("col {col_name}");

    widgets::hintbar::render(frame, chunks[0], &state.theme.colors, HINTS);
    render_table(frame, chunks[1], state);
    widgets::statusbar::render(
        frame,
        chunks[2],
        &state.mode,
        &state.theme.colors,
        &context,
        &status_hints,
    );
}

fn render_table(frame: &mut Frame, area: Rect, state: &AppState) {
    let colors = &state.theme.colors;
    let records = &state.records;
    if records.columns.is_empty() {
        frame.render_widget(Paragraph::new("No results.").block(Block::bordered()), area);
        return;
    }

    if records.min_table_width > area.width {
        render_expanded(frame, area, state);
        return;
    }

    let header = Row::new(
        records
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let style = if i == records.selected_col {
                    Style::new().fg(colors.orange).bold()
                } else {
                    Style::new().fg(colors.fg1).bold()
                };
                Cell::from(c.name.as_str()).style(style)
            })
            .collect::<Vec<_>>(),
    );

    let data_rows: Vec<Row> = records
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let is_sel_row = row_idx == records.selected_row;
            let cells = row
                .iter()
                .enumerate()
                .map(|(col_idx, val)| {
                    let truncated = truncate_cell(val.as_deref().unwrap_or("NULL"));
                    let style = if is_sel_row && col_idx == records.selected_col {
                        Style::new().bg(colors.bg_sel).fg(colors.orange)
                    } else if is_sel_row {
                        Style::new().bg(colors.bg_sel).fg(colors.fg0)
                    } else {
                        Style::new().fg(colors.fg1)
                    };
                    Cell::from(truncated).style(style)
                })
                .collect::<Vec<_>>();
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = records
        .table_column_widths()
        .into_iter()
        .map(|w| Constraint::Length(w.saturating_add(2)))
        .collect();

    let title = format!(
        " {} rows · page {}/{} ",
        records.total_count,
        records.current_page(),
        records.total_pages()
    );

    let mut table_state = TableState::default().with_selected(Some(records.selected_row));
    let table = Table::new(data_rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .title(title)
                .border_style(Style::new().fg(colors.secondary)),
        )
        .row_highlight_style(Style::new().bg(colors.bg_sel));

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_expanded(frame: &mut Frame, area: Rect, state: &AppState) {
    let colors = &state.theme.colors;
    let records = &state.records;
    let mut rows = Vec::new();
    for (row_idx, row) in records.rows.iter().enumerate() {
        let record_num = records.offset + row_idx as u64 + 1;
        rows.push(
            Row::new(vec![
                Cell::from(format!("-[ RECORD {record_num} ]")),
                Cell::from(""),
            ])
            .style(Style::new().fg(colors.primary).bold()),
        );

        for (col_idx, col) in records.columns.iter().enumerate() {
            let text = row
                .get(col_idx)
                .and_then(|value| value.as_deref())
                .unwrap_or("NULL");
            let is_selected_field =
                row_idx == records.selected_row && col_idx == records.selected_col;
            let name_style = if is_selected_field {
                Style::new().fg(colors.orange).bold()
            } else {
                Style::new().fg(colors.fg2)
            };
            let value_style = if is_selected_field {
                Style::new().fg(colors.orange)
            } else {
                Style::new().fg(colors.fg1)
            };

            rows.push(Row::new(vec![
                Cell::from(col.name.clone()).style(name_style),
                Cell::from(truncate_cell(text)).style(value_style),
            ]));
        }
        rows.push(Row::new(vec![Cell::from(""), Cell::from("")]));
    }

    let title = format!(
        " {} rows · page {}/{} · expanded ",
        records.total_count,
        records.current_page(),
        records.total_pages()
    );
    let name_width = records
        .columns
        .iter()
        .map(|col| col.name.chars().count() as u16)
        .max()
        .unwrap_or(1)
        .saturating_add(2)
        .min(area.width.saturating_sub(8).max(1));
    let widths = [Constraint::Length(name_width), Constraint::Fill(1)];
    let num_cols = records.columns.len();
    let flat_pos = records
        .selected_row
        .saturating_mul(num_cols.saturating_add(2))
        .saturating_add(1)
        .saturating_add(records.selected_col);

    let table = Table::new(rows, widths)
        .row_highlight_style(Style::default())
        .block(
            Block::bordered()
                .title(title)
                .border_style(Style::new().fg(colors.secondary)),
        )
        .style(Style::new().fg(colors.fg1));

    let mut table_state = TableState::default().with_selected(Some(flat_pos));
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn truncate_cell(text: &str) -> String {
    if text.chars().count() <= MAX_CELL_LEN {
        return text.to_string();
    }
    let boundary = text
        .char_indices()
        .nth(MAX_CELL_LEN)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    let truncated = &text[..boundary];
    format!("{truncated}…")
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::repo::sql_repo::ColumnInfo;
    use ratatui::{Terminal, backend::TestBackend};

    fn app_with_expanded_records() -> AppState {
        let mut state = AppState::for_test(vec![]);
        state.records = crate::state::records::RecordsState::for_table(
            "public".to_string(),
            "notifications".to_string(),
        );
        state.records.columns = vec![
            ColumnInfo {
                name: "id".to_string(),
            },
            ColumnInfo {
                name: "title".to_string(),
            },
        ];
        state.records.rows = vec![vec![
            Some("19".to_string()),
            Some("Notification".to_string()),
        ]];
        state.records.total_count = 1;
        state.records.rows_per_page = 1;
        state.records.min_table_width = 200;
        state.records.selected_col = 1;
        state
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.cell((x, y)).expect("cell in bounds");
                text.push_str(cell.symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn expanded_records_render_as_two_columns_without_pipe_separator() {
        let state = app_with_expanded_records();
        let backend = TestBackend::new(60, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal
            .draw(|frame| render(frame, &state))
            .expect("draw records");

        let text = buffer_text(&terminal);
        assert!(!text.contains("id | 19"));
        assert!(text.contains("id"));
        assert!(text.contains("19"));
    }

    #[test]
    fn expanded_records_highlight_selected_field_value_without_background() {
        let state = app_with_expanded_records();
        let backend = TestBackend::new(60, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal
            .draw(|frame| render(frame, &state))
            .expect("draw records");

        let highlighted = terminal.backend().buffer().content().iter().any(|cell| {
            cell.symbol() == "N"
                && cell.bg == ratatui::style::Color::Reset
                && cell.fg == state.theme.colors.orange
        });
        assert!(highlighted);
    }
}
