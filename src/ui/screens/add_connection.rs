use crate::state::app::AppState;
use crate::state::connection::FIELD_LABELS;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph},
};

/// Renders the add-connection form.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let outer = Block::bordered()
        .title(" Add Connection (j/k or Tab navigate fields, Enter save, Esc cancel) ");

    let inner_area = outer.inner(area);
    frame.render_widget(outer, area);

    let field_height = 3u16;
    let constraints: Vec<Constraint> = (0..FIELD_LABELS.len())
        .map(|_| Constraint::Length(field_height))
        .chain(std::iter::once(Constraint::Length(1)))
        .collect();

    let chunks = Layout::vertical(constraints).split(inner_area);

    for (i, label) in FIELD_LABELS.iter().enumerate() {
        let is_focused = i == state.form.focused;

        let display_value = if i == 4 {
            "*".repeat(state.form.values[i].len())
        } else {
            state.form.values[i].clone()
        };

        let block = if is_focused {
            Block::bordered().title(format!(" {label} ")).bold()
        } else {
            Block::bordered().title(format!(" {label} "))
        };

        let paragraph = Paragraph::new(display_value.as_str()).block(block);
        frame.render_widget(paragraph, chunks[i]);

        if is_focused && i != 4 {
            let inner = chunks[i];
            let cursor_x = inner.x + 1 + state.form.values[i].len() as u16;
            let cursor_y = inner.y + 1;
            let max_x = inner.x + inner.width.saturating_sub(2);
            frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
        }
    }

    if let Some(err) = &state.form.error {
        frame.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            chunks[FIELD_LABELS.len()],
        );
    }
}
