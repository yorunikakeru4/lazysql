use crate::state::app::AppState;
use crate::ui::widgets::search::render_search_bar;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, List, ListItem, ListState},
};

/// Renders the table list screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_bar = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_bar {
        vec![Constraint::Fill(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Fill(1)]
    })
    .split(area);

    let schema = state.selected_schema_name().unwrap_or_default();
    let names = state.filtered_table_names(&schema);

    let items: Vec<ListItem> = names.iter().map(|n| ListItem::new(n.as_str())).collect();
    let mut list_state = ListState::default().with_selected(Some(state.table_selected));
    let list = List::new(items)
        .block(Block::bordered().title(format!(
            " Tables in '{schema}' (j/k navigate, l/Enter view, / search, h/Esc back) "
        )))
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    if show_bar {
        render_search_bar(frame, chunks[1], state);
    }
}
