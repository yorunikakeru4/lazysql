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
        Screen::Connect => &[HelpSection {
            title: "connections",
            entries: &[
                ("j/k", "navigate"),
                ("/", "search"),
                ("↵/l", "connect"),
                ("a", "add new"),
                ("e", "edit selected"),
                ("d", "delete selected"),
                ("r", "refresh statuses"),
                ("q", "quit"),
            ],
        }],
        Screen::AddConnection => &[HelpSection {
            title: "form",
            entries: &[
                ("Tab/↓", "next field"),
                ("BackTab/↑", "prev field"),
                ("↵ / Ctrl+S", "save"),
                ("Esc", "cancel"),
            ],
        }],
        Screen::Database => &[
            HelpSection {
                title: "navigate",
                entries: &[
                    ("j/k", "move cursor"),
                    ("tab", "switch pane"),
                    ("h/q/Esc", "back"),
                    ("/", "filter"),
                ],
            },
            HelpSection {
                title: "table",
                entries: &[
                    ("↵/l", "open / inspect"),
                    ("r", "view rows"),
                    (":/c", "SQL editor"),
                ],
            },
            HelpSection {
                title: "SQL editor",
                entries: &[("Ctrl+E/↵", "run query"), ("Esc", "close editor")],
            },
        ],
        Screen::Inspect => &[HelpSection {
            title: "inspect",
            entries: &[
                ("r/s", "view rows"),
                ("/", "filter columns"),
                ("q/Esc", "back"),
            ],
        }],
        Screen::Records => &[
            HelpSection {
                title: "navigate",
                entries: &[
                    ("j/k", "row / field"),
                    ("h/l", "field / row"),
                    ("n/p", "next/prev page"),
                ],
            },
            HelpSection {
                title: "actions",
                entries: &[("q/Esc", "close")],
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
        let title = section.title;
        lines.push(Line::from(vec![Span::styled(
            format!("  {title}"),
            Style::new().fg(theme::ORANGE).bold(),
        )]));
        for (key, action) in section.entries {
            lines.push(Line::from(vec![
                Span::styled(format!("    {key:10}"), Style::new().fg(theme::YELLOW)),
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
