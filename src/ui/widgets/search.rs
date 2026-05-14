use crate::state::app::AppState;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    widgets::{Block, Paragraph},
};

/// Renders the search bar at `area` and shows the cursor when active.
pub(crate) fn render_search_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let (title, text) = if state.search.active {
        (" / ", format!("{}_", state.search.query))
    } else {
        (" Filter ", format!("/{}", state.search.query))
    };

    frame.render_widget(
        Paragraph::new(text.as_str()).block(Block::bordered().title(title)),
        area,
    );

    if state.search.active {
        let cursor_x = (area.x + 1 + state.search.query.chars().count() as u16)
            .min(area.x + area.width.saturating_sub(2));
        frame.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}
