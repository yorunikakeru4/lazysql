use ratatui::layout::{Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::{
    Frame,
    layout::Constraint,
    prelude::Line,
    style::Style,
    text::Span,
    widgets::{Block, Cell, Clear, Row, Table},
};

use crate::themes::palette::ThemeColors;

pub struct PickerItem<'a> {
    /// The main label of the item, displayed prominently.
    pub label: &'a str,

    /// Optional metadata for the item, displayed in a secondary style.
    pub meta: Option<&'a str>,
}

pub struct PickerView<'a> {
    pub colors: ThemeColors,
    pub empty_message: &'a str,
    pub footer: Line<'a>,
    pub items: &'a [PickerItem<'a>],
    pub query: &'a str,
    pub selected: usize,
    pub title: &'a str,
}

impl<'a> PickerView<'a> {
    /// Renders the picker view within the given area of the frame.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(format!(" {} ", self.title))
            .title_style(Style::new().fg(self.colors.blue).bold())
            .border_style(Style::new().fg(self.colors.blue))
            .style(Style::new().fg(self.colors.fg0));

        let inner = block.inner(area);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        let rows = self.visible_rows(chunks[1].height as usize);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" filter ", Style::new().fg(self.colors.fg2)),
                Span::styled(
                    if self.query.is_empty() {
                        "type to filter"
                    } else {
                        self.query
                    },
                    Style::new().fg(self.colors.fg0),
                ),
            ]))
            .block(Block::bordered().border_style(Style::new().fg(self.colors.bg2))),
            chunks[0],
        );

        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Fill(2),
                    Constraint::Fill(1),
                ],
            )
            .row_highlight_style(Style::new().bg(self.colors.bg_sel)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(self.footer.clone()).style(Style::new().fg(self.colors.fg1)),
            chunks[2],
        );
    }

    /// Computes the rows to display based on the current selection and available height.
    fn visible_rows(&self, height: usize) -> Vec<Row<'a>> {
        if self.items.is_empty() {
            return vec![Row::new(vec![
                Cell::from(""),
                Cell::from(self.empty_message).style(Style::new().fg(self.colors.fg2)),
                Cell::from(""),
            ])];
        }

        let visible_height = height.max(1);
        let selected = self.selected.min(self.items.len() - 1);

        let start = selected.saturating_add(1).saturating_sub(visible_height);
        let end = (start + visible_height).min(self.items.len());

        self.items[start..end]
            .iter()
            .enumerate()
            .map(|(local_index, item)| {
                let global_index = start + local_index;
                let is_selected = global_index == selected;

                let marker = if is_selected { "▶" } else { " " };

                let label_style = if is_selected {
                    Style::new().fg(self.colors.yellow).bold()
                } else {
                    Style::new().fg(self.colors.fg0)
                };

                Row::new(vec![
                    Cell::from(marker).style(Style::new().fg(self.colors.yellow)),
                    Cell::from(item.label).style(label_style),
                    Cell::from(item.meta.unwrap_or("")).style(Style::new().fg(self.colors.fg2)),
                ])
            })
            .collect()
    }
}
