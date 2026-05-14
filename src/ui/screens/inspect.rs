use crate::state::app::AppState;
use ratatui::{Frame, widgets::{Block, Paragraph}};

/// Renders the table inspect screen (stub — full impl in Task 13).
pub(crate) fn render(frame: &mut Frame, _state: &AppState) {
    frame.render_widget(
        Paragraph::new("Inspect view (coming soon)")
            .block(Block::bordered().title(" Inspect ")),
        frame.area(),
    );
}
