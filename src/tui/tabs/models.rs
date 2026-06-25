use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::tui::TuiApp;

impl TuiApp {
    pub(crate) fn render_models(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        let agent_count = {
            let mut agents = self
                .model_report
                .models
                .iter()
                .map(|model| model.agent.as_str())
                .collect::<Vec<_>>();
            agents.sort_unstable();
            agents.dedup();
            agents.len()
        };

        let has_codex = self
            .model_report
            .models
            .iter()
            .any(|model| Self::is_openai_agent(&model.agent));
        let cache_write = if has_codex {
            format!(
                "{}*",
                Self::format_number_adaptive(
                    self.model_report.totals.cache_creation_tokens,
                    area.width < 110
                )
            )
        } else {
            Self::format_number_adaptive(
                self.model_report.totals.cache_creation_tokens,
                area.width < 110,
            )
        };
        let summary = Paragraph::new(format!(
            "Agents {} · Models {} · ${:.2} · {} tok · Cache W {} · R {}",
            agent_count,
            self.model_report.models.len(),
            self.model_report.totals.total_cost,
            Self::format_number_adaptive(self.model_report.totals.total_tokens, area.width < 110),
            cache_write,
            Self::format_number_adaptive(
                self.model_report.totals.cache_read_tokens,
                area.width < 110
            )
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(if self.model_report_loading {
                    "Models by Agent · loading"
                } else if has_codex {
                    "Models by Agent · Codex W — if unreported"
                } else {
                    "Models by Agent"
                })
                .border_style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: true });
        f.render_widget(summary, chunks[0]);

        if self.model_report_loading && self.model_report.models.is_empty() {
            let loading = Paragraph::new(
                "Loading model and agent usage in the background. Other TUI tabs remain available.",
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Loading Model Data")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true });
            f.render_widget(loading, chunks[1]);
            return;
        }

        if let Some(error) = &self.model_report_error {
            let error = Paragraph::new(format!("Failed to load model usage: {error}"))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Model Data Error")
                        .border_style(Style::default().fg(Color::Red)),
                )
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            f.render_widget(error, chunks[1]);
            return;
        }

        if self.model_report.models.is_empty() {
            let empty = Paragraph::new("No model usage found. Claude Code logs are read from ~/.claude and Codex logs from CODEX_HOME or ~/.codex.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("No Model Data")
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });
            f.render_widget(empty, chunks[1]);
            return;
        }

        let table_width = chunks[1].width;
        let compact = table_width < 112;
        let very_compact = table_width < 64;

        let headers: Vec<&str> = if very_compact {
            vec!["Agent", "Model", "Cost", "Hit%"]
        } else if compact {
            vec!["Agent", "Model", "Cost", "Tokens", "Hit%"]
        } else {
            vec![
                "Agent", "Family", "Model", "Cost", "Tokens", "Cache W", "Cache R", "Hit%",
            ]
        };

        let header = Row::new(headers.iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .height(1)
        .bottom_margin(1);

        let total_cost = self.model_report.totals.total_cost;
        let rows = self.model_report.models.iter().map(|model| {
            let cost_pct = if total_cost > 0.0 {
                model.total_cost / total_cost * 100.0
            } else {
                0.0
            };
            let cost = if compact {
                format!("${:.2}", model.total_cost)
            } else {
                format!("${:.2} ({:.1}%)", model.total_cost, cost_pct)
            };
            let hit_pct = Self::model_cache_hit_rate_pct(
                &model.agent,
                model.input_tokens,
                model.cache_read_tokens,
                model.cache_creation_tokens,
            );
            let model_name = if very_compact {
                Self::truncate_text(
                    &model.model,
                    table_width.saturating_sub(32).max(12) as usize,
                )
            } else if compact {
                Self::truncate_text(
                    &model.model,
                    table_width.saturating_sub(48).max(14) as usize,
                )
            } else {
                Self::truncate_text(&model.model, 32)
            };
            let agent_style = match model.agent.as_str() {
                "Codex" => Style::default().fg(Color::Green),
                "Claude Code" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::White),
            };
            let hit_color = Self::cache_hit_color(hit_pct);

            let mut cells = vec![
                Cell::from(model.agent.clone()).style(agent_style),
                Cell::from(model.family.clone()).style(Style::default().fg(Color::Blue)),
                Cell::from(model_name),
                Cell::from(cost).style(Style::default().fg(Self::cost_color(model.total_cost))),
            ];

            if compact {
                cells.remove(1);
            }

            if very_compact {
                cells.push(
                    Cell::from(format!("{:.1}%", hit_pct)).style(Style::default().fg(hit_color)),
                );
            } else {
                if !compact {
                    let cache_write = if Self::is_openai_agent(&model.agent)
                        && model.cache_creation_tokens == 0
                    {
                        "—".to_string()
                    } else {
                        Self::format_number_adaptive(model.cache_creation_tokens, false)
                    };
                    cells.push(
                        Cell::from(Self::format_number_adaptive(model.total_tokens, false))
                            .style(Style::default().fg(Color::Magenta)),
                    );
                    cells.push(Cell::from(cache_write).style(Style::default().fg(Color::Yellow)));
                    cells.push(
                        Cell::from(Self::format_number_adaptive(model.cache_read_tokens, false))
                            .style(Style::default().fg(Color::Blue)),
                    );
                } else {
                    cells.push(
                        Cell::from(Self::format_number_adaptive(model.total_tokens, true))
                            .style(Style::default().fg(Color::Magenta)),
                    );
                }
                cells.push(
                    Cell::from(format!("{:.1}%", hit_pct)).style(Style::default().fg(hit_color)),
                );
            }

            Row::new(cells).height(1)
        });

        let widths: Vec<Constraint> = if very_compact {
            vec![
                Constraint::Length(10),
                Constraint::Min(12),
                Constraint::Length(9),
                Constraint::Length(9),
            ]
        } else if compact {
            vec![
                Constraint::Length(11),
                Constraint::Min(14),
                Constraint::Length(9),
                Constraint::Length(9),
                Constraint::Length(8),
            ]
        } else {
            vec![
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Min(18),
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Length(11),
                Constraint::Length(11),
                Constraint::Length(8),
            ]
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if compact {
                        "Agent / Model Usage - Compact"
                    } else {
                        "Agent / Model Usage"
                    })
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("\u{25ba} ");

        f.render_stateful_widget(table, chunks[1], &mut self.model_table_state);

        let compact_total = area.width < 100;
        let total = Paragraph::new(format!(
            "${:.2} · {} tok · W {} · R {}",
            self.model_report.totals.total_cost,
            Self::format_number_adaptive(self.model_report.totals.total_tokens, compact_total),
            Self::format_number_adaptive(
                self.model_report.totals.cache_creation_tokens,
                compact_total
            ),
            Self::format_number_adaptive(self.model_report.totals.cache_read_tokens, compact_total)
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" TOTAL · Claude Code + Codex ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(total, chunks[2]);
    }
}
