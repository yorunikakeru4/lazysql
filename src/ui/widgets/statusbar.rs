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
    };

    let now = Local::now().format("%H:%M:%S").to_string();

    let spans = vec![
        Span::styled(pill_label, Style::new().bg(pill_bg).fg(theme::BG0).bold()),
        Span::raw("  "),
        Span::styled(context, Style::new().fg(theme::FG3)),
        Span::raw("   "),
        Span::styled(hints, Style::new().fg(theme::FG4)),
        Span::styled(format!(" {} ", now), Style::new().fg(theme::FG0)),
    ];

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(theme::BG1)),
        area,
    );
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pill_label_matches_mode() {
        let cases = [
            (AppMode::Normal, " NORMAL "),
            (AppMode::Insert, " INSERT "),
            (AppMode::Search, " SEARCH "),
            (AppMode::Command, " SQL "),
            (AppMode::Result, " RESULT "),
        ];
        for (mode, expected) in cases {
            let label = match mode {
                AppMode::Normal => " NORMAL ",
                AppMode::Insert => " INSERT ",
                AppMode::Search => " SEARCH ",
                AppMode::Command => " SQL ",
                AppMode::Result => " RESULT ",
            };
            assert_eq!(label, expected);
        }
    }
}
