use crate::state::app::AppState;
use crate::ui::{layout::centered_rect, theme};
use ratatui::{
    Frame,
    layout::Position,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

const SQL_KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "JOIN",
    "LEFT",
    "RIGHT",
    "INNER",
    "OUTER",
    "ON",
    "GROUP",
    "BY",
    "ORDER",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "CREATE",
    "DROP",
    "ALTER",
    "TABLE",
    "INDEX",
    "AS",
    "AND",
    "OR",
    "NOT",
    "NULL",
    "IS",
    "IN",
    "LIKE",
    "BETWEEN",
    "DISTINCT",
    "COUNT",
    "SUM",
    "AVG",
    "MAX",
    "MIN",
    "WITH",
    "UNION",
    "ALL",
    "EXCEPT",
    "INTERSECT",
    "RETURNING",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "CAST",
];

/// Renders the floating multiline SQL editor overlay.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let popup = centered_rect(70, 20, area);
    frame.render_widget(Clear, popup);

    let driver = if state.current_db.is_some() {
        "postgres"
    } else {
        "—"
    };
    let title = format!(
        " SQL Editor ─── {} ─── Ctrl+E execute · Enter newline · Tab indent · Esc close ",
        driver
    );

    let query = &state.sql_input.query;
    let (cursor_line, cursor_col) = state.sql_input.cursor_line_col();

    let lines: Vec<Line> = query
        .split('\n')
        .enumerate()
        .map(|(line_idx, line_text)| {
            let line_num = Span::styled(
                format!("{:>3}  ", line_idx + 1),
                Style::new().fg(theme::FG4),
            );

            let mut spans = vec![line_num];
            spans.extend(highlight_sql(line_text));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(title)
                    .border_style(Style::new().fg(theme::ORANGE)),
            )
            .style(Style::new().bg(theme::BG1)),
        popup,
    );

    // Position terminal cursor (for accessibility / IME)
    let cursor_x = (popup.x + 5 + cursor_col as u16).min(popup.x + popup.width.saturating_sub(2));
    let cursor_y = (popup.y + 1 + cursor_line as u16).min(popup.y + popup.height.saturating_sub(2));
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}

/// Basic word-by-word SQL keyword highlight for a single line segment.
fn highlight_sql(text: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let matched = SQL_KEYWORDS.iter().find(|&&kw| {
            let upper = remaining.to_uppercase();
            if upper.starts_with(kw) {
                let end = kw.len();
                remaining
                    .as_bytes()
                    .get(end)
                    .map(|c| !c.is_ascii_alphanumeric() && *c != b'_')
                    .unwrap_or(true)
            } else {
                false
            }
        });
        if let Some(&kw) = matched {
            spans.push(Span::styled(
                &remaining[..kw.len()],
                Style::new().fg(theme::PURPLE),
            ));
            remaining = &remaining[kw.len()..];
        } else {
            let end = remaining
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
            spans.push(Span::raw(&remaining[..end]));
            remaining = &remaining[end..];
        }
    }
    spans
}
