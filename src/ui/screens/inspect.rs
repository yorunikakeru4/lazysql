use crate::db::repo::sql_repo::{ConstraintType, FkRef, IndexInfo};
use crate::state::app::AppState;
use crate::ui::widgets;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};

const HINTS: &[(&str, &str)] = &[
    ("r", "rows"),
    ("s", "sample"),
    ("/", "filter"),
    ("q", "back"),
];

const DEFAULT_DISPLAY_CAP: usize = 48;

/// Renders the full table schema inspect screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_search = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_search {
        vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]
    })
    .split(area);

    let Some(details) = &state.table_details else {
        frame.render_widget(
            Paragraph::new("No table loaded.").block(Block::bordered()),
            area,
        );
        return;
    };

    let schema = &details.schema;
    let name = &details.name;
    let context = format!("{schema}.{name}");
    widgets::hintbar::render(frame, chunks[0], &state.theme.colors, HINTS);
    render_body(frame, chunks[1], state);
    let status_idx = if show_search {
        widgets::search::render_search_bar(frame, chunks[2], state);
        3
    } else {
        2
    };
    widgets::statusbar::render(
        frame,
        chunks[status_idx],
        &state.mode,
        &state.theme.colors,
        &context,
        "r:rows  /:filter",
    );
}

fn render_body(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(details) = &state.table_details else {
        return;
    };
    let colors = &state.theme.colors;

    let body_chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(4),
    ])
    .split(area);

    // Header line
    let rows_str = details
        .row_count
        .map(|n| format!("{n} rows"))
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
        Paragraph::new(header_text).style(Style::new().fg(colors.fg1).bg(colors.bg1)),
        body_chunks[0],
    );

    // Columns table
    let fields: Vec<_> = details
        .fields
        .iter()
        .filter(|f| state.search.matches(&f.name))
        .collect();

    let col_header = Row::new(vec![
        Cell::from("COLUMN").style(Style::new().fg(colors.fg1).bold()),
        Cell::from("TYPE").style(Style::new().fg(colors.fg1).bold()),
        Cell::from("NULL").style(Style::new().fg(colors.fg1).bold()),
        Cell::from("CONSTRAINT").style(Style::new().fg(colors.fg1).bold()),
        Cell::from("DEFAULT").style(Style::new().fg(colors.fg1).bold()),
    ]);

    let rows: Vec<Row> = fields
        .iter()
        .map(|f| {
            let null_cell = if f.is_nullable == "YES" {
                Cell::from("YES").style(Style::new().fg(colors.green))
            } else {
                Cell::from("NO").style(Style::new().fg(colors.red))
            };

            let constraint_cell = match &f.constraint {
                Some(ConstraintType::PrimaryKey) => {
                    Cell::from("PRIMARY KEY").style(Style::new().fg(colors.yellow))
                }
                Some(ConstraintType::Unique) => {
                    Cell::from("UNIQUE").style(Style::new().fg(colors.yellow))
                }
                Some(ConstraintType::ForeignKey(t)) => {
                    Cell::from(format!("FK → {t}")).style(Style::new().fg(colors.blue))
                }
                None => Cell::from("—").style(Style::new().fg(colors.fg2)),
            };

            let default_style = if f
                .default_value
                .as_deref()
                .map(|d| d.contains('('))
                .unwrap_or(false)
            {
                Style::new().fg(colors.aqua)
            } else {
                Style::new().fg(colors.purple)
            };
            let default_cell =
                Cell::from(f.default_value.as_deref().unwrap_or("—")).style(default_style);

            Row::new(vec![
                Cell::from(f.name.as_str()).style(Style::new().fg(colors.fg0)),
                Cell::from(f.data_type.as_str()).style(Style::new().fg(colors.blue)),
                null_cell,
                constraint_cell,
                default_cell,
            ])
        })
        .collect();

    let widths = inspect_column_widths(&fields).map(Constraint::Length);

    let table = Table::new(rows, widths)
        .header(col_header)
        .row_highlight_style(Style::new().bg(colors.bg_sel))
        .block(Block::bordered().border_style(Style::new().fg(colors.bg2)));
    let clamped = state.inspect.selected.min(fields.len().saturating_sub(1));
    let mut table_state = TableState::default().with_selected(Some(clamped));
    frame.render_stateful_widget(table, body_chunks[1], &mut table_state);

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
            .map(|fk: &FkRef| {
                let table = &fk.from_table;
                let column = &fk.column;
                format!("{table}.{column}")
            })
            .collect::<Vec<_>>()
            .join("  ")
    };

    let footer_lines = vec![
        Line::from(vec![
            Span::styled("  indexes  ", Style::new().fg(colors.primary)),
            Span::styled(idx_str, Style::new().fg(colors.fg1)),
        ]),
        Line::from(vec![
            Span::styled("  fk-refs  ", Style::new().fg(colors.primary)),
            Span::styled(fk_str, Style::new().fg(colors.fg1)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(footer_lines)
            .block(Block::bordered().border_style(Style::new().fg(colors.bg2))),
        body_chunks[2],
    );
}

fn inspect_column_widths(fields: &[&crate::db::repo::sql_repo::TableField]) -> [u16; 5] {
    let mut widths = [
        "COLUMN".len(),
        "TYPE".len(),
        "NULL".len(),
        "CONSTRAINT".len(),
        "DEFAULT".len(),
    ];

    for field in fields {
        widths[0] = widths[0].max(field.name.chars().count());
        widths[1] = widths[1].max(field.data_type.chars().count());
        widths[2] = widths[2].max(field.is_nullable.chars().count());
        widths[3] = widths[3].max(constraint_text_len(&field.constraint));
        widths[4] = widths[4].max(
            field
                .default_value
                .as_deref()
                .map(|d| d.chars().count().min(DEFAULT_DISPLAY_CAP))
                .unwrap_or(1),
        );
    }

    widths.map(|w| w as u16)
}

fn constraint_text_len(constraint: &Option<ConstraintType>) -> usize {
    match constraint {
        Some(ConstraintType::PrimaryKey) => "PRIMARY KEY".len(),
        Some(ConstraintType::Unique) => "UNIQUE".len(),
        Some(ConstraintType::ForeignKey(t)) => "FK -> ".len() + t.chars().count(),
        None => 1,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::repo::sql_repo::TableField;

    #[test]
    fn inspect_widths_use_max_visible_text_not_fill_columns() {
        let fields = vec![
            TableField {
                name: "id".into(),
                data_type: "bigint".into(),
                is_nullable: "NO".into(),
                constraint: Some(ConstraintType::PrimaryKey),
                default_value: Some("nextval('bun_migrations_id_seq')".into()),
            },
            TableField {
                name: "migrated_at".into(),
                data_type: "timestamp with time zone".into(),
                is_nullable: "NO".into(),
                constraint: None,
                default_value: Some("CURRENT_TIMESTAMP".into()),
            },
        ];
        let refs: Vec<&TableField> = fields.iter().collect();

        assert_eq!(inspect_column_widths(&refs), [11, 24, 4, 11, 32]);
    }
}
