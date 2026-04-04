use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, Table, Wrap},
};

use crate::tui::{AppMode, SortMode, TuiApp};

impl TuiApp {
    /// Extract a human-readable project name from a hyphen-encoded path.
    ///
    /// Strategy: look for `github-com-OWNER-REPO` and return `OWNER/REPO`.
    /// Otherwise fall back to the last 2 hyphen-segments.
    pub(crate) fn extract_project_name(raw_path: &str) -> String {
        let stripped = raw_path.trim_start_matches('-');
        let segments: Vec<&str> = stripped.split('-').collect();

        // Look for github-com pattern
        for (i, seg) in segments.iter().enumerate() {
            if seg.eq_ignore_ascii_case("com")
                && i > 0
                && segments[i - 1].eq_ignore_ascii_case("github")
                && i + 2 < segments.len()
            {
                let owner = segments[i + 1];
                let repo = segments[i + 2..].join("-");
                return format!("{}/{}", owner, repo);
            }
        }

        // Fallback: last 2 segments
        if segments.len() >= 2 {
            format!(
                "{}/{}",
                segments[segments.len() - 2],
                segments[segments.len() - 1]
            )
        } else if segments.len() == 1 {
            segments[0].to_string()
        } else {
            raw_path.to_string()
        }
    }

    pub(crate) fn render_sessions(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Enhanced controls bar
        let comparison_info = if self.comparison_sessions.is_empty() {
            "No sessions selected for comparison".to_string()
        } else {
            format!(
                "Comparison: {} sessions selected",
                self.comparison_sessions.len()
            )
        };

        let controls_text = Line::from(vec![
            Span::styled(
                "x",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Compare | ", Style::default().fg(Color::White)),
            Span::styled(
                "b",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Bookmark | ", Style::default().fg(Color::White)),
            Span::styled(
                "/",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Search | ", Style::default().fg(Color::White)),
            Span::styled(
                "s",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Sort | ", Style::default().fg(Color::White)),
            Span::styled(comparison_info, Style::default().fg(Color::Cyan)),
        ]);

        let sort_label = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Efficiency => "Efficiency",
            SortMode::Project => "Project",
        };

        let controls = Paragraph::new(controls_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("\u{1f4ca} Sessions [Sort: {}]", sort_label)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        // Check if we have sessions to display
        if self.session_report.sessions.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("No sessions found for current filters"),
                Line::from(""),
                Line::from("Try:"),
                Line::from("- Press 'f' to change time filter"),
                Line::from("- Press 'r' to refresh data"),
                Line::from("- Check if you have usage data in ~/.claude/projects/"),
            ];
            let empty_paragraph = Paragraph::new(empty_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("\u{1f4cb} No Sessions Found")
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_paragraph, chunks[1]);
            return;
        }

        let header_cells = [
            "Project",
            "Session",
            "Cost",
            "Tokens",
            "Cache Hit%",
            "Last Activity",
        ]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self
            .session_report
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let full_path = if session.project_path.is_empty() {
                    session.session_id.clone()
                } else {
                    format!("{}/{}", session.project_path, session.session_id)
                };

                let parts: Vec<&str> = full_path.split('/').collect();
                let mut uuid_part = String::new();
                let mut dir_parts = Vec::new();
                for part in &parts {
                    if part.len() == 36
                        && part.chars().filter(|c| *c == '-').count() == 4
                        && part.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
                    {
                        if uuid_part.is_empty() {
                            uuid_part = part.to_string();
                        }
                    } else {
                        dir_parts.push(*part);
                    }
                }
                let dir_part = if dir_parts.is_empty() {
                    full_path.clone()
                } else {
                    dir_parts.join("/")
                };

                let project_name = Self::extract_project_name(&dir_part);
                let session_short = if uuid_part.len() >= 8 {
                    uuid_part[..8].to_string()
                } else {
                    "-".to_string()
                };

                let cache_denom = session.cache_read_tokens
                    + session.cache_creation_tokens
                    + session.input_tokens;
                let cache_hit_pct = if cache_denom > 0 {
                    session.cache_read_tokens as f64 / cache_denom as f64 * 100.0
                } else {
                    0.0
                };

                let cost_color = Self::cost_color(session.total_cost);

                let hit_color = Self::cache_hit_color(cache_hit_pct);

                let style = if self.current_mode == AppMode::Visual
                    && self.visual_mode_selections.contains(&i)
                {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else if i % 2 == 0 {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };

                Row::new(vec![
                    Cell::from(Self::truncate_text(&project_name, 30)).style(style),
                    Cell::from(session_short).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(format!("${:.2}", session.total_cost))
                        .style(Style::default().fg(cost_color)),
                    Cell::from(Self::format_number(session.total_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(format!("{:.1}%", cache_hit_pct))
                        .style(Style::default().fg(hit_color)),
                    Cell::from(session.last_activity.clone())
                        .style(Style::default().fg(Color::Yellow)),
                ])
                .height(1)
            });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(30),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "\u{1f4cb} Session Data ({} items)",
                    self.session_report.sessions.len()
                ))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25ba} ");

        f.render_stateful_widget(
            table,
            chunks[1].inner(Margin::new(0, 1)),
            &mut self.session_table_state,
        );

        // Enhanced scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("\u{2191}"))
            .end_symbol(Some("\u{2193}"));
        let scrollbar_area = chunks[1].inner(Margin::new(1, 1));
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.session_scroll_state);
    }
}
