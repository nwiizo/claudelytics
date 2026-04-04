use std::path::PathBuf;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::cache_analysis::{self, CacheAnalysis};
use crate::tui::TuiApp;

impl TuiApp {
    pub(crate) fn ensure_cache_analysis(&mut self) {
        if self.cache_analysis.is_some() {
            return;
        }
        // Discover claude directories
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let legacy = PathBuf::from(&home).join(".claude");
        let xdg = PathBuf::from(&home).join(".config").join("claude");

        let mut dirs = Vec::new();
        if legacy.exists() {
            dirs.push(legacy);
        }
        if xdg.exists() {
            dirs.push(xdg);
        }

        // Try each directory until we get a result
        for dir in &dirs {
            if let Ok(analysis) = cache_analysis::analyze_cache(dir, None, None, None, 0.5) {
                if !analysis.sessions.is_empty() {
                    self.cache_table_state.select(Some(0));
                }
                self.cache_analysis = Some(analysis);
                return;
            }
        }

        // If no directory worked, store an empty analysis
        self.cache_analysis = Some(CacheAnalysis {
            sessions: Vec::new(),
            total_cold_start: 0,
            total_5m_miss: 0,
            total_60m_miss: 0,
            total_normal_churn: 0,
            total_cache_writes: 0,
            total_cache_reads: 0,
            avg_warmup_turn: 0.0,
            avg_breakeven_turn: 0.0,
            project_aggregates: Vec::new(),
        });
    }

    pub(crate) fn render_cache(&mut self, f: &mut Frame, area: Rect) {
        self.ensure_cache_analysis();

        let analysis = match &self.cache_analysis {
            Some(a) => a,
            None => {
                let msg = Paragraph::new("No cache data available").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Cache Analysis"),
                );
                f.render_widget(msg, area);
                return;
            }
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Summary
                Constraint::Min(0),    // Sessions table
            ])
            .split(area);

        // Summary section
        let tw = analysis.total_cache_writes.max(1) as f64;
        let total_io = analysis
            .total_cache_writes
            .saturating_add(analysis.total_cache_reads);
        let hit_rate = if total_io > 0 {
            analysis.total_cache_reads as f64 / total_io as f64 * 100.0
        } else {
            0.0
        };

        let cold_pct = analysis.total_cold_start as f64 / tw * 100.0;
        let miss_5m_pct = analysis.total_5m_miss as f64 / tw * 100.0;
        let miss_60m_pct = analysis.total_60m_miss as f64 / tw * 100.0;
        let churn_pct = analysis.total_normal_churn as f64 / tw * 100.0;

        let summary_lines = vec![
            Line::from(vec![
                Span::styled("Overall Hit Rate: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.1}%", hit_rate),
                    Style::default()
                        .fg(if hit_rate > 70.0 {
                            Color::Green
                        } else if hit_rate > 40.0 {
                            Color::Yellow
                        } else {
                            Color::Red
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  |  "),
                Span::styled(
                    format!(
                        "Writes: {}  Reads: {}",
                        Self::format_tokens_static(analysis.total_cache_writes),
                        Self::format_tokens_static(analysis.total_cache_reads)
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("Avg Warmup Turn: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.1}", analysis.avg_warmup_turn),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  |  "),
                Span::styled("Avg Breakeven Turn: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.1}", analysis.avg_breakeven_turn),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Cache Write Breakdown:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    format!("  Cold Start: {:>5.1}%", cold_pct),
                    Style::default().fg(Color::Red),
                ),
                Span::raw("  |  "),
                Span::styled(
                    format!("5m Miss: {:>5.1}%", miss_5m_pct),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  |  "),
                Span::styled(
                    format!("60m Miss: {:>5.1}%", miss_60m_pct),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw("  |  "),
                Span::styled(
                    format!("Normal Churn: {:>5.1}%", churn_pct),
                    Style::default().fg(Color::Green),
                ),
            ]),
        ];

        let summary = Paragraph::new(summary_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cache Analysis Summary"),
        );
        f.render_widget(summary, chunks[0]);

        // Sessions table
        let header_cells = [
            "Session",
            "Project",
            "Turns",
            "Hit Rate",
            "Cold Start",
            "5m Miss",
            "60m Miss",
            "Churn",
            "Breakeven",
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

        let rows: Vec<Row> = analysis
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let style = if i % 2 == 0 {
                    Style::default()
                } else {
                    Style::default().bg(Color::Rgb(30, 30, 30))
                };

                let hit_color = if s.hit_rate_pct > 70.0 {
                    Color::Green
                } else if s.hit_rate_pct > 40.0 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let session_short = if s.session_id.len() > 12 {
                    format!("{}...", &s.session_id[..12])
                } else {
                    s.session_id.clone()
                };

                let project_short = if s.project.len() > 20 {
                    format!("{}...", &s.project[..20])
                } else {
                    s.project.clone()
                };

                let breakeven_str = match s.breakeven_turn {
                    Some(t) => format!("{}", t),
                    None => "-".to_string(),
                };

                Row::new(vec![
                    Cell::from(session_short).style(style),
                    Cell::from(project_short).style(style),
                    Cell::from(format!("{}", s.turn_count)).style(style),
                    Cell::from(format!("{:.1}%", s.hit_rate_pct))
                        .style(Style::default().fg(hit_color)),
                    Cell::from(Self::format_tokens_static(s.cold_start_tokens))
                        .style(Style::default().fg(Color::Red)),
                    Cell::from(Self::format_tokens_static(s.ttl_5m_miss_tokens))
                        .style(Style::default().fg(Color::Yellow)),
                    Cell::from(Self::format_tokens_static(s.ttl_60m_miss_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(Self::format_tokens_static(s.normal_churn_tokens))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(breakeven_str).style(style),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(15),
                Constraint::Length(22),
                Constraint::Length(6),
                Constraint::Length(9),
                Constraint::Length(11),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Sessions ({} total)", analysis.sessions.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(table, chunks[1], &mut self.cache_table_state);
    }
}
