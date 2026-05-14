use crate::state::app::AppState;
use crate::state::connection::{ConnectionMeta, ConnectionStatus};
use crate::ui::{layout, theme, widgets};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Color,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState},
};

const HINTS: &[(&str, &str)] = &[
    ("a", "add"),
    ("r", "refresh"),
    ("↵", "connect"),
    ("e", "edit"),
    ("d", "delete"),
    ("/", "filter"),
    ("?", "help"),
    ("q", "quit"),
];

/// Renders the connection list screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_search = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_search {
        vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(5),
            Constraint::Length(1),
        ]
    })
    .split(area);

    widgets::hintbar::render(frame, chunks[0], HINTS);
    render_connection_list(frame, chunks[1], state);
    let details_idx = if show_search {
        widgets::search::render_search_bar(frame, chunks[2], state);
        3
    } else {
        2
    };
    render_details_panel(frame, chunks[details_idx], state);
    widgets::statusbar::render(
        frame,
        chunks[details_idx + 1],
        &state.mode,
        &format!("dbx — {} connections", state.connections.len()),
        "j/k:move  /:filter  a:add  ↵:connect",
    );
}

fn render_connection_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let filtered_indices = state.filtered_connection_indices();
    let metas: Vec<(usize, ConnectionMeta)> = filtered_indices
        .iter()
        .map(|i| (*i, ConnectionMeta::from(&state.connections[*i])))
        .collect();

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
        .map(|(visible_i, (index, m))| {
            let status_cell = render_status_cell(state.connection_status(*index));
            Row::new(vec![
                Cell::from(format!(" {}", visible_i + 1)).style(Style::new().fg(theme::FG4)),
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
    } else if metas.is_empty() {
        format!("0/{}", state.connections.len())
    } else {
        let selected = state
            .selected_filtered_connection_position()
            .map(|i| i + 1)
            .unwrap_or(0);
        format!("{}/{}", selected, metas.len())
    };

    let mut table_state =
        TableState::default().with_selected(state.selected_filtered_connection_position());
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .title(format!(" Saved Connections ─── {} ", count_str))
                .title_style(Style::new().fg(theme::BLUE).bold())
                .border_style(Style::new().fg(theme::BLUE)),
        )
        .row_highlight_style(selected_row_highlight_style())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_status_cell(status: ConnectionStatus) -> Cell<'static> {
    let (color, label) = status_indicator(status);
    Cell::from(Line::from(vec![
        Span::styled(status_dot_symbol(), Style::new().fg(color)),
        Span::styled(label, Style::new().fg(color)),
    ]))
}

fn status_indicator(status: ConnectionStatus) -> (Color, &'static str) {
    match status {
        ConnectionStatus::Unknown => (theme::FG4, " unknown"),
        ConnectionStatus::Online => (theme::GREEN, " online"),
        ConnectionStatus::Offline => (theme::RED, " offline"),
    }
}

fn status_dot_symbol() -> &'static str {
    "●"
}

fn selected_row_highlight_style() -> Style {
    Style::new().bg(theme::BG_SEL)
}

/// Renders a centered error popup when a connection attempt failed.
pub(crate) fn render_connect_error_popup(frame: &mut Frame, state: &AppState) {
    let Some(err) = &state.connect.error else {
        return;
    };

    let popup_area = layout::centered_rect(60, 7, frame.area());
    frame.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(format!("{}\n\nPress Enter or Esc to dismiss", err)).block(
        Block::bordered()
            .title(" Connection Error ")
            .style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(paragraph, popup_area);
}

fn render_details_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let metas: Vec<ConnectionMeta> = state.connections.iter().map(ConnectionMeta::from).collect();
    let selected = state
        .selected_filtered_connection_position()
        .and_then(|_| metas.get(state.connect.selected));

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
                .title_style(Style::new().fg(theme::BLUE).bold())
                .border_style(Style::new().fg(theme::BLUE)),
        ),
        area,
    );
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state::app::AppState;
    use ratatui::{Terminal, backend::TestBackend};

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn connect_error_popup_renders_error_message() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.error = Some("connection refused".to_string());

        terminal
            .draw(|frame| render_connect_error_popup(frame, &state))
            .unwrap();

        assert!(buffer_text(&terminal).contains("connection refused"));
    }

    #[test]
    fn connect_error_popup_renders_nothing_when_no_error() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState::new(vec![]);

        terminal
            .draw(|frame| render_connect_error_popup(frame, &state))
            .unwrap();

        assert!(!buffer_text(&terminal).contains("Error"));
    }

    #[test]
    fn status_indicator_uses_expected_labels_and_colors() {
        assert_eq!(
            status_indicator(ConnectionStatus::Unknown),
            (theme::FG4, " unknown")
        );
        assert_eq!(
            status_indicator(ConnectionStatus::Online),
            (theme::GREEN, " online")
        );
        assert_eq!(
            status_indicator(ConnectionStatus::Offline),
            (theme::RED, " offline")
        );
    }

    #[test]
    fn status_dot_uses_filled_circle_symbol() {
        assert_eq!(status_dot_symbol(), "●");
    }

    #[test]
    fn selected_row_highlight_does_not_override_foreground_color() {
        let style = selected_row_highlight_style();
        assert_eq!(style.bg, Some(theme::BG_SEL));
        assert_eq!(style.fg, None);
    }
}
