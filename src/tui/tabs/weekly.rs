use chrono::{NaiveDate, Weekday};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::models::TokenUsage;
use crate::reports::generate_weekly_report_sorted;
use crate::tui::TuiApp;

impl TuiApp {
    pub(crate) fn ensure_weekly_report(&mut self) {
        if self.weekly_report.is_some() {
            return;
        }
        // Reconstruct DailyUsageMap from original_daily_report
        let mut daily_map = std::collections::HashMap::new();
        for day in &self.original_daily_report.daily {
            if let Ok(date) = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") {
                let usage = TokenUsage {
                    input_tokens: day.input_tokens,
                    output_tokens: day.output_tokens,
                    cache_creation_tokens: day.cache_creation_tokens,
                    cache_read_tokens: day.cache_read_tokens,
                    total_cost: day.total_cost,
                    fast_mode_cost: 0.0,
                };
                daily_map.insert(date, usage);
            }
        }
        let report = generate_weekly_report_sorted(daily_map, None, None, Weekday::Mon);
        if !report.weekly.is_empty() {
            self.weekly_table_state.select(Some(0));
        }
        self.weekly_report = Some(report);
    }

    pub(crate) fn render_weekly(&mut self, f: &mut Frame, area: Rect) {
        self.ensure_weekly_report();

        let report = match &self.weekly_report {
            Some(r) => r,
            None => {
                let msg = Paragraph::new("No weekly data available").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Weekly Report"),
                );
                f.render_widget(msg, area);
                return;
            }
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Header
        let header_cells = [
            "Week",
            "Days",
            "Input",
            "Output",
            "Total Tokens",
            "Cost",
            "Avg/Day",
        ]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = report
            .weekly
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let style = if i % 2 == 0 {
                    Style::default()
                } else {
                    Style::default().bg(Color::Rgb(30, 30, 30))
                };

                let cost_color = if w.total_cost > 5.0 {
                    Color::Red
                } else if w.total_cost > 2.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                let week_range = format!("{} ~ {}", w.week_start, w.week_end);

                Row::new(vec![
                    Cell::from(week_range).style(style),
                    Cell::from(format!("{}", w.days_active)).style(style),
                    Cell::from(Self::format_number(w.input_tokens))
                        .style(Style::default().fg(Color::Blue)),
                    Cell::from(Self::format_number(w.output_tokens))
                        .style(Style::default().fg(Color::Cyan)),
                    Cell::from(Self::format_number(w.total_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(format!("${:.2}", w.total_cost))
                        .style(Style::default().fg(cost_color)),
                    Cell::from(format!("${:.2}", w.avg_daily_cost))
                        .style(Style::default().fg(Color::White)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(25),
                Constraint::Length(6),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(14),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Weekly Report ({} weeks)", report.weekly.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(table, chunks[0], &mut self.weekly_table_state);

        // Totals bar
        let totals = &report.totals;
        let total_info = Paragraph::new(Line::from(vec![
            Span::styled("Totals: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("${:.2}", totals.total_cost),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("Tokens: {}", Self::format_number(totals.total_tokens)),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("In: {}", Self::format_number(totals.input_tokens)),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("Out: {}", Self::format_number(totals.output_tokens)),
                Style::default().fg(Color::Cyan),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(total_info, chunks[1]);
    }
}
