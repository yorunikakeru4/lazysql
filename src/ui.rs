use crate::config::Connect;
use crate::db::repo::tables_repo::Table;
use crate::state::app_state::AppState;
use crate::state::router::{Router, Screen};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, state: &AppState, router: &Router) {
    match router.current() {
        Some(Screen::Connect) => render_connect(frame, state),
        Some(Screen::Schemas) => render_schemas(frame, state),
        Some(Screen::Tables) => render_tables(frame, state),
        Some(Screen::TableView) => render_table_view(frame, state),
        None => {}
    }
}

fn render_connect(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(area);

    let items: Vec<ListItem> = state
        .connections
        .iter()
        .map(|c| {
            let label = match c {
                Connect::Postgres(cfg) => {
                    format!("{}@{}:{}/{}", cfg.user, cfg.host, cfg.port, cfg.db_name)
                }
            };
            ListItem::new(label)
        })
        .collect();

    let mut list_state = ListState::default().with_selected(Some(state.connect.selected));
    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" lazy-sql — Connections (↑↓ navigate, Enter connect, q quit) "),
        )
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let hint = state
        .connect
        .error
        .as_deref()
        .unwrap_or("No connections saved. Edit ~/.config/lazy-sql/config.toml to add one.");
    frame.render_widget(
        Paragraph::new(hint).block(Block::bordered().title(" Status ")),
        chunks[1],
    );
}

fn render_schemas(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let names = state.schema_names();

    let items: Vec<ListItem> = names.iter().map(|n| ListItem::new(n.as_str())).collect();
    let mut list_state = ListState::default().with_selected(Some(state.schema_selected));
    let list = List::new(items)
        .block(Block::bordered().title(" Schemas (Enter select, Esc back) "))
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_tables(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let schema = state.selected_schema_name().unwrap_or_default();
    let names = state.table_names_in_schema(&schema);

    let items: Vec<ListItem> = names.iter().map(|n| ListItem::new(n.as_str())).collect();
    let mut list_state = ListState::default().with_selected(Some(state.table_selected));
    let list = List::new(items)
        .block(Block::bordered().title(format!(
            " Tables in schema '{schema}' (Enter view, Esc back) "
        )))
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_table_view(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let Some(table) = &state.loaded_table else {
        frame.render_widget(
            Paragraph::new("No table loaded.").block(Block::bordered()),
            area,
        );
        return;
    };

    let content = format_table(table);
    frame.render_widget(
        Paragraph::new(content)
            .block(Block::bordered().title(format!(" Table: {} (Esc back) ", table.name))),
        area,
    );
}

fn format_table(table: &Table) -> String {
    let header = format!(
        "{:<30} {:<20} {:<10} {:<20} {}\n{}\n",
        "Column",
        "Type",
        "Nullable",
        "Constraint",
        "Default",
        "-".repeat(90)
    );
    let rows: String = table
        .fields
        .iter()
        .map(|f| {
            let constraint = f
                .constraint_type
                .as_ref()
                .map(|c| format!("{c:?}"))
                .unwrap_or_default();
            let default = f.default.clone().unwrap_or_default();
            format!(
                "{:<30} {:<20} {:<10} {:<20} {}\n",
                f.name, f.data_type, f.is_nullable, constraint, default
            )
        })
        .collect();
    format!("{header}{rows}")
}
