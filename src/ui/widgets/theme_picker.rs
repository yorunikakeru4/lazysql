use crate::{
    state::app::AppState,
    themes::palette::Theme,
    ui::{
        layout,
        widgets::components::picker::{PickerItem, PickerView},
    },
};

use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
};

/// Renders the theme picker popup if it's open.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    if !state.theme_picker.open {
        return;
    }

    let colors = state.theme.colors;
    let popup_area = layout::centered_rect(50, 12, frame.area());

    let filtered = filtered_theme_names(&state.available_themes, &state.theme_picker.query);
    let items: Vec<PickerItem<'_>> = filtered
        .iter()
        .map(|name| PickerItem {
            label: name,
            meta: None,
        })
        .collect();

    PickerView {
        title: "Select theme",
        query: &state.theme_picker.query,
        selected: state.theme_picker.selected,
        items: &items,
        colors,
        empty_message: "No themes found",
        footer: footer_line(state, filtered.len()),
    }
    .render(frame, popup_area);
}

/// Generates the footer line for the theme picker, showing either an error message or the count of filtered themes and instructions.
fn footer_line(state: &AppState, filtered_count: usize) -> Line<'_> {
    let colors = state.theme.colors;

    if let Some(error) = &state.theme_error {
        return Line::from(vec![
            Span::styled(" error: ", Style::new().fg(colors.red).bold()),
            Span::styled(error.as_str(), Style::new().fg(colors.red)),
            Span::raw("  "),
            Span::styled("enter:select", Style::new().fg(colors.yellow)),
            Span::raw("  "),
            Span::styled("esc:cancel", Style::new().fg(colors.fg1)),
        ]);
    }

    Line::from(vec![
        Span::styled(
            format!(" {filtered_count} themes  "),
            Style::new().fg(colors.fg2),
        ),
        Span::styled("enter:select", Style::new().fg(colors.yellow)),
        Span::raw("  "),
        Span::styled("esc:cancel", Style::new().fg(colors.fg1)),
        Span::raw("  "),
        Span::styled("type to filter", Style::new().fg(colors.fg1)),
    ])
}

/// Filters the list of themes based on the query string, returning the names of matching themes.
pub fn filtered_theme_names<'a>(themes: &'a [Theme], query: &str) -> Vec<&'a str> {
    themes
        .iter()
        .map(|theme| theme.name.as_str())
        .filter(|name| name.contains(query))
        .collect()
}
