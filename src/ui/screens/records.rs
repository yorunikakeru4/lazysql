use crate::state::app::AppState;
use crate::state::records::{MAX_CELL_LEN, RecordsSource};
use crate::ui::{theme, widgets};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};

const HINTS: &[(&str, &str)] = &[
    ("j/k", "row"),
    ("h/l", "col"),
    ("yy", "yank row"),
    ("y", "yank cell"),
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
        Some(RecordsSource::Table { table, .. }) => format!("records · {}", table),
        Some(RecordsSource::Query { .. }) => "query results".into(),
        None => "records".into(),
    };
    let status_hints = format!(
        "row {}/{}  col {}",
        records.selected_row + 1,
        records.rows.len(),
        col_name
    );

    widgets::hintbar::render(frame, chunks[0], HINTS);
    render_table(frame, chunks[1], state);
    widgets::statusbar::render(frame, chunks[2], &state.mode, &context, &status_hints);
}

fn render_table(frame: &mut Frame, area: Rect, state: &AppState) {
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
                    Style::new().fg(theme::ORANGE).bold()
                } else {
                    Style::new().fg(theme::FG4).bold()
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
                        Style::new().bg(theme::BG_SEL).fg(theme::ORANGE)
                    } else if is_sel_row {
                        Style::new().bg(theme::BG_SEL).fg(theme::FG0)
                    } else {
                        Style::new().fg(theme::FG3)
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
                .border_style(Style::new().fg(theme::BG3)),
        )
        .row_highlight_style(Style::new().bg(theme::BG_SEL));

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_expanded(frame: &mut Frame, area: Rect, state: &AppState) {
    let records = &state.records;
    let mut lines = Vec::new();
    for (row_idx, row) in records.rows.iter().enumerate() {
        let record_num = records.offset + row_idx as u64 + 1;
        lines.push(format!("-[ RECORD {} ]", record_num));
        for (col_idx, col) in records.columns.iter().enumerate() {
            let text = row
                .get(col_idx)
                .and_then(|value| value.as_deref())
                .unwrap_or("NULL");
            lines.push(format!("{} | {}", col.name, truncate_cell(text)));
        }
        lines.push(String::new());
    }

    let title = format!(
        " {} rows · page {}/{} · expanded ",
        records.total_count,
        records.current_page(),
        records.total_pages()
    );
    frame.render_widget(
        Paragraph::new(lines.join("\n")).block(
            Block::bordered()
                .title(title)
                .border_style(Style::new().fg(theme::BG3)),
        ),
        area,
    );
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
    format!("{}…", &text[..boundary])
}
