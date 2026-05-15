use crate::state::app::AppState;
use crate::state::connection::{ConnectionMeta, ConnectionStatus, DriverDefinition};
use crate::ui::{layout, theme, widgets};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Position, Rect},
    style::Color,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState},
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
    let show_search = state.search.active || !state.search.query.is_empty();
    if state.connect.form_open {
        let chunks = Layout::vertical(if show_search {
            vec![
                Constraint::Length(3),
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Fill(1),
                Constraint::Length(1),
            ]
        })
        .split(area);

        render_connections_header(frame, chunks[0]);
        let panes = Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(chunks[1]);
        render_connection_list(frame, panes[0], state);
        render_connection_form_panel(frame, panes[1], state);

        let status_idx = if show_search {
            widgets::search::render_search_bar(frame, chunks[2], state);
            3
        } else {
            2
        };
        let connection_count = state.connections_config.len();
        widgets::statusbar::render(
            frame,
            chunks[status_idx],
            &state.mode,
            &format!("lazysql — {connection_count} connections"),
            "tab:next  shift-tab:back  ^s:save  ^t:test  esc:cancel",
        );
        return;
    }

    let chunks = Layout::vertical(if show_search {
        vec![
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(5),
            Constraint::Length(1),
        ]
    })
    .split(area);

    render_connections_header(frame, chunks[0]);
    render_connection_list(frame, chunks[1], state);
    let details_idx = if show_search {
        widgets::search::render_search_bar(frame, chunks[2], state);
        3
    } else {
        2
    };
    render_details_panel(frame, chunks[details_idx], state);
    let connection_count = state.connections_config.len();
    widgets::statusbar::render(
        frame,
        chunks[details_idx + 1],
        &state.mode,
        &format!("lazysql — {connection_count} connections"),
        "j/k:move  /:search  a:add  ↵:connect",
    );

    if state.connect.driver_picker_open {
        render_driver_picker(frame, state);
    }
}

fn render_connections_header(frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" lazysql ", Style::new().fg(theme::BLUE).bold()),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::new().fg(theme::FG3)),
        Span::raw(" "),
    ]);
    let block = Block::bordered()
        .title(title)
        .title(
            Line::styled(" Connections ", Style::new().fg(theme::BLUE).bold())
                .alignment(Alignment::Right),
        )
        .border_style(Style::new().fg(theme::BLUE));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    widgets::hintbar::render(frame, inner, HINTS);
}

fn render_connection_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let filtered_indices = state.filtered_connection_indices();
    let metas: Vec<(usize, ConnectionMeta)> = filtered_indices
        .iter()
        .map(|i| (*i, ConnectionMeta::from(&state.connections_config[*i])))
        .collect();

    let header = Row::new(vec![
        Cell::from(" #").style(Style::new().fg(theme::FG4)),
        Cell::from("NAME").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("HOST").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("DRIVER").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("DATABASE").style(Style::new().fg(theme::FG4).bold()),
        Cell::from("STATUS").style(Style::new().fg(theme::FG4).bold()),
    ]);

    let rows: Vec<Row> = metas
        .iter()
        .enumerate()
        .map(|(visible_i, (index, m))| {
            let status_cell = render_status_cell(state.connection_status(*index));
            let row_number = visible_i + 1;
            let host = &m.host;
            let port = m.port;
            Row::new(vec![
                Cell::from(format!(" {row_number}")).style(Style::new().fg(theme::FG4)),
                Cell::from(m.name.clone()).style(Style::new().fg(theme::FG0)),
                Cell::from(format!("{host}:{port}")).style(Style::new().fg(theme::FG3)),
                Cell::from(m.driver.clone()).style(Style::new().fg(theme::BLUE)),
                Cell::from(m.db_name.clone()).style(Style::new().fg(theme::FG3)),
                status_cell,
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Length(12),
    ];

    let count_str = if state.connections_config.is_empty() {
        "0/0".to_string()
    } else if metas.is_empty() {
        let total = state.connections_config.len();
        format!("0/{total}")
    } else {
        let selected = state
            .selected_filtered_connection_position()
            .map(|i| i + 1)
            .unwrap_or(0);
        let total = metas.len();
        format!("{selected}/{total}")
    };

    let mut table_state =
        TableState::default().with_selected(state.selected_filtered_connection_position());
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .title(format!(" Saved Connections ─── {count_str} "))
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

fn render_connection_form_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = if state.form.is_editing() {
        format!(" Edit Connection · {} ", state.form.driver.label())
    } else {
        format!(" New Connection · {} ", state.form.driver.label())
    };
    let block = Block::bordered()
        .title(title)
        .title_style(Style::new().fg(theme::ORANGE).bold())
        .border_style(Style::new().fg(theme::ORANGE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let field_rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  driver             ", Style::new().fg(theme::FG4)),
            Span::styled(state.form.driver.label(), Style::new().fg(theme::BLUE)),
            Span::styled("  (^d to change)", Style::new().fg(theme::FG4)),
        ])),
        field_rows[0],
    );

    for (i, label) in crate::state::connection::FIELD_LABELS.iter().enumerate() {
        let value = if i == 5 {
            "*".repeat(state.form.values[i].chars().count())
        } else {
            state.form.values[i].clone()
        };
        let color = if i == state.form.focused {
            theme::ORANGE
        } else {
            theme::FG4
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {label:<19}"), Style::new().fg(color)),
                Span::styled(value, Style::new().fg(theme::FG0)),
            ])),
            field_rows[i + 1],
        );
    }

    let status = state
        .connect
        .draft_status
        .unwrap_or(ConnectionStatus::Unknown);
    let (color, label) = status_indicator(status);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  status             ", Style::new().fg(theme::FG4)),
            Span::styled(status_dot_symbol(), Style::new().fg(color)),
            Span::styled(label, Style::new().fg(color)),
        ])),
        field_rows[7],
    );

    if let Some(err) = &state.form.error {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ! ", Style::new().fg(theme::RED).bold()),
                Span::styled(err.as_str(), Style::new().fg(theme::RED)),
            ])),
            field_rows[8],
        );
    }

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("[ Test ]", Style::new().fg(theme::BLUE).bold()),
            Span::raw("   "),
            Span::styled("[ Save ]", Style::new().fg(theme::GREEN).bold()),
            Span::raw("   "),
            Span::styled("[ Cancel ]", Style::new().fg(theme::RED).bold()),
        ])),
        field_rows[9],
    );

    if state.form.focused != 5 {
        let focused_row = state.form.focused + 1;
        let cursor_x =
            field_rows[focused_row].x + 21 + state.form.values[state.form.focused].len() as u16;
        let max_x = field_rows[focused_row]
            .x
            .saturating_add(field_rows[focused_row].width.saturating_sub(1));
        frame.set_cursor_position(Position::new(
            cursor_x.min(max_x),
            field_rows[focused_row].y,
        ));
    }
}

fn render_driver_picker(frame: &mut Frame, state: &AppState) {
    let area = layout::centered_rect(48, 12, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(" Select driver ")
        .title_style(Style::new().fg(theme::ORANGE).bold())
        .border_style(Style::new().fg(theme::ORANGE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  › "),
            Span::styled(
                state.connect.driver_picker.query.as_str(),
                Style::new().fg(theme::FG0),
            ),
        ])),
        rows[0],
    );

    let filtered = state.connect.driver_picker.filtered_drivers();
    let driver_lines: Vec<Line> = filtered
        .iter()
        .enumerate()
        .map(|(index, driver)| render_driver_picker_row(index, driver, state))
        .collect();
    frame.render_widget(Paragraph::new(driver_lines), rows[2]);

    let total = crate::state::connection::DRIVER_REGISTRY.len();
    let visible = filtered.len();
    frame.render_widget(
        Paragraph::new(format!("  {visible} of {total} drivers · type to filter")),
        rows[3],
    );
    frame.render_widget(Paragraph::new("  ↵:select  esc:cancel"), rows[4]);

    let cursor_x = rows[0]
        .x
        .saturating_add(4 + state.connect.driver_picker.query.len() as u16);
    frame.set_cursor_position(Position::new(
        cursor_x.min(rows[0].x.saturating_add(rows[0].width.saturating_sub(1))),
        rows[0].y,
    ));
}

fn render_driver_picker_row(
    index: usize,
    driver: &DriverDefinition,
    state: &AppState,
) -> Line<'static> {
    let selected = index == state.connect.driver_picker.selected;
    let prefix = if selected { " ▶ " } else { "   " };
    let style = if selected {
        Style::new().fg(theme::ORANGE).bold()
    } else {
        Style::new().fg(theme::FG0)
    };

    Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(format!("{:<12}", driver.label), style),
        Span::styled(driver.summary, Style::new().fg(theme::FG4)),
    ])
}

/// Renders a centered error popup when a connection attempt failed.
pub(crate) fn render_connect_error_popup(frame: &mut Frame, state: &AppState) {
    let Some(err) = &state.connect.error else {
        return;
    };

    let popup_area = layout::centered_rect(60, 7, frame.area());
    frame.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(format!("{err}\n\nPress Enter or Esc to dismiss")).block(
        Block::bordered()
            .title(" Connection Error ")
            .style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(paragraph, popup_area);
}

fn render_details_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let metas: Vec<ConnectionMeta> = state
        .connections_config
        .iter()
        .map(ConnectionMeta::from)
        .collect();

    let selected = state
        .selected_filtered_connection_position()
        .and_then(|_| metas.get(state.connect.selected));

    let content: Vec<Line> = if let Some(m) = selected {
        let user = &m.user;
        let host = &m.host;
        vec![
            Line::from(vec![
                Span::styled("  driver  ", Style::new().fg(theme::FG4)),
                Span::styled(&m.driver, Style::new().fg(theme::BLUE)),
            ]),
            Line::from(vec![
                Span::styled("  user    ", Style::new().fg(theme::FG4)),
                Span::styled(format!("{user}@{host}"), Style::new().fg(theme::PURPLE)),
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
        .map(|m| {
            let name = &m.name;
            format!(" Details · {name} ")
        })
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
    fn inline_new_connection_panel_renders_next_to_saved_connections() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.form_open = true;
        state.form.values[1] = "127.0.0.1".to_string();

        terminal.draw(|frame| render(frame, &state)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("Saved Connections"));
        assert!(text.contains("New Connection"));
        assert!(text.contains("127.0.0.1"));
    }

    #[test]
    fn driver_picker_renders_filtered_drivers() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.open_driver_picker();
        state.connect.driver_picker.query = "my".to_string();

        terminal.draw(|frame| render(frame, &state)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("Select driver"));
        assert!(text.contains("mysql"));
        assert!(!text.contains("postgres      libpq"));
    }

    #[test]
    fn inline_connection_panel_renders_selected_driver() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.form_open = true;
        state.form = crate::state::connection::FormState::new_for_driver(
            crate::state::connection::DriverKind::MySql,
        );

        terminal.draw(|frame| render(frame, &state)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("New Connection · mysql"));
        assert!(text.contains("driver"));
        assert!(text.contains("mysql"));
    }

    #[test]
    fn inline_connection_panel_renders_draft_offline_status() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.form_open = true;
        state.connect.draft_status = Some(ConnectionStatus::Offline);

        terminal.draw(|frame| render(frame, &state)).unwrap();

        assert!(buffer_text(&terminal).contains("offline"));
    }

    #[test]
    fn inline_connection_panel_renders_unknown_status_by_default() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.form_open = true;

        terminal.draw(|frame| render(frame, &state)).unwrap();

        assert!(buffer_text(&terminal).contains("unknown"));
    }

    #[test]
    fn connections_header_uses_lazysql_title_and_search_hint() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState::new(vec![]);

        terminal.draw(|frame| render(frame, &state)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("lazysql"));
        assert!(text.contains("Connections"));
        assert!(text.contains("/:search"));
        assert!(!text.contains("r:refresh"));
        assert!(!text.contains("/:filter"));
    }

    #[test]
    fn inline_connection_actions_are_colored_by_action() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.form_open = true;

        terminal.draw(|frame| render(frame, &state)).unwrap();

        let buffer = terminal.backend().buffer();
        assert!(
            buffer
                .content()
                .iter()
                .any(|cell| cell.symbol() == "T" && cell.fg == theme::BLUE)
        );
        assert!(
            buffer
                .content()
                .iter()
                .any(|cell| cell.symbol() == "S" && cell.fg == theme::GREEN)
        );
        assert!(
            buffer
                .content()
                .iter()
                .any(|cell| cell.symbol() == "C" && cell.fg == theme::RED)
        );
    }

    #[test]
    fn inline_connection_panel_renders_validation_error() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new(vec![]);
        state.connect.form_open = true;
        state.form.error = Some("Port must be a number 1–65535".to_string());

        terminal.draw(|frame| render(frame, &state)).unwrap();

        assert!(buffer_text(&terminal).contains("Port must be a number 1–65535"));
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
