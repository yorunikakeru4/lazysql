use crate::state::app::AppState;
use crate::state::sql_input::SqlResult;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
};

use super::super::layout::centered_rect;

/// Renders the SQL input bar at the bottom of the screen.
pub(crate) fn render_input_bar(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let bar_area = Rect {
        x: area.x,
        y: area.height.saturating_sub(3),
        width: area.width,
        height: 3,
    };

    frame.render_widget(Clear, bar_area);

    let display_query = state.sql_input.query.replace('\n', " ↵ ");
    let text = format!(":{}", display_query);
    let paragraph = Paragraph::new(text.as_str())
        .block(Block::bordered().title(" SQL (Shift+Enter newline, Esc cancel) "));
    frame.render_widget(paragraph, bar_area);

    let cursor_x = (bar_area.x + 1 + 1 + display_query.len() as u16)
        .min(bar_area.x + bar_area.width.saturating_sub(2));
    frame.set_cursor_position(Position::new(cursor_x, bar_area.y + 1));
}

/// Renders a centered popup showing the SQL execution result.
pub(crate) fn render_result_popup(frame: &mut Frame, state: &AppState) {
    let Some(result) = &state.sql_input.result else {
        return;
    };

    let (title, body, color) = match result {
        SqlResult::Success { rows_affected } => (
            " Success ",
            format!("Rows Affected: {}", rows_affected),
            Color::Green,
        ),
        SqlResult::Rows { count } => (
            " Success ",
            format!("Rows Returned: {}", count),
            Color::Green,
        ),
        SqlResult::Error(msg) => (" Error ", msg.clone(), Color::Red),
    };

    let popup_area = centered_rect(50, 5, frame.area());
    frame.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(format!("{}\n\nPress Enter to dismiss", body)).block(
        Block::bordered()
            .title(title)
            .style(Style::default().fg(color)),
    );
    frame.render_widget(paragraph, popup_area);
}
