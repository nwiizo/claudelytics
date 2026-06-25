use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::tui::{SortMode, TuiApp};

impl TuiApp {
    pub(crate) fn render_daily(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Enhanced header with controls
        let controls_text = Line::from(vec![
            Span::styled("Controls: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "s",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Sort | ", Style::default().fg(Color::White)),
            Span::styled(
                "e",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Export | ", Style::default().fg(Color::White)),
            Span::styled(
                "r",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Refresh", Style::default().fg(Color::White)),
        ]);

        let sort_label = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Efficiency => "Efficiency",
            SortMode::Project => "Date (no project)",
        };

        let controls = Paragraph::new(controls_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("\u{1f4c5} Daily Report [Sort: {}]", sort_label)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        // Responsive table inspired by ccusage: keep the decision columns visible
        // and drop cache detail first on narrow terminals.
        let table_width = chunks[1].width;
        let compact = table_width < 96;
        let very_compact = table_width < 64;

        let headers: Vec<&str> = if very_compact {
            vec!["Date", "Cost", "Tokens", "Out"]
        } else if compact {
            vec!["Date", "Cost", "Tokens", "Out", "Hit%"]
        } else {
            vec![
                "Date", "Cost", "$/1M", "Tokens", "Input", "Output", "Cache%", "Hit%",
            ]
        };

        let header_cells = headers.iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.daily_report.daily.iter().enumerate().map(|(i, day)| {
            let style = if i == 0 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let cache_hit_pct =
                Self::cache_hit_rate_pct(day.cache_read_tokens, day.cache_creation_tokens);
            let cache_share_pct = Self::cache_share_pct(
                day.total_tokens,
                day.cache_read_tokens,
                day.cache_creation_tokens,
            );
            let cost_per_million = Self::cost_per_million_tokens(day.total_cost, day.total_tokens);

            let cost_color = Self::cost_color(day.total_cost);
            let hit_color = Self::cache_hit_color(cache_hit_pct);
            let date = if very_compact {
                Self::compact_date(&day.date)
            } else {
                day.date.clone()
            };

            let mut cells = vec![
                Cell::from(date).style(style),
                Cell::from(format!("${:.2}", day.total_cost))
                    .style(Style::default().fg(cost_color)),
            ];

            if !compact {
                cells.push(
                    Cell::from(format!("${:.2}", cost_per_million))
                        .style(Style::default().fg(Color::Green)),
                );
            }

            cells.push(
                Cell::from(Self::format_number_adaptive(day.total_tokens, compact))
                    .style(Style::default().fg(Color::Magenta)),
            );

            if !very_compact && !compact {
                cells.push(
                    Cell::from(Self::format_number_adaptive(day.input_tokens, compact))
                        .style(Style::default().fg(Color::Blue)),
                );
            }

            cells.push(
                Cell::from(Self::format_number_adaptive(day.output_tokens, compact))
                    .style(Style::default().fg(Color::Cyan)),
            );

            if !compact {
                cells.push(
                    Cell::from(format!("{:.1}%", cache_share_pct))
                        .style(Style::default().fg(Color::Yellow)),
                );
            }
            if !very_compact {
                cells.push(
                    Cell::from(format!("{:.1}%", cache_hit_pct))
                        .style(Style::default().fg(hit_color)),
                );
            }

            Row::new(cells).height(if very_compact { 2 } else { 1 })
        });

        let widths: Vec<Constraint> = if very_compact {
            vec![
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Min(8),
                Constraint::Length(8),
            ]
        } else if compact {
            vec![
                Constraint::Length(12),
                Constraint::Length(9),
                Constraint::Min(9),
                Constraint::Length(9),
                Constraint::Length(8),
            ]
        } else {
            vec![
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Length(8),
            ]
        };

        let title = if compact {
            "\u{1f4cb} Daily Usage Data - Compact"
        } else {
            "\u{1f4cb} Daily Usage Data"
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("\u{25ba} ");

        f.render_stateful_widget(table, chunks[1], &mut self.daily_table_state);
    }
}
