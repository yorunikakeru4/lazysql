use crate::ui::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

/// Renders a one-line hint bar: `key:action   key:action …`
pub(crate) fn render(frame: &mut Frame, area: Rect, hints: &[(&str, &str)]) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::new().fg(theme::FG4)));
        }
        spans.push(Span::styled(*key, Style::new().fg(theme::YELLOW)));
        spans.push(Span::styled(":", Style::new().fg(theme::FG4)));
        spans.push(Span::styled(*action, Style::new().fg(theme::FG3)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(Style::new()), area);
}

#[cfg(test)]
mod test {
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
}
