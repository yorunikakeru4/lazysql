use crate::state::app::AppState;
use crate::state::connection::{ConnectionMeta, HealthStatus};
use crate::ui::{theme, widgets};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};

const HINTS: &[(&str, &str)] = &[
    ("a", "add"),
    ("↵", "connect"),
    ("e", "edit"),
    ("d", "delete"),
    ("/", "search"),
    ("?", "help"),
    ("q", "quit"),
];

/// Renders the connection list screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .split(area);

    widgets::hintbar::render(frame, chunks[0], HINTS);
    render_connection_list(frame, chunks[1], state);
    render_details_panel(frame, chunks[2], state);
    widgets::statusbar::render(
        frame,
        chunks[3],
        &state.mode,
        &format!("dbx — {} connections", state.connections.len()),
        "j/k:move  a:add  ↵:connect",
    );
}

fn render_connection_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let metas: Vec<ConnectionMeta> = state.connections.iter().map(ConnectionMeta::from).collect();
    let health = &state.connect.health;

    let header = Row::new(vec![
        Cell::from(" #").style(Style::new().fg(theme::FG4)),
        Cell::from("NAME").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("HOST").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("DATABASE").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("STATUS").style(Style::new().fg(theme::FG4).bold()),
    ]);

    let rows: Vec<Row> = metas
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let status = health.get(i).cloned().unwrap_or(HealthStatus::Unknown);
            let (dot, status_str, status_color) = match status {
                HealthStatus::Live => ("●", "live", theme::GREEN),
                HealthStatus::Idle => ("○", "idle", theme::YELLOW),
                HealthStatus::Offline => ("×", "offline", theme::RED),
                HealthStatus::Unknown => ("·", "—", theme::FG4),
            };
            let status_cell = Cell::from(Line::from(vec![
                Span::styled(dot, Style::new().fg(status_color)),
                Span::styled(format!(" {}", status_str), Style::new().fg(status_color)),
            ]));
            Row::new(vec![
                Cell::from(format!(" {}", i + 1)).style(Style::new().fg(theme::FG4)),
                Cell::from(m.name.clone()).style(Style::new().fg(theme::FG0)),
                Cell::from(format!("{}:{}", m.host, m.port)).style(Style::new().fg(theme::FG3)),
                Cell::from(m.db_name.clone()).style(Style::new().fg(theme::FG3)),
                status_cell,
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Fill(2),
        Constraint::Fill(3),
        Constraint::Fill(2),
        Constraint::Length(12),
    ];

    let count_str = if state.connections.is_empty() {
        "0/0".to_string()
    } else {
        format!("{}/{}", state.connect.selected + 1, state.connections.len())
    };

    let mut table_state = TableState::default().with_selected(Some(state.connect.selected));
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .title(format!(" Saved Connections ─── {} ", count_str))
                .border_style(Style::new().fg(theme::BG3)),
        )
        .row_highlight_style(Style::new().bg(theme::BG_SEL).fg(theme::FG0))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_details_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let metas: Vec<ConnectionMeta> = state.connections.iter().map(ConnectionMeta::from).collect();
    let selected = metas.get(state.connect.selected);

    let content: Vec<Line> = if let Some(m) = selected {
        vec![
            Line::from(vec![
                Span::styled("  driver  ", Style::new().fg(theme::FG4)),
                Span::styled(&m.driver, Style::new().fg(theme::BLUE)),
                Span::styled("   ·   ssl  ", Style::new().fg(theme::FG4)),
                Span::styled("—", Style::new().fg(theme::FG3)),
            ]),
            Line::from(vec![
                Span::styled("  user    ", Style::new().fg(theme::FG4)),
                Span::styled(
                    format!("{}@{}", m.user, m.host),
                    Style::new().fg(theme::PURPLE),
                ),
                Span::styled("   ·   port  ", Style::new().fg(theme::FG4)),
                Span::styled(m.port.to_string(), Style::new().fg(theme::FG0)),
            ]),
            Line::from(vec![
                Span::styled("  db      ", Style::new().fg(theme::FG4)),
                Span::styled(&m.db_name, Style::new().fg(theme::FG0)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "  No connection selected",
            Style::new().fg(theme::FG4),
        ))]
    };

    let title = selected
        .map(|m| format!(" Details · {} ", m.name))
        .unwrap_or_else(|| " Details ".into());

    frame.render_widget(
        Paragraph::new(content).block(
            Block::bordered()
                .title(title)
                .border_style(Style::new().fg(theme::BG3)),
        ),
        area,
    );
}
