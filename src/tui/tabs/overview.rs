use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

use crate::tui::TuiApp;
use crate::tui_visuals::{ProgressColorScheme, SmoothProgressBar};

impl TuiApp {
    pub(crate) fn render_overview(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Enhanced summary
                Constraint::Length(8),  // Quick stats
                Constraint::Length(3),  // Progress bars
                Constraint::Min(0),     // Cost breakdown with chart
            ])
            .margin(1)
            .split(area);

        // Enhanced summary card
        let summary_text = vec![
            Line::from(vec![
                Span::styled("\u{1f4b0} Total Cost: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("${:.2}", self.daily_report.totals.total_cost),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("\u{1f4c5} Days: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}", self.daily_report.daily.len()),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("\u{1f4c2} Sessions: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}", self.session_report.sessions.len()),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "\u{1f3af} Total Tokens: ",
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    self.format_number(self.daily_report.totals.total_tokens),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("\u{1f4e5} Input: ", Style::default().fg(Color::White)),
                Span::styled(
                    self.format_number(self.daily_report.totals.input_tokens),
                    Style::default().fg(Color::Green),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("\u{1f4e4} Output: ", Style::default().fg(Color::White)),
                Span::styled(
                    self.format_number(self.daily_report.totals.output_tokens),
                    Style::default().fg(Color::Blue),
                ),
            ]),
            Line::from(vec![
                Span::styled("\u{1f504} Cache: ", Style::default().fg(Color::White)),
                Span::styled(
                    self.format_number(
                        self.daily_report.totals.cache_creation_tokens
                            + self.daily_report.totals.cache_read_tokens,
                    ),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "\u{26a1} Quick Actions: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("/ Search", Style::default().fg(Color::Gray)),
                Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                Span::styled("s Sort", Style::default().fg(Color::Gray)),
                Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                Span::styled("f Filter", Style::default().fg(Color::Gray)),
                Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                Span::styled("r Refresh", Style::default().fg(Color::Gray)),
            ]),
        ];

        let summary = Paragraph::new(summary_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("\u{1f4ca} Enhanced Usage Summary")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(summary, chunks[0]);

        // Quick stats with current filters
        let filter_info = match self.time_filter {
            crate::tui::TimeFilter::All => "All Time",
            crate::tui::TimeFilter::Today => "Today",
            crate::tui::TimeFilter::LastWeek => "Last Week",
            crate::tui::TimeFilter::LastMonth => "Last Month",
        };

        let sort_info = match self.sort_mode {
            crate::tui::SortMode::Date => "Date",
            crate::tui::SortMode::Cost => "Cost",
            crate::tui::SortMode::Tokens => "Tokens",
            crate::tui::SortMode::Efficiency => "Efficiency",
            crate::tui::SortMode::Project => "Project",
        };

        let stats_text = vec![
            Line::from(vec![
                Span::styled("\u{1f50d} Current View: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    filter_info,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("\u{1f4ca} Sort: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    sort_info,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "\u{1f4c8} Avg Cost/Day: ",
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        "${:.2}",
                        if self.daily_report.daily.is_empty() {
                            0.0
                        } else {
                            self.daily_report.totals.total_cost
                                / self.daily_report.daily.len() as f64
                        }
                    ),
                    Style::default().fg(Color::Green),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "\u{1f680} Avg Tokens/Day: ",
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    self.format_number(if self.daily_report.daily.is_empty() {
                        0
                    } else {
                        self.daily_report.totals.total_tokens / self.daily_report.daily.len() as u64
                    }),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
        ];

        let stats = Paragraph::new(stats_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("\u{1f4cb} Current Filters & Stats")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(stats, chunks[1]);

        // Render progress bars
        let progress_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Compute dynamic maximums from actual data
        let max_daily_cost = self
            .daily_report
            .daily
            .iter()
            .map(|d| d.total_cost)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let max_daily_tokens = self
            .daily_report
            .daily
            .iter()
            .map(|d| d.total_tokens)
            .max()
            .unwrap_or(1) as f64;
        let cost_ceiling = (max_daily_cost * 1.2).max(10.0);
        let token_ceiling = (max_daily_tokens * 1.2).max(100_000.0);

        // Cost progress bar
        if self.visual_effects.progress_bars.is_empty() {
            let mut cost_bar = SmoothProgressBar::new("Daily Cost".to_string(), cost_ceiling);
            cost_bar.set_value(self.daily_report.totals.total_cost);
            cost_bar.set_color_scheme(ProgressColorScheme::CostBased);
            self.visual_effects.progress_bars.push(cost_bar);

            let mut token_bar = SmoothProgressBar::new("Token Usage".to_string(), token_ceiling);
            token_bar.set_value(self.daily_report.totals.total_tokens as f64);
            token_bar.set_color_scheme(ProgressColorScheme::TokenBased);
            self.visual_effects.progress_bars.push(token_bar);
        }

        // Update progress bar values and max
        if let Some(cost_bar) = self.visual_effects.progress_bars.get_mut(0) {
            cost_bar.set_max(cost_ceiling);
            cost_bar.set_value(self.daily_report.totals.total_cost);
            cost_bar.render(f, progress_chunks[0]);
        }

        if let Some(token_bar) = self.visual_effects.progress_bars.get_mut(1) {
            token_bar.set_max(token_ceiling);
            token_bar.set_value(self.daily_report.totals.total_tokens as f64);
            token_bar.render(f, progress_chunks[1]);
        }

        // Cost gauge based on total cost relative to dynamic max
        if self.daily_report.totals.total_cost > 0.0 {
            let cost_ratio = (self.daily_report.totals.total_cost / cost_ceiling).min(1.0);
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("\u{1f4b3} Total Cost")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(cost_ratio)
                .label(format!(
                    "${:.2} / ${:.0}",
                    self.daily_report.totals.total_cost, cost_ceiling
                ));
            f.render_widget(gauge, chunks[3]);
        }
    }
}
