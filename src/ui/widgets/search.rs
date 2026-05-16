use crate::state::app::AppState;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Style,
    widgets::{Block, Paragraph},
};

/// Renders the search bar at `area` and shows the cursor when active.
pub(crate) fn render_search_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let colors = &state.theme.colors;
    let query = &state.search.query;
    let (title, text) = if state.search.active {
        (" / ", format!("{query}_"))
    } else {
        (" Filter ", format!("/{query}"))
    };

    frame.render_widget(
        Paragraph::new(text.as_str()).block(
            Block::bordered()
                .title(title)
                .title_style(Style::new().fg(colors.blue).bold())
                .border_style(Style::new().fg(colors.bg2)),
        ),
        area,
    );

    if state.search.active {
        let cursor_x = (area.x + 1 + state.search.query.chars().count() as u16)
            .min(area.x + area.width.saturating_sub(2));
        frame.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}
