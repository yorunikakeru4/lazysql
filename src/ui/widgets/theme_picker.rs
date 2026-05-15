use crate::state::app::AppState;
use crate::ui::layout;
use ratatui::{
    Frame,
    layout::Constraint,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Row, Table},
};

/// Renders the centered theme picker overlay when it is open.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    if !state.theme_picker.open {
        return;
    }

    let colors = state.theme.colors;
    let popup_area = layout::centered_rect(50, 12, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Select theme ")
        .title_style(Style::new().fg(colors.blue).bold())
        .border_style(Style::new().fg(colors.blue))
        .style(Style::new().fg(colors.fg0).bg(colors.bg0));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let filtered = state.theme_picker.filtered_names();
    let rows = theme_rows(&filtered, state.theme_picker.selected, colors);
    let footer = footer_line(state, filtered.len());
    let constraints = [
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ];
    let chunks = ratatui::layout::Layout::vertical(constraints).split(inner);

    let query = if state.theme_picker.query.is_empty() {
        "type to filter".to_string()
    } else {
        state.theme_picker.query.clone()
    };
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(vec![
            Span::styled(" filter ", Style::new().fg(colors.fg4)),
            Span::styled(query, Style::new().fg(colors.fg0)),
        ]))
        .block(Block::bordered().border_style(Style::new().fg(colors.bg3))),
        chunks[0],
    );

    frame.render_widget(
        Table::new(rows, [Constraint::Length(3), Constraint::Fill(1)])
            .row_highlight_style(Style::new().bg(colors.bg_sel)),
        chunks[1],
    );
    frame.render_widget(
        ratatui::widgets::Paragraph::new(footer).style(Style::new().fg(colors.fg3)),
        chunks[2],
    );
}

fn theme_rows<'a>(
    names: &'a [&'a str],
    selected: usize,
    colors: crate::themes::palette::ThemeColors,
) -> Vec<Row<'a>> {
    if names.is_empty() {
        return vec![Row::new(vec![
            Cell::from(""),
            Cell::from("No themes found").style(Style::new().fg(colors.fg4)),
        ])];
    }

    names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let marker = if index == selected { ">" } else { " " };
            let style = if index == selected {
                Style::new().fg(colors.yellow).bold()
            } else {
                Style::new().fg(colors.fg0)
            };
            Row::new(vec![
                Cell::from(marker).style(Style::new().fg(colors.yellow)),
                Cell::from(*name).style(style),
            ])
        })
        .collect()
}

fn footer_line(state: &AppState, filtered_count: usize) -> Line<'_> {
    let colors = state.theme.colors;
    if let Some(error) = &state.theme_error {
        return Line::from(vec![
            Span::styled(" error: ", Style::new().fg(colors.red).bold()),
            Span::styled(error.as_str(), Style::new().fg(colors.red)),
            Span::raw("  "),
            Span::styled("enter:select", Style::new().fg(colors.yellow)),
            Span::raw("  "),
            Span::styled("esc:cancel", Style::new().fg(colors.fg3)),
        ]);
    }

    Line::from(vec![
        Span::styled(
            format!(" {filtered_count} themes  "),
            Style::new().fg(colors.fg4),
        ),
        Span::styled("select", Style::new().fg(colors.yellow)),
        Span::raw("  "),
        Span::styled("cancel", Style::new().fg(colors.fg3)),
        Span::raw("  "),
        Span::styled(
            "type to filter enter:select esc:cancel",
            Style::new().fg(colors.fg3),
        ),
    ])
}

#[cfg(test)]
mod test {
    use crate::state::app::AppState;
    
    use ratatui::{Terminal, backend::TestBackend};

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_theme_picker_overlay() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = crate::themes::builtin::fallback_theme();
        let mut state = AppState::new(vec![], theme.clone(), vec![theme]);
        state.theme_picker.open();

        terminal.draw(|frame| super::render(frame, &state)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("Select theme"));
        assert!(text.contains("gruvbox"));
        assert!(text.contains("select"));
        assert!(text.contains("cancel"));
    }
}
