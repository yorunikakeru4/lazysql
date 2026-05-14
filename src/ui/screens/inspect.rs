use crate::db::repo::tables_repo::{ConstraintType, FkRef, IndexInfo};
use crate::state::app::AppState;
use crate::ui::{theme, widgets};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table},
};

const HINTS: &[(&str, &str)] = &[
    ("r", "rows"),
    ("s", "sample"),
    ("/", "filter"),
    ("q", "back"),
];

/// Renders the full table schema inspect screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    let Some(details) = &state.table_details else {
        frame.render_widget(
            Paragraph::new("No table loaded.").block(Block::bordered()),
            area,
        );
        return;
    };

    let context = format!("{}.{}", details.schema, details.name);
    widgets::hintbar::render(frame, chunks[0], HINTS);
    render_body(frame, chunks[1], state);
    widgets::statusbar::render(frame, chunks[2], &state.mode, &context, "r:rows  /:filter");
}

fn render_body(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(details) = &state.table_details else {
        return;
    };

    let body_chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(4),
    ])
    .split(area);

    // Header line
    let rows_str = details
        .row_count
        .map(|n| format!("{} rows", n))
        .unwrap_or_else(|| "? rows".into());
    let size_str = details.size_pretty.clone().unwrap_or_else(|| "?".into());
    let header_text = format!(
        " {}.{} · {} · {} · {} columns ",
        details.schema,
        details.name,
        rows_str,
        size_str,
        details.fields.len()
    );
    frame.render_widget(
        Paragraph::new(header_text).style(Style::new().fg(theme::FG3).bg(theme::BG1)),
        body_chunks[0],
    );

    // Columns table
    let fields: Vec<_> = details
        .fields
        .iter()
        .filter(|f| state.search.matches(&f.name))
        .collect();

    let col_header = Row::new(vec![
        Cell::from("COLUMN").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("TYPE").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("NULL").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("CONSTRAINT").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("DEFAULT").style(Style::new().fg(theme::FG4).bold()),
    ]);

    let rows: Vec<Row> = fields
        .iter()
        .map(|f| {
            let null_cell = if f.is_nullable == "YES" {
                Cell::from("YES").style(Style::new().fg(theme::GREEN))
            } else {
                Cell::from("NO").style(Style::new().fg(theme::RED))
            };

            let constraint_cell = match &f.constraint {
                Some(ConstraintType::PrimaryKey) => {
                    Cell::from("PRIMARY KEY").style(Style::new().fg(theme::YELLOW))
                }
                Some(ConstraintType::Unique) => {
                    Cell::from("UNIQUE").style(Style::new().fg(theme::YELLOW))
                }
                Some(ConstraintType::ForeignKey(t)) => {
                    Cell::from(format!("FK → {}", t)).style(Style::new().fg(theme::BLUE))
                }
                None => Cell::from("—").style(Style::new().fg(theme::FG4)),
            };

            let default_style = if f
                .default_value
                .as_deref()
                .map(|d| d.contains('('))
                .unwrap_or(false)
            {
                Style::new().fg(theme::AQUA)
            } else {
                Style::new().fg(theme::PURPLE)
            };
            let default_cell =
                Cell::from(f.default_value.as_deref().unwrap_or("—")).style(default_style);

            Row::new(vec![
                Cell::from(f.name.as_str()).style(Style::new().fg(theme::FG0)),
                Cell::from(f.data_type.as_str()).style(Style::new().fg(theme::BLUE)),
                null_cell,
                constraint_cell,
                default_cell,
            ])
        })
        .collect();

    let widths = [
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Length(5),
        Constraint::Fill(2),
        Constraint::Fill(2),
    ];

    let table = Table::new(rows, widths)
        .header(col_header)
        .block(Block::bordered().border_style(Style::new().fg(theme::BG3)));
    frame.render_widget(table, body_chunks[1]);

    // Footer: indexes + FK refs
    let idx_str = if details.indexes.is_empty() {
        "—".into()
    } else {
        details
            .indexes
            .iter()
            .map(|idx: &IndexInfo| idx.name.as_str())
            .collect::<Vec<_>>()
            .join("  ")
    };
    let fk_str = if details.fk_refs.is_empty() {
        "—".into()
    } else {
        details
            .fk_refs
            .iter()
            .map(|fk: &FkRef| format!("{}.{}", fk.from_table, fk.column))
            .collect::<Vec<_>>()
            .join("  ")
    };

    let footer_lines = vec![
        Line::from(vec![
            Span::styled("  indexes  ", Style::new().fg(theme::ORANGE)),
            Span::styled(idx_str, Style::new().fg(theme::FG3)),
        ]),
        Line::from(vec![
            Span::styled("  fk-refs  ", Style::new().fg(theme::ORANGE)),
            Span::styled(fk_str, Style::new().fg(theme::FG3)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(footer_lines)
            .block(Block::bordered().border_style(Style::new().fg(theme::BG3))),
        body_chunks[2],
    );
}
