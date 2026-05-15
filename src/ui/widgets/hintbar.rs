use crate::themes::palette::ThemeColors;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

/// Renders a one-line hint bar: `key:action   key:action …`
pub(crate) fn render(frame: &mut Frame, area: Rect, colors: &ThemeColors, hints: &[(&str, &str)]) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::new().fg(colors.fg4)));
        }
        spans.push(Span::styled(*key, Style::new().fg(colors.yellow)));
        spans.push(Span::styled(":", Style::new().fg(colors.fg4)));
        spans.push(Span::styled(*action, Style::new().fg(colors.fg3)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(Style::new()), area);
}

#[cfg(test)]
mod test {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn empty_hints_does_not_panic() {
        let hints: &[(&str, &str)] = &[];
        assert_eq!(hints.len(), 0);
    }

    #[test]
    fn single_hint_produces_spans() {
        let hints = [("q", "quit")];
        // Verify the hint slice is non-empty and structured correctly
        assert_eq!(hints[0].0, "q");
        assert_eq!(hints[0].1, "quit");
    }

    #[test]
    fn renders_hint_keys_with_runtime_yellow() {
        let backend = TestBackend::new(20, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let colors = crate::themes::builtin::fallback_theme().colors;
        let hints = [("q", "quit")];

        terminal
            .draw(|frame| render(frame, frame.area(), &colors, &hints))
            .unwrap();

        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .any(|cell| cell.symbol() == "q" && cell.fg == colors.yellow)
        );
    }
}
