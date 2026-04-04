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
                "f",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Filter | ", Style::default().fg(Color::White)),
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
            SortMode::Project => "Date",
        };

        let controls = Paragraph::new(controls_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("\u{1f4c5} Daily Report [Sort: {}]", sort_label)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        // Enhanced table with color coding
        let header_cells = ["Date", "Cost", "Tokens", "Input", "Output", "Cache", "Hit%"]
            .iter()
            .map(|h| {
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

            let cache_denom = day.cache_read_tokens + day.cache_creation_tokens + day.input_tokens;
            let cache_hit_pct = if cache_denom > 0 {
                day.cache_read_tokens as f64 / cache_denom as f64 * 100.0
            } else {
                0.0
            };

            let cost_color = if day.total_cost > 1.0 {
                Color::Red
            } else if day.total_cost > 0.5 {
                Color::Yellow
            } else {
                Color::Green
            };

            let hit_color = if cache_hit_pct > 95.0 {
                Color::Green
            } else if cache_hit_pct > 85.0 {
                Color::Yellow
            } else {
                Color::Red
            };

            Row::new(vec![
                Cell::from(day.date.clone()).style(style),
                Cell::from(format!("${:.2}", day.total_cost))
                    .style(Style::default().fg(cost_color)),
                Cell::from(self.format_number(day.total_tokens))
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(self.format_number(day.input_tokens))
                    .style(Style::default().fg(Color::Blue)),
                Cell::from(self.format_number(day.output_tokens))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(self.format_number(day.cache_creation_tokens + day.cache_read_tokens))
                    .style(Style::default().fg(Color::Yellow)),
                Cell::from(format!("{:.1}%", cache_hit_pct)).style(Style::default().fg(hit_color)),
            ])
            .height(1)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("\u{1f4cb} Daily Usage Data")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25ba} ");

        f.render_stateful_widget(table, chunks[1], &mut self.daily_table_state);
    }
}
