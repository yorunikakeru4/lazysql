use crate::state::app::AppState;
use crate::state::sql_input::SqlResult;
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
};

use super::super::layout::centered_rect;

/// Renders a centered popup showing the SQL execution result.
pub(crate) fn render_result_popup(frame: &mut Frame, state: &AppState) {
    let Some(result) = &state.sql_input.result else {
        return;
    };

    let (title, body, color) = match result {
        SqlResult::Success { rows_affected } => (
            " Success ",
            format!("Rows Affected: {rows_affected}"),
            Color::Green,
        ),
        SqlResult::Rows { count } => (" Success ", format!("Rows Returned: {count}"), Color::Green),
        SqlResult::Error(msg) => (" Error ", msg.clone(), Color::Red),
    };

    let popup_area = centered_rect(50, 5, frame.area());
    frame.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(format!("{body}\n\nPress Enter to dismiss")).block(
        Block::bordered()
            .title(title)
            .style(Style::default().fg(color)),
    );
    frame.render_widget(paragraph, popup_area);
}
