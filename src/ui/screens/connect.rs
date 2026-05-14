use crate::config::Connect;
use crate::state::app::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

/// Renders the connection list screen.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
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
                .title(" lazysql — Connections (j/k navigate, l/Enter connect, a add, q quit) "),
        )
        .highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let hint = state
        .connect
        .error
        .as_deref()
        .unwrap_or("No connections saved. Edit ~/.config/lazysql/config.toml to add one.");
    frame.render_widget(
        Paragraph::new(hint).block(Block::bordered().title(" Status ")),
        chunks[1],
    );
}
