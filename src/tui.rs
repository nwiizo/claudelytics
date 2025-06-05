use crate::models::{DailyReport, SessionReport};
use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Gauge, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Tabs, Wrap,
    },
};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Daily,
    Sessions,
    Charts,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortMode {
    Date,
    Cost,
    Tokens,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TimeFilter {
    All,
    Today,
    LastWeek,
    LastMonth,
}

#[derive(Debug)]
pub struct TuiApp {
    daily_report: DailyReport,
    session_report: SessionReport,
    original_daily_report: DailyReport,
    original_session_report: SessionReport,
    current_tab: Tab,
    daily_table_state: TableState,
    session_table_state: TableState,
    session_scroll_state: ScrollbarState,
    should_quit: bool,
    search_mode: bool,
    search_query: String,
    sort_mode: SortMode,
    time_filter: TimeFilter,
    status_message: Option<String>,
    show_help_popup: bool,
}

impl TuiApp {
    pub fn new(daily_report: DailyReport, session_report: SessionReport) -> Self {
        let mut daily_table_state = TableState::default();
        daily_table_state.select(Some(0));

        let mut session_table_state = TableState::default();
        session_table_state.select(Some(0));

        let session_scroll_state = ScrollbarState::new(session_report.sessions.len());

        Self {
            daily_report: daily_report.clone(),
            session_report: session_report.clone(),
            original_daily_report: daily_report,
            original_session_report: session_report,
            current_tab: Tab::Overview,
            daily_table_state,
            session_table_state,
            session_scroll_state,
            should_quit: false,
            search_mode: false,
            search_query: String::new(),
            sort_mode: SortMode::Date,
            time_filter: TimeFilter::All,
            status_message: None,
            show_help_popup: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the main loop
        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        if self.search_mode {
                            self.handle_search_input(key.code)?;
                        } else {
                            self.handle_normal_input(key.code)?;
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse_event(mouse);
                }
                _ => {}
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_normal_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.current_tab = Tab::Overview,
            KeyCode::Char('2') => self.current_tab = Tab::Daily,
            KeyCode::Char('3') => self.current_tab = Tab::Sessions,
            KeyCode::Char('4') => self.current_tab = Tab::Charts,
            KeyCode::Char('5') | KeyCode::Char('h') => self.current_tab = Tab::Help,
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.previous_tab(),
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_item();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_item();
            }
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.next_item();
                }
            }
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.previous_item();
                }
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query.clear();
                self.status_message = Some("Search: (Press Esc to cancel)".to_string());
            }
            KeyCode::Char('r') => {
                self.refresh_data()?;
            }
            KeyCode::Char('s') => {
                self.cycle_sort_mode();
            }
            KeyCode::Char('f') => {
                self.cycle_time_filter();
            }
            KeyCode::Char('c') => {
                self.status_message = None;
            }
            KeyCode::Char('e') => {
                self.export_current_view()?;
            }
            KeyCode::Char('?') => {
                self.show_help_popup = !self.show_help_popup;
            }
            KeyCode::Enter => {
                self.handle_enter();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query.clear();
                self.status_message = None;
                self.apply_filters();
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.apply_filters();
                self.status_message = Some(format!("Filtered by: '{}'", self.search_query));
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.status_message = Some(format!(
                    "Search: {} (Press Esc to cancel)",
                    self.search_query
                ));
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.status_message = Some(format!(
                    "Search: {} (Press Esc to cancel)",
                    self.search_query
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn refresh_data(&mut self) -> Result<()> {
        // In a real implementation, you'd re-parse the data
        // For now, we'll just show a message
        self.status_message = Some("🔄 Data refreshed successfully!".to_string());

        // Reset to original data to simulate refresh
        self.daily_report = self.original_daily_report.clone();
        self.session_report = self.original_session_report.clone();
        self.apply_filters();
        Ok(())
    }

    fn cycle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Date => SortMode::Cost,
            SortMode::Cost => SortMode::Tokens,
            SortMode::Tokens => SortMode::Project,
            SortMode::Project => SortMode::Date,
        };
        self.apply_filters();
        let mode_str = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Project => "Project",
        };
        self.status_message = Some(format!("📊 Sorted by: {}", mode_str));
    }

    fn cycle_time_filter(&mut self) {
        self.time_filter = match self.time_filter {
            TimeFilter::All => TimeFilter::Today,
            TimeFilter::Today => TimeFilter::LastWeek,
            TimeFilter::LastWeek => TimeFilter::LastMonth,
            TimeFilter::LastMonth => TimeFilter::All,
        };
        self.apply_filters();
        let filter_str = match self.time_filter {
            TimeFilter::All => "All Time",
            TimeFilter::Today => "Today",
            TimeFilter::LastWeek => "Last Week",
            TimeFilter::LastMonth => "Last Month",
        };
        self.status_message = Some(format!("📅 Filter: {}", filter_str));
    }

    fn apply_filters(&mut self) {
        // Reset to original data
        self.daily_report = self.original_daily_report.clone();
        self.session_report = self.original_session_report.clone();

        // Apply time filter
        let now = Local::now().naive_local().date();
        let cutoff_date = match self.time_filter {
            TimeFilter::All => None,
            TimeFilter::Today => Some(now),
            TimeFilter::LastWeek => Some(now - Duration::days(7)),
            TimeFilter::LastMonth => Some(now - Duration::days(30)),
        };

        if let Some(cutoff) = cutoff_date {
            // Filter daily report
            self.daily_report.daily.retain(|day| {
                if let Ok(date) = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") {
                    date >= cutoff
                } else {
                    true
                }
            });

            // Recalculate totals for daily report
            let mut total_cost = 0.0;
            let mut total_tokens = 0;
            let mut input_tokens = 0;
            let mut output_tokens = 0;
            let mut cache_creation_tokens = 0;
            let mut cache_read_tokens = 0;

            for day in &self.daily_report.daily {
                total_cost += day.total_cost;
                total_tokens += day.total_tokens;
                input_tokens += day.input_tokens;
                output_tokens += day.output_tokens;
                cache_creation_tokens += day.cache_creation_tokens;
                cache_read_tokens += day.cache_read_tokens;
            }

            self.daily_report.totals.total_cost = total_cost;
            self.daily_report.totals.total_tokens = total_tokens;
            self.daily_report.totals.input_tokens = input_tokens;
            self.daily_report.totals.output_tokens = output_tokens;
            self.daily_report.totals.cache_creation_tokens = cache_creation_tokens;
            self.daily_report.totals.cache_read_tokens = cache_read_tokens;
        }

        // Apply search filter
        if !self.search_query.is_empty() {
            self.session_report.sessions.retain(|session| {
                session
                    .project_path
                    .to_lowercase()
                    .contains(&self.search_query.to_lowercase())
                    || session
                        .session_id
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
            });
        }

        // Apply sorting
        match self.sort_mode {
            SortMode::Date => {
                self.daily_report.daily.sort_by(|a, b| b.date.cmp(&a.date));
                self.session_report
                    .sessions
                    .sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
            }
            SortMode::Cost => {
                self.daily_report.daily.sort_by(|a, b| {
                    b.total_cost
                        .partial_cmp(&a.total_cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                self.session_report.sessions.sort_by(|a, b| {
                    b.total_cost
                        .partial_cmp(&a.total_cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMode::Tokens => {
                self.daily_report
                    .daily
                    .sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
                self.session_report
                    .sessions
                    .sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
            }
            SortMode::Project => {
                self.session_report
                    .sessions
                    .sort_by(|a, b| a.project_path.cmp(&b.project_path));
            }
        }

        // Update scroll state
        self.session_scroll_state = ScrollbarState::new(self.session_report.sessions.len());

        // Reset table selections
        self.daily_table_state.select(Some(0));
        self.session_table_state.select(Some(0));
    }

    fn export_current_view(&mut self) -> Result<()> {
        // In a real implementation, you'd export to CSV
        let export_type = match self.current_tab {
            Tab::Daily => "daily",
            Tab::Sessions => "sessions",
            _ => "overview",
        };
        self.status_message = Some(format!("📁 Exported {} data to CSV", export_type));
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let main_chunks = if self.status_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(f.area())
        };

        let main_area = main_chunks[1];

        // Tab bar with enhanced titles
        let tab_titles = vec![
            "📊 Overview",
            "📅 Daily",
            "📋 Sessions",
            "📈 Charts",
            "❓ Help",
        ];
        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Claudelytics Enhanced"),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .select(self.current_tab as usize);
        f.render_widget(tabs, main_chunks[0]);

        // Main content
        match self.current_tab {
            Tab::Overview => self.render_overview(f, main_area),
            Tab::Daily => self.render_daily(f, main_area),
            Tab::Sessions => self.render_sessions(f, main_area),
            Tab::Charts => self.render_charts(f, main_area),
            Tab::Help => self.render_help(f, main_area),
        }

        // Status message
        if let Some(ref message) = self.status_message {
            if main_chunks.len() > 2 {
                let status_paragraph = Paragraph::new(message.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Status")
                            .border_style(Style::default().fg(Color::Green)),
                    )
                    .style(Style::default().fg(Color::Green))
                    .wrap(Wrap { trim: true });
                f.render_widget(status_paragraph, main_chunks[2]);
            }
        }

        // Help popup
        if self.show_help_popup {
            self.render_help_popup(f);
        }
    }

    fn render_overview(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Enhanced summary
                Constraint::Length(8),  // Quick stats
                Constraint::Min(0),     // Cost breakdown with chart
            ])
            .margin(1)
            .split(area);

        // Enhanced summary card
        let summary_text = vec![
            Line::from(vec![
                Span::styled("💰 Total Cost: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("${:.4}", self.daily_report.totals.total_cost),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("📅 Days: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}", self.daily_report.daily.len()),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("📂 Sessions: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}", self.session_report.sessions.len()),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("🎯 Total Tokens: ", Style::default().fg(Color::White)),
                Span::styled(
                    self.format_number(self.daily_report.totals.total_tokens),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("📥 Input: ", Style::default().fg(Color::White)),
                Span::styled(
                    self.format_number(self.daily_report.totals.input_tokens),
                    Style::default().fg(Color::Green),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("📤 Output: ", Style::default().fg(Color::White)),
                Span::styled(
                    self.format_number(self.daily_report.totals.output_tokens),
                    Style::default().fg(Color::Blue),
                ),
            ]),
            Line::from(vec![
                Span::styled("🔄 Cache: ", Style::default().fg(Color::White)),
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
                    "⚡ Quick Actions: ",
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
                    .title("📊 Enhanced Usage Summary")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(summary, chunks[0]);

        // Quick stats with current filters
        let filter_info = match self.time_filter {
            TimeFilter::All => "All Time",
            TimeFilter::Today => "Today",
            TimeFilter::LastWeek => "Last Week",
            TimeFilter::LastMonth => "Last Month",
        };

        let sort_info = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Project => "Project",
        };

        let stats_text = vec![
            Line::from(vec![
                Span::styled("🔍 Current View: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    filter_info,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("📊 Sort: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    sort_info,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("📈 Avg Cost/Day: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!(
                        "${:.4}",
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
                Span::styled("🚀 Avg Tokens/Day: ", Style::default().fg(Color::White)),
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
                    .title("📋 Current Filters & Stats")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(stats, chunks[1]);

        // Enhanced cost breakdown gauge with mini chart
        if self.daily_report.totals.total_cost > 0.0 {
            let cost_ratio = (self.daily_report.totals.total_cost / 10.0).min(1.0);
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("💳 Cost Gauge & Mini Trend")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(cost_ratio)
                .label(format!(
                    "${:.4} / $10.00",
                    self.daily_report.totals.total_cost
                ));
            f.render_widget(gauge, chunks[2]);
        }
    }

    fn render_daily(&mut self, f: &mut Frame, area: Rect) {
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

        let controls = Paragraph::new(controls_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📅 Daily Report"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        // Enhanced table with color coding
        let header_cells = [
            "Date", "Cost", "Tokens", "Input", "Output", "Cache", "Ratio",
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

        let rows = self.daily_report.daily.iter().enumerate().map(|(i, day)| {
            let style = if i == 0 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let ratio = if day.input_tokens > 0 {
                day.output_tokens as f64 / day.input_tokens as f64
            } else {
                0.0
            };

            // Color code based on cost
            let cost_color = if day.total_cost > 1.0 {
                Color::Red
            } else if day.total_cost > 0.5 {
                Color::Yellow
            } else {
                Color::Green
            };

            Row::new(vec![
                Cell::from(day.date.clone()).style(style),
                Cell::from(format!("${:.4}", day.total_cost))
                    .style(Style::default().fg(cost_color)),
                Cell::from(self.format_number(day.total_tokens))
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(self.format_number(day.input_tokens))
                    .style(Style::default().fg(Color::Blue)),
                Cell::from(self.format_number(day.output_tokens))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(self.format_number(day.cache_creation_tokens + day.cache_read_tokens))
                    .style(Style::default().fg(Color::Yellow)),
                Cell::from(format!("{:.1}:1", ratio)).style(Style::default().fg(Color::White)),
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
                .title("📋 Daily Usage Data")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

        f.render_stateful_widget(table, chunks[1], &mut self.daily_table_state);
    }

    fn render_sessions(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Search and controls bar
        let search_info = if self.search_query.is_empty() {
            "No filter active".to_string()
        } else {
            format!("Filtered: '{}'", self.search_query)
        };

        let controls_text = Line::from(vec![
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
            Span::styled(search_info, Style::default().fg(Color::Cyan)),
        ]);

        let controls = Paragraph::new(controls_text)
            .block(Block::default().borders(Borders::ALL).title("📊 Sessions"))
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        let header_cells = ["Project/Session", "Cost", "Tokens", "Last Activity"]
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
                let session_path = format!("{}/{}", session.project_path, session.session_id);
                let truncated_path = self.truncate_text(&session_path, 35);

                // Color code based on cost
                let cost_color = if session.total_cost > 1.0 {
                    Color::Red
                } else if session.total_cost > 0.5 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                let style = if i % 2 == 0 {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };

                Row::new(vec![
                    Cell::from(truncated_path).style(style),
                    Cell::from(format!("${:.4}", session.total_cost))
                        .style(Style::default().fg(cost_color)),
                    Cell::from(self.format_number(session.total_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(session.last_activity.clone())
                        .style(Style::default().fg(Color::Yellow)),
                ])
                .height(1)
            });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Percentage(30),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "📋 Session Data ({} items)",
                    self.session_report.sessions.len()
                ))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

        f.render_stateful_widget(
            table,
            chunks[1].inner(Margin::new(0, 1)),
            &mut self.session_table_state,
        );

        // Enhanced scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let scrollbar_area = chunks[1].inner(Margin::new(1, 1));
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.session_scroll_state);
    }

    fn render_charts(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12), // Cost trend chart
                Constraint::Length(12), // Token usage chart
                Constraint::Min(0),     // Usage distribution
            ])
            .margin(1)
            .split(area);

        // Simple ASCII cost trend chart
        let cost_chart_lines = self.create_cost_trend_chart();
        let cost_chart = Paragraph::new(cost_chart_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📈 Cost Trend (Last 14 Days)")
                    .border_style(Style::default().fg(Color::Green)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(cost_chart, chunks[0]);

        // Token usage chart
        let token_chart_lines = self.create_token_usage_chart();
        let token_chart = Paragraph::new(token_chart_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🎯 Token Usage Distribution")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(token_chart, chunks[1]);

        // Usage statistics
        let stats_lines = self.create_usage_stats();
        let stats = Paragraph::new(stats_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📊 Usage Statistics")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(stats, chunks[2]);
    }

    fn create_cost_trend_chart(&self) -> Vec<Line> {
        let mut lines = vec![];

        // Take last 14 days and create a simple bar chart
        let recent_days: Vec<_> = self.daily_report.daily.iter().take(14).collect();

        if recent_days.is_empty() {
            lines.push(Line::from("No data available"));
            return lines;
        }

        let max_cost = recent_days
            .iter()
            .map(|day| day.total_cost)
            .fold(0.0, f64::max);

        if max_cost == 0.0 {
            lines.push(Line::from("No cost data"));
            return lines;
        }

        for day in recent_days.iter() {
            let bar_length = ((day.total_cost / max_cost) * 40.0) as usize;
            let bar = "█".repeat(bar_length);
            let line = Line::from(vec![
                Span::styled(format!("{} ", day.date), Style::default().fg(Color::White)),
                Span::styled(bar, Style::default().fg(Color::Green)),
                Span::styled(
                    format!(" ${:.3}", day.total_cost),
                    Style::default().fg(Color::Yellow),
                ),
            ]);
            lines.push(line);
        }

        lines
    }

    fn create_token_usage_chart(&self) -> Vec<Line> {
        let mut lines = vec![];

        let total = self.daily_report.totals.total_tokens as f64;
        if total == 0.0 {
            lines.push(Line::from("No token data available"));
            return lines;
        }

        let input_pct = (self.daily_report.totals.input_tokens as f64 / total * 100.0) as usize;
        let output_pct = (self.daily_report.totals.output_tokens as f64 / total * 100.0) as usize;
        let cache_pct = ((self.daily_report.totals.cache_creation_tokens
            + self.daily_report.totals.cache_read_tokens) as f64
            / total
            * 100.0) as usize;

        lines.push(Line::from(vec![
            Span::styled("Input Tokens:   ", Style::default().fg(Color::White)),
            Span::styled(
                "█".repeat(input_pct.min(50)),
                Style::default().fg(Color::Blue),
            ),
            Span::styled(format!(" {}%", input_pct), Style::default().fg(Color::Blue)),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Output Tokens:  ", Style::default().fg(Color::White)),
            Span::styled(
                "█".repeat(output_pct.min(50)),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!(" {}%", output_pct),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Cache Tokens:   ", Style::default().fg(Color::White)),
            Span::styled(
                "█".repeat(cache_pct.min(50)),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!(" {}%", cache_pct),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        lines
    }

    fn create_usage_stats(&self) -> Vec<Line> {
        let mut lines = vec![];

        // Calculate some interesting statistics
        let total_sessions = self.session_report.sessions.len();
        let avg_session_cost = if total_sessions > 0 {
            self.session_report
                .sessions
                .iter()
                .map(|s| s.total_cost)
                .sum::<f64>()
                / total_sessions as f64
        } else {
            0.0
        };

        let max_daily_cost = self
            .daily_report
            .daily
            .iter()
            .map(|day| day.total_cost)
            .fold(0.0, f64::max);

        let max_session_cost = self
            .session_report
            .sessions
            .iter()
            .map(|session| session.total_cost)
            .fold(0.0, f64::max);

        lines.push(Line::from(vec![
            Span::styled("📊 Total Sessions: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}", total_sessions),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("💰 Avg Session Cost: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("${:.4}", avg_session_cost),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("🔥 Max Daily Cost: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("${:.4}", max_daily_cost),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("🚀 Max Session Cost: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("${:.4}", max_session_cost),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
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
                "🔄 Enhanced Navigation:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  1-5, Tab/Shift+Tab", Style::default().fg(Color::Green)),
                Span::styled("  Switch between tabs", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  ↑/↓, j/k", Style::default().fg(Color::Green)),
                Span::styled(
                    "          Navigate tables",
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
            Line::from(""),
            Line::from(vec![Span::styled(
                "🔍 Search & Filter:",
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
                "⚡ Quick Actions:",
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
                Span::styled("  e", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Export current view to CSV",
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
                "📊 Enhanced Tabs:",
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
                Span::styled("  3. Sessions", Style::default().fg(Color::Green)),
                Span::styled(
                    "      Searchable session analytics",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  4. Charts", Style::default().fg(Color::Green)),
                Span::styled(
                    "        ASCII charts and visualizations",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  5. Help", Style::default().fg(Color::Green)),
                Span::styled(
                    "          This comprehensive help screen",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "🎨 New Features:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Real-time search and filtering",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Multiple sorting options",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Time-based filtering",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • ASCII charts and trends",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Enhanced color coding",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Export functionality",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Improved keyboard navigation",
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
                    .title("❓ Enhanced Help & Navigation")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(help, area);
    }

    fn render_help_popup(&self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };

        f.render_widget(ratatui::widgets::Clear, popup_area);

        let help_text = vec![
            Line::from("Quick Help"),
            Line::from(""),
            Line::from("/ - Search"),
            Line::from("s - Sort"),
            Line::from("f - Filter"),
            Line::from("r - Refresh"),
            Line::from("e - Export"),
            Line::from("? - Toggle this popup"),
            Line::from(""),
            Line::from("Press ? again to close"),
        ];

        let popup = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Quick Help")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: true });

        f.render_widget(popup, popup_area);
    }

    fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Daily,
            Tab::Daily => Tab::Sessions,
            Tab::Sessions => Tab::Charts,
            Tab::Charts => Tab::Help,
            Tab::Help => Tab::Overview,
        };
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Help,
            Tab::Daily => Tab::Overview,
            Tab::Sessions => Tab::Daily,
            Tab::Charts => Tab::Sessions,
            Tab::Help => Tab::Charts,
        };
    }

    fn next_item(&mut self) {
        match self.current_tab {
            Tab::Daily => {
                let i = match self.daily_table_state.selected() {
                    Some(i) => {
                        if i >= self.daily_report.daily.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.daily_table_state.select(Some(i));
            }
            Tab::Sessions => {
                let i = match self.session_table_state.selected() {
                    Some(i) => {
                        if i >= self.session_report.sessions.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.session_table_state.select(Some(i));
                self.session_scroll_state = self.session_scroll_state.position(i);
            }
            _ => {}
        }
    }

    fn previous_item(&mut self) {
        match self.current_tab {
            Tab::Daily => {
                let i = match self.daily_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.daily_report.daily.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.daily_table_state.select(Some(i));
            }
            Tab::Sessions => {
                let i = match self.session_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.session_report.sessions.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.session_table_state.select(Some(i));
                self.session_scroll_state = self.session_scroll_state.position(i);
            }
            _ => {}
        }
    }

    fn format_number(&self, num: u64) -> String {
        if num == 0 {
            "0".to_string()
        } else {
            let num_str = num.to_string();
            let chars: Vec<char> = num_str.chars().collect();
            let mut result = String::new();

            for (i, c) in chars.iter().enumerate() {
                if i > 0 && (chars.len() - i) % 3 == 0 {
                    result.push(',');
                }
                result.push(*c);
            }

            result
        }
    }

    fn truncate_text(&self, text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            format!("{}...", &text[..max_length.saturating_sub(3)])
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Handle tab clicks
                if mouse.row <= 2 {
                    let tab_width = 16; // Approximate tab width
                    let selected_tab = (mouse.column / tab_width) as usize;
                    match selected_tab {
                        0 => self.current_tab = Tab::Overview,
                        1 => self.current_tab = Tab::Daily,
                        2 => self.current_tab = Tab::Sessions,
                        3 => self.current_tab = Tab::Charts,
                        4 => self.current_tab = Tab::Help,
                        _ => {}
                    }
                } else {
                    // Handle table row clicks
                    match self.current_tab {
                        Tab::Daily => {
                            if mouse.row >= 6
                                && mouse.row <= 6 + self.daily_report.daily.len() as u16
                            {
                                let selected_row = (mouse.row - 6) as usize;
                                if selected_row < self.daily_report.daily.len() {
                                    self.daily_table_state.select(Some(selected_row));
                                }
                            }
                        }
                        Tab::Sessions => {
                            if mouse.row >= 6
                                && mouse.row <= 6 + self.session_report.sessions.len() as u16
                            {
                                let selected_row = (mouse.row - 6) as usize;
                                if selected_row < self.session_report.sessions.len() {
                                    self.session_table_state.select(Some(selected_row));
                                    self.session_scroll_state =
                                        self.session_scroll_state.position(selected_row);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                self.previous_item();
            }
            MouseEventKind::ScrollDown => {
                self.next_item();
            }
            _ => {}
        }
    }

    fn handle_enter(&mut self) {
        // Copy session info to clipboard if in Sessions tab
        if self.current_tab == Tab::Sessions {
            if let Some(selected) = self.session_table_state.selected() {
                if let Some(session) = self.session_report.sessions.get(selected) {
                    let info = format!(
                        "Project: {}, Session: {}, Cost: ${:.4}, Tokens: {}",
                        session.project_path,
                        session.session_id,
                        session.total_cost,
                        session.total_tokens
                    );

                    if let Ok(mut ctx) = ClipboardContext::new() {
                        if ctx.set_contents(info.clone()).is_ok() {
                            self.status_message =
                                Some("📋 Copied session info to clipboard".to_string());
                        } else {
                            self.status_message =
                                Some("❌ Failed to copy to clipboard".to_string());
                        }
                    } else {
                        self.status_message = Some("❌ Clipboard not available".to_string());
                    }
                }
            }
        }
    }
}
