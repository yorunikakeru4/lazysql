pub(crate) mod format;
pub(crate) mod layout;
pub(crate) mod screens;
pub(crate) mod widgets;

use crate::state::app::AppState;
use crate::state::navigation::{Router, Screen};
use ratatui::Frame;

/// Renders the current screen and active overlays.
pub fn render(frame: &mut Frame, state: &AppState, router: &Router) {
    match router.current() {
        Some(Screen::Connect) => screens::connect::render(frame, state),
        Some(Screen::AddConnection) => screens::add_connection::render(frame, state),
        Some(Screen::Schemas) => screens::schemas::render(frame, state),
        Some(Screen::Tables) => screens::tables::render(frame, state),
        Some(Screen::TableView) => screens::table_view::render(frame, state),
        Some(Screen::Records) => screens::records::render(frame, state),
        None => {}
    }

    if state.sql_input.active {
        widgets::sql::render_input_bar(frame, state);
    }

    if state.sql_input.has_result() {
        widgets::sql::render_result_popup(frame, state);
    }
}
