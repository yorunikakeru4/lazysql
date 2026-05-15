pub(crate) mod layout;
pub(crate) mod screens;
pub(crate) mod theme;
pub(crate) mod widgets;

use crate::state;
use crate::state::app::AppState;
use crate::state::navigation::{Router, Screen};
use ratatui::Frame;

pub fn render(frame: &mut Frame, state: &AppState, router: &Router) {
    match router.current() {
        Some(Screen::Connect) => screens::connect::render(frame, state),
        Some(Screen::Database) => screens::database::render(frame, state),
        Some(Screen::Inspect) => screens::inspect::render(frame, state),
        Some(Screen::Records) => screens::records::render(frame, state),
        None => {}
    }

    if matches!(router.current(), Some(Screen::Connect)) {
        screens::connect::render_connect_error_popup(frame, state);
    }

    if state.sql_input.active {
        widgets::sql_editor::render(frame, state);
    }
    if state.sql_input.has_result() {
        widgets::sql::render_result_popup(frame, state);
    }

    if state.mode == state::mode::AppMode::Help
        && let Some(screen) = router.current()
    {
        widgets::help::render(frame, screen);
    }
}
