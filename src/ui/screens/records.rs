use crate::state::app::AppState;
use crate::state::records::RecordsSource;
use crate::ui::format::format_records;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

/// Renders the record paging screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(area);

    let records = &state.records;
    let title = match &records.source {
        Some(RecordsSource::Table { table, .. }) => {
            format!(" Records: {} ", table)
        }
        Some(RecordsSource::Query { .. }) => " Query Results ".to_string(),
        None => " Records ".to_string(),
    };

    let content = format_records(records, chunks[0].width);

    frame.render_widget(
        Paragraph::new(content)
            .block(Block::bordered().title(format!("{} (h/← prev, l/→ next, Esc back)", title))),
        chunks[0],
    );

    let start_row = records.offset + 1;
    let end_row = (records.offset + records.rows.len() as u64).min(records.total_count);
    let status = format!(
        "Page {}/{} | Rows {}-{} of {}",
        records.current_page(),
        records.total_pages(),
        start_row,
        end_row,
        records.total_count
    );
    frame.render_widget(
        Paragraph::new(status).block(Block::bordered().title(" Status ")),
        chunks[1],
    );
}
