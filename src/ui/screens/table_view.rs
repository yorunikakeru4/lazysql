use crate::state::app::AppState;
use crate::ui::format::format_fields;
use crate::ui::widgets::search::render_search_bar;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

/// Renders the table metadata screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_bar = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_bar {
        vec![Constraint::Fill(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Fill(1)]
    })
    .split(area);

    let Some(table) = &state.loaded_table else {
        frame.render_widget(
            Paragraph::new("No table loaded.").block(Block::bordered()),
            chunks[0],
        );
        return;
    };

    let fields: Vec<&crate::db::repo::tables_repo::TableField> = table
        .fields
        .iter()
        .filter(|f| state.search.matches(&f.name))
        .collect();

    let content = format_fields(&fields);
    frame.render_widget(
        Paragraph::new(content).block(
            Block::bordered().title(format!(" Table: {} (/ search, h/Esc back) ", table.name)),
        ),
        chunks[0],
    );

    if show_bar {
        render_search_bar(frame, chunks[1], state);
    }
}
