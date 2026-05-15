use crate::state::app::AppState;
use crate::state::connection::ActivePane;
use crate::ui::{theme, widgets};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
};

const HINTS: &[(&str, &str)] = &[
    ("↵", "open"),
    ("i", "inspect"),
    ("r", "rows"),
    ("/", "filter"),
    (":", "sql"),
    ("tab", "switch pane"),
    ("?", "help"),
    ("q", "back"),
];

/// Renders the split schema+table view.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let show_search = state.search.active || !state.search.query.is_empty();
    let chunks = Layout::vertical(if show_search {
        vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]
    })
    .split(area);

    let context = state
        .selected_schema_name()
        .map(|s| format!("database › {s}"))
        .unwrap_or_else(|| "database".into());

    widgets::hintbar::render(frame, chunks[0], HINTS);
    render_split(frame, chunks[1], state);
    let status_idx = if show_search {
        widgets::search::render_search_bar(frame, chunks[2], state);
        3
    } else {
        2
    };
    widgets::statusbar::render(
        frame,
        chunks[status_idx],
        &state.mode,
        &context,
        "tab:switch  /:filter  ::sql",
    );
}

fn render_split(frame: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::horizontal([Constraint::Length(20), Constraint::Fill(1)]).split(area);
    render_schemas_pane(frame, cols[0], state);
    render_tables_pane(frame, cols[1], state);
}

fn render_schemas_pane(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_pane == ActivePane::Schemas;
    let schemas = state.filtered_schema_names();

    let items: Vec<ListItem> = schemas
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if is_active && i == state.schema_selected {
                ListItem::new(Line::from(vec![
                    Span::styled("▶ ", Style::new().fg(theme::ORANGE)),
                    Span::styled(name.as_str(), Style::new().fg(theme::FG0).bold()),
                ]))
            } else {
                ListItem::new(Line::from(Span::styled(
                    format!("  {name}"),
                    Style::new().fg(if is_active { theme::FG3 } else { theme::FG4 }),
                )))
            }
        })
        .collect();

    let border_style = if is_active {
        Style::new().fg(theme::ORANGE)
    } else {
        Style::new().fg(theme::BG3)
    };

    let mut list_state = ListState::default().with_selected(if is_active {
        Some(state.schema_selected)
    } else {
        None
    });

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" Schemas ")
                .border_style(border_style),
        )
        .highlight_style(Style::new().bg(theme::BG_SEL));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_tables_pane(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_pane == ActivePane::Tables;
    let schema = state.selected_schema_name().unwrap_or_default();
    let tables = state.filtered_table_names(&schema);

    let items: Vec<ListItem> = tables
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if is_active && i == state.table_selected {
                ListItem::new(Line::from(vec![
                    Span::styled("▶ ", Style::new().fg(theme::ORANGE)),
                    Span::styled(name.as_str(), Style::new().fg(theme::FG0).bold()),
                ]))
            } else {
                ListItem::new(Line::from(Span::styled(
                    format!("  {name}"),
                    Style::new().fg(if is_active { theme::FG3 } else { theme::FG4 }),
                )))
            }
        })
        .collect();

    let border_style = if is_active {
        Style::new().fg(theme::ORANGE)
    } else {
        Style::new().fg(theme::BG3)
    };

    let title = format!(
        " Tables · {} ─── {}/{} ",
        schema,
        if tables.is_empty() {
            0
        } else {
            state.table_selected + 1
        },
        tables.len()
    );

    let mut list_state = ListState::default().with_selected(if is_active {
        Some(state.table_selected)
    } else {
        None
    });

    let list = List::new(items)
        .block(Block::bordered().title(title).border_style(border_style))
        .highlight_style(Style::new().bg(theme::BG_SEL));

    frame.render_stateful_widget(list, area, &mut list_state);
}
