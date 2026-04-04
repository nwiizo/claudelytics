use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, Table, Wrap},
};

use crate::tui::TuiApp;

impl TuiApp {
    pub(crate) fn render_billing_blocks(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.show_billing_summary { 12 } else { 3 }),
                Constraint::Min(0),
            ])
            .split(area);

        let report = self.billing_manager.generate_report();

        // Summary section
        if self.show_billing_summary {
            let current_block = self.billing_manager.get_current_block();
            let current_block_cost = current_block.map(|b| b.usage.total_cost).unwrap_or(0.0);
            let current_block_info = current_block
                .map(|b| {
                    format!(
                        "{} - {}",
                        b.start_time.format("%H:%M UTC"),
                        b.end_time.format("%H:%M UTC")
                    )
                })
                .unwrap_or_else(|| "No active block".to_string());

            let summary_text = vec![
                Line::from(vec![
                    Span::styled(
                        "\u{1f4b0} Current Block Cost: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("${:.2}", current_block_cost),
                        Style::default()
                            .fg(if current_block_cost > 5.0 {
                                Color::Red
                            } else if current_block_cost > 2.5 {
                                Color::Yellow
                            } else {
                                Color::Green
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        "\u{1f4ca} Total Blocks: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("{}", report.blocks.len()),
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "\u{23f0} Current Period: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(current_block_info, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("\u{1f4c8} Peak Block: ", Style::default().fg(Color::White)),
                    Span::styled(
                        if let Some(ref peak) = report.peak_block {
                            format!(
                                "${:.2} ({} {})",
                                peak.usage.total_cost, peak.date, peak.time_range
                            )
                        } else {
                            "No peak block yet".to_string()
                        },
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "\u{1f4b5} Average per Block: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("${:.2}", report.average_per_block.total_cost),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("\u{1f3af} Total Cost: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.2}", report.total_usage.total_cost),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "\u{1f4be} Cache Status: ",
                        Style::default().fg(Color::White),
                    ),
                    if let Some(ref cache_status) = self.pricing_cache_status {
                        if cache_status.exists && cache_status.valid {
                            Span::styled(
                                format!(
                                    "\u{2713} Valid (Updated: {}, {} models)",
                                    cache_status.last_updated, cache_status.model_count
                                ),
                                Style::default().fg(Color::Green),
                            )
                        } else if cache_status.exists {
                            Span::styled(
                                "\u{26a0} Expired - Update recommended",
                                Style::default().fg(Color::Yellow),
                            )
                        } else {
                            Span::styled(
                                "\u{2717} No cache - Using fallback pricing",
                                Style::default().fg(Color::Red),
                            )
                        }
                    } else {
                        Span::styled(
                            "\u{2717} No cache - Using fallback pricing",
                            Style::default().fg(Color::Red),
                        )
                    },
                ]),
            ];

            let summary = Paragraph::new(summary_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("\u{23f0} 5-Hour Billing Blocks Summary")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(summary, chunks[0]);
        } else {
            let controls = Paragraph::new("Press 's' to toggle summary | Arrow keys to navigate")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("\u{23f0} Billing Blocks"),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(controls, chunks[0]);
        }

        // Billing blocks table
        if report.blocks.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("No billing blocks found"),
                Line::from(""),
                Line::from("Billing blocks track usage in 5-hour periods:"),
                Line::from("  00:00-05:00, 05:00-10:00, 10:00-15:00"),
                Line::from("  15:00-20:00, 20:00-00:00 (UTC)"),
            ];
            let empty_paragraph = Paragraph::new(empty_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("\u{1f4cb} No Billing Blocks")
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_paragraph, chunks[1]);
            return;
        }

        let header_cells = [
            "Block Period",
            "Cost",
            "Tokens",
            "Sessions",
            "Avg/Session",
            "% of Total",
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

        let current_block = self.billing_manager.get_current_block();
        let rows = report.blocks.iter().enumerate().map(|(i, block)| {
            let is_current = i == 0 && current_block.is_some();
            let style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let cost_color = if block.usage.total_cost > 5.0 {
                Color::Red
            } else if block.usage.total_cost > 2.5 {
                Color::Yellow
            } else {
                Color::Green
            };

            let avg_per_session = if block.session_count > 0 {
                block.usage.total_cost / block.session_count as f64
            } else {
                0.0
            };

            let percentage = if report.total_usage.total_cost > 0.0 {
                (block.usage.total_cost / report.total_usage.total_cost) * 100.0
            } else {
                0.0
            };

            Row::new(vec![
                Cell::from(format!("{} - {}", &block.date, &block.time_range)).style(style),
                Cell::from(format!("${:.2}", block.usage.total_cost))
                    .style(Style::default().fg(cost_color)),
                Cell::from(Self::format_number(block.usage.total_tokens()))
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(format!("{}", block.session_count))
                    .style(Style::default().fg(Color::Blue)),
                Cell::from(format!("${:.2}", avg_per_session))
                    .style(Style::default().fg(Color::Yellow)),
                Cell::from(format!("{:.1}%", percentage)).style(Style::default().fg(Color::Cyan)),
            ])
            .height(1)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(28),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("\u{1f4cb} 5-Hour Billing Blocks")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25ba} ");

        f.render_stateful_widget(table, chunks[1], &mut self.billing_blocks_table_state);

        // Scrollbar
        if report.blocks.len() > 10 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("\u{2191}"))
                .end_symbol(Some("\u{2193}"));
            f.render_stateful_widget(
                scrollbar,
                chunks[1].inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut self.billing_blocks_scroll_state,
            );
        }
    }
}
