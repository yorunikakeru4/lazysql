use crate::state::mode::AppMode;
use crate::ui::theme;
use chrono::Local;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

/// Renders the bottom status bar: mode pill · context · hints · clock.
pub(crate) fn render(frame: &mut Frame, area: Rect, mode: &AppMode, context: &str, hints: &str) {
    let (pill_label, pill_bg) = match mode {
        AppMode::Normal => (" NORMAL ", theme::AQUA),
        AppMode::Insert => (" INSERT ", theme::ORANGE),
        AppMode::Search => (" SEARCH ", theme::YELLOW),
        AppMode::Command => (" SQL ", theme::YELLOW),
        AppMode::Result => (" RESULT ", theme::BLUE),
        AppMode::Help => (" HELP ", theme::ORANGE),
    };

    let now = Local::now().format("%H:%M:%S").to_string();

    let mut spans = vec![
        Span::styled(pill_label, Style::new().bg(pill_bg).fg(theme::BG0).bold()),
        Span::raw("  "),
        Span::styled(context, Style::new().fg(theme::BLUE).bold()),
        Span::raw("   "),
    ];
    spans.extend(hint_spans(hints));
    spans.push(Span::styled(
        format!(" {now} "),
        Style::new().fg(theme::FG0),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)).style(Style::new()), area);
}

fn hint_spans(hints: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, hint) in hints
        .split("  ")
        .filter(|hint| !hint.is_empty())
        .enumerate()
    {
        if i > 0 {
            spans.push(Span::styled("  ", Style::new().fg(theme::FG4)));
        }
        let Some((key, action)) = hint.split_once(':') else {
            spans.push(Span::styled(hint.to_string(), Style::new().fg(theme::FG4)));
            continue;
        };
        spans.push(Span::styled(
            key.to_string(),
            Style::new().fg(theme::YELLOW),
        ));
        spans.push(Span::styled(":", Style::new().fg(theme::FG4)));
        spans.push(Span::styled(
            action.to_string(),
            Style::new().fg(theme::FG3),
        ));
    }
    spans
}

#[cfg(test)]
mod test {
    use super::*;
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
    fn renders_lazysql_context_and_colored_hint_keys() {
        let backend = TestBackend::new(90, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    &AppMode::Insert,
                    "lazysql — 0 connections",
                    "tab:next  shift-tab:back  ^s:save  ^t:test  esc:cancel",
                );
            })
            .unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("lazysql"));

        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .any(|cell| cell.symbol() == "t" && cell.fg == theme::YELLOW)
        );
    }
}
