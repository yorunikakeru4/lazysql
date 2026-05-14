use crate::state::app::AppState;
use ratatui::{Frame, widgets::{Block, Paragraph}};

/// Renders the schema+table split view (stub — full impl in Task 12).
pub(crate) fn render(frame: &mut Frame, _state: &AppState) {
    frame.render_widget(
        Paragraph::new("Database view (coming soon)")
            .block(Block::bordered().title(" Database ")),
        frame.area(),
    );
}
