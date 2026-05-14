use crate::state::app::AppState;
use crate::state::connection::FIELD_LABELS;
use crate::ui::{theme, widgets};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

const HINTS: &[(&str, &str)] = &[
    ("tab", "next"),
    ("shift-tab", "prev"),
    ("ctrl+s", "save"),
    ("esc", "cancel"),
];

/// Renders the add-connection form.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let outer_chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    widgets::hintbar::render(frame, outer_chunks[0], HINTS);

    let title = if state.form.is_editing() {
        " Edit Connection "
    } else {
        " New Connection "
    };
    let outer = Block::bordered()
        .title(title)
        .border_style(Style::new().fg(theme::ORANGE));

    let inner_area = outer.inner(outer_chunks[1]);
    frame.render_widget(outer, outer_chunks[1]);

    let field_height = 3u16;
    let constraints: Vec<Constraint> = (0..FIELD_LABELS.len())
        .map(|_| Constraint::Length(field_height))
        .chain(std::iter::once(Constraint::Length(1)))
        .collect();

    let chunks = Layout::vertical(constraints).split(inner_area);

    for (i, label) in FIELD_LABELS.iter().enumerate() {
        let is_focused = i == state.form.focused;

        let display_value = if i == 5 {
            "*".repeat(state.form.values[i].len())
        } else {
            state.form.values[i].clone()
        };

        let block = if is_focused {
            Block::bordered()
                .title(format!(" {label} "))
                .border_style(Style::new().fg(theme::ORANGE))
        } else {
            Block::bordered()
                .title(format!(" {label} "))
                .border_style(Style::new().fg(theme::BG3))
        };

        let paragraph = Paragraph::new(display_value.as_str()).block(block);
        frame.render_widget(paragraph, chunks[i]);

        if is_focused && i != 5 {
            let inner = chunks[i];
            let cursor_x = inner.x + 1 + state.form.values[i].len() as u16;
            let cursor_y = inner.y + 1;
            let max_x = inner.x + inner.width.saturating_sub(2);
            frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
        }
    }

    if let Some(err) = &state.form.error {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ! ", Style::new().fg(theme::RED).bold()),
                Span::styled(err.as_str(), Style::new().fg(Color::Red)),
            ])),
            chunks[FIELD_LABELS.len()],
        );
    }

    widgets::statusbar::render(
        frame,
        outer_chunks[2],
        &state.mode,
        if state.form.is_editing() {
            "connection edit"
        } else {
            "connection new"
        },
        "tab:next  ctrl+s:save  esc:cancel",
    );
}
