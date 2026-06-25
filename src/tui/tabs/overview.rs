use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table},
};
use std::collections::BTreeMap;

use crate::models::ModelUsage;
use crate::tui::{SortMode, TuiApp};

#[derive(Default)]
struct AgentSummary {
    tokens: u64,
    cost: f64,
}

impl TuiApp {
    pub(crate) fn render_overview(&self, frame: &mut Frame, area: Rect) {
        let sort = match self.sort_mode {
            SortMode::Date => "date",
            SortMode::Cost => "cost",
            SortMode::Tokens => "tokens",
            SortMode::Efficiency => "efficiency",
            SortMode::Project => "project",
        };

        let outer = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Audit overview · loaded scope · sorted by {sort} "
            ))
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.width < 40 || inner.height < 8 {
            frame.render_widget(
                Paragraph::new("Terminal is too small. Resize to at least 40×10."),
                inner,
            );
            return;
        }

        let show_trend = inner.height >= 17;
        let sections = if show_trend {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(7),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(0),
                    Constraint::Length(1),
                ])
                .split(inner)
        };

        self.render_overview_totals(frame, sections[0]);
        self.render_overview_details(frame, sections[1]);
        if show_trend {
            self.render_overview_trend(frame, sections[2]);
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("4", Style::default().fg(Color::Yellow)),
                Span::raw(" models  "),
                Span::styled("5", Style::default().fg(Color::Yellow)),
                Span::raw(" sessions  "),
                Span::styled("2", Style::default().fg(Color::Yellow)),
                Span::raw(" daily  "),
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::raw(" refresh  "),
                Span::styled("?", Style::default().fg(Color::Yellow)),
                Span::raw(" help"),
            ])),
            sections[3],
        );
    }

    fn render_overview_totals(&self, frame: &mut Frame, area: Rect) {
        let totals = &self.daily_report.totals;
        let cache_share = Self::cache_share_pct(
            totals.total_tokens,
            totals.cache_read_tokens,
            totals.cache_creation_tokens,
        );
        let separator = Span::styled("  │  ", Style::default().fg(Color::DarkGray));
        let line = Line::from(vec![
            Span::styled("COST ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("${:.2}", totals.total_cost),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            separator.clone(),
            Span::styled("TOKENS ", Style::default().fg(Color::Gray)),
            Span::styled(
                Self::format_number(totals.total_tokens),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            separator.clone(),
            Span::styled("CACHE ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{cache_share:.1}%"),
                Style::default().fg(Color::Yellow),
            ),
            separator.clone(),
            Span::styled("DAYS ", Style::default().fg(Color::Gray)),
            Span::styled(
                self.daily_report.daily.len().to_string(),
                Style::default().fg(Color::Blue),
            ),
            separator,
            Span::styled("SESSIONS ", Style::default().fg(Color::Gray)),
            Span::styled(
                self.session_report.sessions.len().to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]);

        frame.render_widget(
            Paragraph::new(line).block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_overview_details(&self, frame: &mut Frame, area: Rect) {
        if area.width >= 112 {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
                .split(area);
            self.render_agent_summary(frame, columns[0]);
            self.render_top_models(frame, columns[1]);
        } else {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(4)])
                .split(area);
            self.render_agent_summary(frame, rows[0]);
            self.render_top_models(frame, rows[1]);
        }
    }

    fn render_agent_summary(&self, frame: &mut Frame, area: Rect) {
        let mut summaries = BTreeMap::from([
            ("Claude Code".to_string(), AgentSummary::default()),
            ("Codex".to_string(), AgentSummary::default()),
        ]);
        for model in &self.model_report.models {
            let summary = summaries.entry(model.agent.clone()).or_default();
            summary.tokens = summary.tokens.saturating_add(model.total_tokens);
            summary.cost += model.total_cost;
        }

        let total_cost: f64 = summaries.values().map(|summary| summary.cost).sum();
        let total_tokens = summaries
            .values()
            .fold(0_u64, |total, summary| total.saturating_add(summary.tokens));
        let rows = summaries.into_iter().map(|(agent, summary)| {
            let share = if total_cost > 0.0 {
                summary.cost / total_cost * 100.0
            } else if total_tokens > 0 {
                summary.tokens as f64 / total_tokens as f64 * 100.0
            } else {
                0.0
            };
            Row::new(vec![
                Cell::from(agent),
                Cell::from(format!("${:.2}", summary.cost)),
                Cell::from(format!("{share:.1}%")),
                Cell::from(Self::format_number(summary.tokens)),
            ])
        });
        let title = if self.model_report_loading {
            " Agent split · loading "
        } else {
            " Agent split "
        };
        let table = Table::new(
            rows,
            [
                Constraint::Length(13),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(["Agent", "Cost", "Share", "Tokens"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .column_spacing(1);
        frame.render_widget(table, area);
    }

    fn render_top_models(&self, frame: &mut Frame, area: Rect) {
        let rows = self
            .model_report
            .models
            .iter()
            .take(area.height.saturating_sub(3) as usize)
            .map(model_row);
        let title = if let Some(error) = &self.model_report_error {
            format!(" Top models · {error} ")
        } else if self.model_report_loading {
            " Top models · loading ".to_string()
        } else {
            " Top models by cost · loaded scope ".to_string()
        };
        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Min(18),
                Constraint::Length(10),
                Constraint::Length(12),
            ],
        )
        .header(
            Row::new(["Agent", "Model", "Cost", "Tokens"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .column_spacing(1);
        frame.render_widget(table, area);
    }

    fn render_overview_trend(&self, frame: &mut Frame, area: Rect) {
        let mut days: Vec<_> = self.daily_report.daily.iter().collect();
        days.sort_by(|left, right| left.date.cmp(&right.date));
        let start = days
            .len()
            .saturating_sub(area.width.saturating_sub(2) as usize);
        let data: Vec<_> = days[start..]
            .iter()
            .map(|day| (day.total_cost.max(0.0) * 100.0).round() as u64)
            .collect();
        let date_range = days
            .first()
            .zip(days.last())
            .map(|(first, last)| format!("{} → {}", first.date, last.date))
            .unwrap_or_else(|| "no daily data".to_string());
        let trend = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Daily cost trend · {date_range} "))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .data(&data)
            .style(Style::default().fg(Color::Green));
        frame.render_widget(trend, area);
    }
}

fn model_row(model: &ModelUsage) -> Row<'_> {
    Row::new(vec![
        Cell::from(model.agent.as_str()),
        Cell::from(model.model.as_str()),
        Cell::from(format!("${:.2}", model.total_cost)),
        Cell::from(TuiApp::format_number(model.total_tokens)),
    ])
}
