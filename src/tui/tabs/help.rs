use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::tui::TuiApp;

impl TuiApp {
    pub(crate) fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Claudelytics Enhanced TUI",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " - Claude Code Usage Analytics",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "\u{1f504} Enhanced Navigation:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  1-6/h, Tab/Shift+Tab", Style::default().fg(Color::Green)),
                Span::styled("  Switch between tabs", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  \u{2191}/\u{2193}, j/k",
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    "          Navigate up/down",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  gg/G", Style::default().fg(Color::Green)),
                Span::styled(
                    "              Jump to top/bottom",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+d/Ctrl+u", Style::default().fg(Color::Green)),
                Span::styled(
                    "       Half page down/up",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  0/$", Style::default().fg(Color::Green)),
                Span::styled(
                    "               Beginning/end of line",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Page Up/Down", Style::default().fg(Color::Green)),
                Span::styled(
                    "        Fast scroll (10 items)",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  v", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Visual mode (multi-select)",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "\u{1f50d} Search & Filter:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  /", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Enter search mode",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("    w/b", Style::default().fg(Color::Blue)),
                Span::styled(
                    "             Word forward/backward in search",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("  f", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Cycle time filters (All/Today/Week/Month)",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  s", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Cycle sort modes (Date/Cost/Tokens/Project)",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "\u{1f4cc} Visual Mode:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  v", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Enter/exit visual mode",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![Span::styled(
                "  In visual mode:",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![
                Span::styled("    j/k", Style::default().fg(Color::Blue)),
                Span::styled(
                    "             Extend selection",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("    x", Style::default().fg(Color::Blue)),
                Span::styled(
                    "               Mark/unmark for multi-select",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("    b", Style::default().fg(Color::Blue)),
                Span::styled(
                    "               Bookmark selected items",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("    e", Style::default().fg(Color::Blue)),
                Span::styled(
                    "               Export selected items",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "\u{26a1} Quick Actions:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  r", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Refresh data",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  e, Ctrl+E", Style::default().fg(Color::Green)),
                Span::styled(
                    "          Open export dialog (CSV/JSON)",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  c", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Clear status message",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ?", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Toggle help popup",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  q, Esc", Style::default().fg(Color::Green)),
                Span::styled(
                    "             Quit application",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "\u{1f4ca} Enhanced Tabs:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  1. Overview", Style::default().fg(Color::Green)),
                Span::styled(
                    "       Enhanced summary with quick stats",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  2. Daily", Style::default().fg(Color::Green)),
                Span::styled(
                    "         Color-coded daily usage breakdown",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  3. Weekly", Style::default().fg(Color::Green)),
                Span::styled(
                    "        Weekly usage aggregation",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  4. Sessions", Style::default().fg(Color::Green)),
                Span::styled(
                    "      Searchable session analytics",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  5. Cache", Style::default().fg(Color::Green)),
                Span::styled(
                    "         Cache analysis view",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  6. Billing", Style::default().fg(Color::Green)),
                Span::styled(
                    "       Billing blocks view",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  h. Help", Style::default().fg(Color::Green)),
                Span::styled(
                    "          This comprehensive help screen",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "\u{1f3a8} New Features:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  \u{2022} Real-time search and filtering",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  \u{2022} Multiple sorting options",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  \u{2022} Time-based filtering",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  \u{2022} Enhanced color coding",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  \u{2022} Export functionality",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  \u{2022} Improved keyboard navigation",
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press 'q' or 'Esc' to exit",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
        ];

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("\u{2753} Enhanced Help & Navigation")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(help, area);
    }
}
