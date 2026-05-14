use crate::state::navigation::Screen;
use crate::ui::{layout::centered_rect, theme};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

struct HelpSection {
    title: &'static str,
    entries: &'static [(&'static str, &'static str)],
}

fn sections_for(screen: &Screen) -> &'static [HelpSection] {
    match screen {
        Screen::Connect | Screen::AddConnection => &[HelpSection {
            title: "connections",
            entries: &[
                ("a", "add new connection"),
                ("e", "edit selected"),
                ("d", "delete selected"),
                ("↵", "connect"),
                ("/", "search"),
                ("q", "quit"),
            ],
        }],
        Screen::Database => &[
            HelpSection {
                title: "navigate",
                entries: &[
                    ("j/k", "move cursor"),
                    ("tab", "switch pane"),
                    ("h/Esc", "back"),
                    ("/", "filter"),
                ],
            },
            HelpSection {
                title: "table",
                entries: &[
                    ("↵/l", "open rows"),
                    ("i", "inspect schema"),
                    ("r", "view rows"),
                    (":", "SQL command"),
                ],
            },
        ],
        Screen::Inspect => &[HelpSection {
            title: "inspect",
            entries: &[
                ("r", "view rows"),
                ("s", "sample 100"),
                ("/", "filter columns"),
                ("q", "back"),
            ],
        }],
        Screen::Records => &[
            HelpSection {
                title: "navigate",
                entries: &[
                    ("j/k", "row down/up"),
                    ("h/l", "col left/right"),
                    ("gg", "first row"),
                    ("G", "last row"),
                ],
            },
            HelpSection {
                title: "actions",
                entries: &[
                    ("yy", "yank row"),
                    ("y", "yank cell"),
                    ("v", "visual select"),
                    ("q", "close"),
                ],
            },
        ],
    }
}

/// Renders the help overlay. Call when `state.help_visible` is true.
pub(crate) fn render(frame: &mut Frame, screen: &Screen) {
    let area = centered_rect(60, 24, frame.area());
    frame.render_widget(Clear, area);

    let sections = sections_for(screen);
    let mut lines: Vec<Line> = vec![Line::from("")];

    for section in sections {
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", section.title),
            Style::new().fg(theme::ORANGE).bold(),
        )]));
        for (key, action) in section.entries {
            lines.push(Line::from(vec![
                Span::styled(format!("    {:10}", key), Style::new().fg(theme::YELLOW)),
                Span::styled(*action, Style::new().fg(theme::FG3)),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(
        "  press ? or Esc to close",
        Style::new().fg(theme::FG4),
    )]));

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(" Help ")
                    .border_style(Style::new().fg(theme::AQUA)),
            )
            .style(Style::new().bg(theme::BG1)),
        area,
    );
}
