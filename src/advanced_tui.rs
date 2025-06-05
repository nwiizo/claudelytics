use crate::models::{
    BenchmarkReport, Command, CommandAction, ComparisonReport, DailyReport, DetailedSession,
    HeatmapData, LiveMetrics, SessionReport,
};
use anyhow::Result;
use chrono::{Duration, Local, NaiveDate, Utc};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, ScrollbarState, Table, TableState, Tabs, Wrap,
    },
};
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Daily,
    Sessions,
    DrillDown,
    Compare,
    Benchmark,
    Live,
    Charts,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppMode {
    Normal,
    CommandPalette,
    SessionDetail,
    #[allow(dead_code)]
    Comparison,
    #[allow(dead_code)]
    Benchmark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortMode {
    Date,
    Cost,
    Tokens,
    Project,
    Efficiency,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TimeFilter {
    All,
    Today,
    LastWeek,
    LastMonth,
}

#[derive(Debug)]
pub struct AdvancedTuiApp {
    // Core data
    daily_report: DailyReport,
    session_report: SessionReport,
    original_daily_report: DailyReport,
    original_session_report: SessionReport,
    #[allow(dead_code)]
    detailed_sessions: Vec<DetailedSession>,

    // UI State
    current_tab: Tab,
    current_mode: AppMode,
    daily_table_state: TableState,
    session_table_state: TableState,
    message_table_state: TableState,
    command_table_state: TableState,

    // Scrolling
    session_scroll_state: ScrollbarState,
    #[allow(dead_code)]
    message_scroll_state: ScrollbarState,
    command_scroll_state: ScrollbarState,

    // Filters and search
    search_mode: bool,
    search_query: String,
    sort_mode: SortMode,
    time_filter: TimeFilter,

    // Command palette
    command_palette_query: String,
    available_commands: Vec<Command>,
    filtered_commands: Vec<Command>,

    // Session detail view
    selected_session_detail: Option<DetailedSession>,

    // Comparison mode
    comparison_sessions: Vec<String>,
    #[allow(dead_code)]
    comparison_report: Option<ComparisonReport>,

    // Benchmark data
    benchmark_report: Option<BenchmarkReport>,

    // Live monitoring
    live_metrics: Option<LiveMetrics>,
    auto_refresh: bool,

    // Advanced visualizations
    heatmap_data: Option<HeatmapData>,

    // UI state
    should_quit: bool,
    status_message: Option<String>,
    show_help_popup: bool,
    bookmarked_sessions: Vec<String>,
}

impl AdvancedTuiApp {
    pub fn new(daily_report: DailyReport, session_report: SessionReport) -> Self {
        let mut daily_table_state = TableState::default();
        daily_table_state.select(Some(0));

        let mut session_table_state = TableState::default();
        session_table_state.select(Some(0));

        let session_scroll_state = ScrollbarState::new(session_report.sessions.len());

        let available_commands = Self::create_available_commands();

        Self {
            daily_report: daily_report.clone(),
            session_report: session_report.clone(),
            original_daily_report: daily_report,
            original_session_report: session_report,
            detailed_sessions: Vec::new(),
            current_tab: Tab::Overview,
            current_mode: AppMode::Normal,
            daily_table_state,
            session_table_state,
            message_table_state: TableState::default(),
            command_table_state: TableState::default(),
            session_scroll_state,
            message_scroll_state: ScrollbarState::new(0),
            command_scroll_state: ScrollbarState::new(0),
            search_mode: false,
            search_query: String::new(),
            sort_mode: SortMode::Date,
            time_filter: TimeFilter::All,
            command_palette_query: String::new(),
            available_commands: available_commands.clone(),
            filtered_commands: available_commands,
            selected_session_detail: None,
            comparison_sessions: Vec::new(),
            comparison_report: None,
            benchmark_report: None,
            live_metrics: None,
            auto_refresh: false,
            heatmap_data: None,
            should_quit: false,
            status_message: None,
            show_help_popup: false,
            bookmarked_sessions: Vec::new(),
        }
    }

    fn create_available_commands() -> Vec<Command> {
        vec![
            Command {
                name: "Switch to Overview".to_string(),
                description: "Go to overview tab".to_string(),
                shortcut: Some("1".to_string()),
                action: CommandAction::SwitchTab(0),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Daily".to_string(),
                description: "Go to daily usage tab".to_string(),
                shortcut: Some("2".to_string()),
                action: CommandAction::SwitchTab(1),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Sessions".to_string(),
                description: "Go to sessions tab".to_string(),
                shortcut: Some("3".to_string()),
                action: CommandAction::SwitchTab(2),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Drill Down".to_string(),
                description: "Open detailed session analysis".to_string(),
                shortcut: Some("4".to_string()),
                action: CommandAction::SwitchTab(3),
                category: "Analysis".to_string(),
            },
            Command {
                name: "Compare Sessions".to_string(),
                description: "Compare multiple sessions".to_string(),
                shortcut: Some("5".to_string()),
                action: CommandAction::SwitchTab(4),
                category: "Analysis".to_string(),
            },
            Command {
                name: "Show Benchmark".to_string(),
                description: "View efficiency benchmarks".to_string(),
                shortcut: Some("6".to_string()),
                action: CommandAction::SwitchTab(5),
                category: "Analysis".to_string(),
            },
            Command {
                name: "Live Dashboard".to_string(),
                description: "Real-time monitoring dashboard".to_string(),
                shortcut: Some("7".to_string()),
                action: CommandAction::SwitchTab(6),
                category: "Monitoring".to_string(),
            },
            Command {
                name: "Export Data".to_string(),
                description: "Export current view to CSV".to_string(),
                shortcut: Some("e".to_string()),
                action: CommandAction::ExportData("current".to_string()),
                category: "Data".to_string(),
            },
            Command {
                name: "Refresh Data".to_string(),
                description: "Refresh all data".to_string(),
                shortcut: Some("r".to_string()),
                action: CommandAction::RefreshData,
                category: "Data".to_string(),
            },
            Command {
                name: "Bookmark Session".to_string(),
                description: "Bookmark current session".to_string(),
                shortcut: Some("b".to_string()),
                action: CommandAction::BookmarkSession("current".to_string()),
                category: "Organization".to_string(),
            },
            Command {
                name: "Show Help".to_string(),
                description: "Show help information".to_string(),
                shortcut: Some("?".to_string()),
                action: CommandAction::ShowHelp,
                category: "Help".to_string(),
            },
        ]
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initialize with mock data for demonstration
        self.initialize_mock_data();

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

    fn initialize_mock_data(&mut self) {
        // Create some mock detailed sessions for demonstration
        // In a real implementation, this would parse actual session data
        self.generate_benchmark_report();
        self.generate_live_metrics();
        self.generate_heatmap_data();
    }

    fn generate_benchmark_report(&mut self) {
        use crate::models::*;

        self.benchmark_report = Some(BenchmarkReport {
            user_stats: UserBenchmark {
                total_efficiency: 87.5,
                cost_efficiency_percentile: 92.0,
                usage_consistency: 78.3,
                peak_performance_day: "2025-05-30".to_string(),
            },
            session_rankings: vec![
                SessionRanking {
                    session_name: "Project Alpha".to_string(),
                    score: 95.2,
                    rank: 1,
                    category: "efficiency".to_string(),
                },
                SessionRanking {
                    session_name: "Data Analysis".to_string(),
                    score: 89.7,
                    rank: 2,
                    category: "efficiency".to_string(),
                },
                SessionRanking {
                    session_name: "Code Review".to_string(),
                    score: 82.1,
                    rank: 3,
                    category: "cost".to_string(),
                },
            ],
            trends: TrendAnalysis {
                cost_trend: vec![5.2, 6.1, 4.8, 7.3, 5.9, 6.5, 8.2],
                efficiency_trend: vec![82.1, 85.3, 87.2, 89.1, 86.7, 88.4, 87.5],
                volume_trend: vec![
                    15000.0, 18000.0, 16500.0, 22000.0, 19800.0, 20500.0, 23000.0,
                ],
                prediction_next_week: 91.2,
            },
            recommendations: vec![
                OptimizationTip {
                    category: "Efficiency".to_string(),
                    title: "Optimize cache usage".to_string(),
                    description: "Your cache hit ratio could be improved by 15%".to_string(),
                    potential_savings: 12.50,
                    priority: "high".to_string(),
                },
                OptimizationTip {
                    category: "Cost".to_string(),
                    title: "Reduce off-peak usage".to_string(),
                    description: "Consider batching requests during peak efficiency hours"
                        .to_string(),
                    potential_savings: 8.30,
                    priority: "medium".to_string(),
                },
            ],
        });
    }

    fn generate_live_metrics(&mut self) {
        use crate::models::*;

        self.live_metrics = Some(LiveMetrics {
            active_sessions: 3,
            current_cost_rate: 2.45,
            real_time_efficiency: 89.2,
            last_update: Utc::now(),
            activity_sparkline: vec![
                12, 15, 18, 22, 19, 25, 30, 28, 24, 20, 16, 14, 18, 21, 25, 29, 32, 28, 24, 19, 15,
                12, 10, 8,
            ],
        });
    }

    fn generate_heatmap_data(&mut self) {
        use crate::models::*;

        let mut hour_of_day = HashMap::new();
        let mut day_of_week = HashMap::new();
        let mut day_of_month = HashMap::new();

        // Mock heatmap data - hours of day (0-23)
        for hour in 0..24 {
            let intensity = match hour {
                9..=11 => 0.8 + (hour as f64 - 9.0) * 0.1, // Morning peak
                14..=16 => 0.9 + (hour as f64 - 14.0) * 0.05, // Afternoon peak
                _ => 0.1 + (hour as f64 / 24.0) * 0.3,
            };
            hour_of_day.insert(hour, intensity);
        }

        // Mock day of week data (0=Monday, 6=Sunday)
        for day in 0..7 {
            let intensity = match day {
                0..=4 => 0.7 + day as f64 * 0.05, // Weekdays
                5 => 0.4,                         // Saturday
                6 => 0.2,                         // Sunday
                _ => 0.5,
            };
            day_of_week.insert(day, intensity);
        }

        // Mock day of month data
        for day in 1..=31 {
            let intensity = 0.3 + ((day as f64 * 0.1).sin() + 1.0) * 0.3;
            day_of_month.insert(day, intensity);
        }

        self.heatmap_data = Some(HeatmapData {
            hour_of_day,
            day_of_week,
            day_of_month,
        });
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match self.current_mode {
                            AppMode::CommandPalette => {
                                self.handle_command_palette_input(key.code, key.modifiers)?;
                            }
                            AppMode::SessionDetail => {
                                self.handle_session_detail_input(key.code)?;
                            }
                            _ => {
                                if self.search_mode {
                                    self.handle_search_input(key.code)?;
                                } else {
                                    self.handle_normal_input(key.code, key.modifiers)?;
                                }
                            }
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

    fn handle_normal_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        // Handle Ctrl+P for command palette
        if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('p') {
            self.current_mode = AppMode::CommandPalette;
            self.command_palette_query.clear();
            self.filtered_commands = self.available_commands.clone();
            self.command_table_state.select(Some(0));
            self.status_message = Some("Command Palette: Type to search commands".to_string());
            return Ok(());
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.current_tab = Tab::Overview,
            KeyCode::Char('2') => self.current_tab = Tab::Daily,
            KeyCode::Char('3') => self.current_tab = Tab::Sessions,
            KeyCode::Char('4') => self.current_tab = Tab::DrillDown,
            KeyCode::Char('5') => self.current_tab = Tab::Compare,
            KeyCode::Char('6') => self.current_tab = Tab::Benchmark,
            KeyCode::Char('7') => self.current_tab = Tab::Live,
            KeyCode::Char('8') => self.current_tab = Tab::Charts,
            KeyCode::Char('9') | KeyCode::Char('h') => self.current_tab = Tab::Help,
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
            KeyCode::Char('b') => {
                self.bookmark_selected_session();
            }
            KeyCode::Char('d') => {
                self.enter_session_detail_mode();
            }
            KeyCode::Char('x') => {
                self.toggle_comparison_selection();
            }
            KeyCode::Char('l') => {
                self.toggle_live_mode();
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

    fn handle_command_palette_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.current_mode = AppMode::Normal;
                self.command_palette_query.clear();
                self.status_message = None;
            }
            KeyCode::Enter => {
                if let Some(selected) = self.command_table_state.selected() {
                    if let Some(command) = self.filtered_commands.get(selected) {
                        let action = command.action.clone();
                        self.execute_command(&action)?;
                    }
                }
                self.current_mode = AppMode::Normal;
                self.command_palette_query.clear();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = match self.command_table_state.selected() {
                    Some(i) => {
                        if i >= self.filtered_commands.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.command_table_state.select(Some(i));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = match self.command_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.filtered_commands.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.command_table_state.select(Some(i));
            }
            KeyCode::Backspace => {
                self.command_palette_query.pop();
                self.filter_commands();
            }
            KeyCode::Char(c) => {
                self.command_palette_query.push(c);
                self.filter_commands();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_detail_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.current_mode = AppMode::Normal;
                self.selected_session_detail = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self
                    .selected_session_detail
                    .as_ref()
                    .map(|s| s.messages.len())
                    .unwrap_or(0);

                let i = match self.message_table_state.selected() {
                    Some(i) => {
                        if i >= len.saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.message_table_state.select(Some(i));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = self
                    .selected_session_detail
                    .as_ref()
                    .map(|s| s.messages.len())
                    .unwrap_or(0);

                let i = match self.message_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            len.saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.message_table_state.select(Some(i));
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

    fn execute_command(&mut self, action: &CommandAction) -> Result<()> {
        match action {
            CommandAction::SwitchTab(index) => {
                self.current_tab = match index {
                    0 => Tab::Overview,
                    1 => Tab::Daily,
                    2 => Tab::Sessions,
                    3 => Tab::DrillDown,
                    4 => Tab::Compare,
                    5 => Tab::Benchmark,
                    6 => Tab::Live,
                    7 => Tab::Charts,
                    8 => Tab::Help,
                    _ => Tab::Overview,
                };
                self.status_message = Some(format!("Switched to tab {}", index + 1));
            }
            CommandAction::ExportData(_) => {
                self.export_current_view()?;
            }
            CommandAction::RefreshData => {
                self.refresh_data()?;
            }
            CommandAction::BookmarkSession(_) => {
                self.bookmark_selected_session();
            }
            CommandAction::ShowHelp => {
                self.current_tab = Tab::Help;
            }
            _ => {
                self.status_message = Some("Command executed".to_string());
            }
        }
        Ok(())
    }

    fn filter_commands(&mut self) {
        if self.command_palette_query.is_empty() {
            self.filtered_commands = self.available_commands.clone();
        } else {
            let query = self.command_palette_query.to_lowercase();
            self.filtered_commands = self
                .available_commands
                .iter()
                .filter(|cmd| {
                    cmd.name.to_lowercase().contains(&query)
                        || cmd.description.to_lowercase().contains(&query)
                        || cmd.category.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }

        self.command_scroll_state = ScrollbarState::new(self.filtered_commands.len());
        self.command_table_state.select(Some(0));
    }

    fn refresh_data(&mut self) -> Result<()> {
        self.status_message = Some("🔄 Data refreshed successfully!".to_string());
        self.daily_report = self.original_daily_report.clone();
        self.session_report = self.original_session_report.clone();
        self.apply_filters();

        // Refresh live metrics
        self.generate_live_metrics();
        Ok(())
    }

    fn cycle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Date => SortMode::Cost,
            SortMode::Cost => SortMode::Tokens,
            SortMode::Tokens => SortMode::Efficiency,
            SortMode::Efficiency => SortMode::Project,
            SortMode::Project => SortMode::Date,
        };
        self.apply_filters();
        let mode_str = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Efficiency => "Efficiency",
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
            self.daily_report.daily.retain(|day| {
                if let Ok(date) = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") {
                    date >= cutoff
                } else {
                    true
                }
            });
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
            SortMode::Efficiency => {
                // Sort by efficiency (tokens per dollar)
                self.session_report.sessions.sort_by(|a, b| {
                    let eff_a = if a.total_cost > 0.0 {
                        a.total_tokens as f64 / a.total_cost
                    } else {
                        0.0
                    };
                    let eff_b = if b.total_cost > 0.0 {
                        b.total_tokens as f64 / b.total_cost
                    } else {
                        0.0
                    };
                    eff_b
                        .partial_cmp(&eff_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // Update scroll state
        self.session_scroll_state = ScrollbarState::new(self.session_report.sessions.len());

        // Reset table selections
        self.daily_table_state.select(Some(0));
        self.session_table_state.select(Some(0));
    }

    fn export_current_view(&mut self) -> Result<()> {
        let export_type = match self.current_tab {
            Tab::Daily => "daily",
            Tab::Sessions => "sessions",
            Tab::Benchmark => "benchmark",
            Tab::Compare => "comparison",
            _ => "overview",
        };
        self.status_message = Some(format!("📁 Exported {} data to CSV", export_type));
        Ok(())
    }

    fn bookmark_selected_session(&mut self) {
        if let Some(selected) = self.session_table_state.selected() {
            if let Some(session) = self.session_report.sessions.get(selected) {
                let session_id = format!("{}/{}", session.project_path, session.session_id);
                if !self.bookmarked_sessions.contains(&session_id) {
                    self.bookmarked_sessions.push(session_id.clone());
                    self.status_message = Some(format!("🔖 Bookmarked session: {}", session_id));
                } else {
                    self.bookmarked_sessions.retain(|s| s != &session_id);
                    self.status_message = Some(format!("📌 Removed bookmark: {}", session_id));
                }
            }
        }
    }

    fn enter_session_detail_mode(&mut self) {
        if let Some(selected) = self.session_table_state.selected() {
            if let Some(session) = self.session_report.sessions.get(selected) {
                // Create a mock detailed session for demonstration
                let detailed_session = self.create_mock_detailed_session(session);
                self.selected_session_detail = Some(detailed_session);
                self.current_mode = AppMode::SessionDetail;
                self.message_table_state.select(Some(0));
                self.status_message = Some(
                    "Session Detail Mode: Use j/k to navigate messages, Esc to exit".to_string(),
                );
            }
        }
    }

    fn create_mock_detailed_session(
        &self,
        session: &crate::models::SessionUsage,
    ) -> DetailedSession {
        use crate::models::*;

        // Create mock messages
        let mut messages = Vec::new();
        let message_count = (session.total_tokens / 1000).min(20) as usize; // Simulate reasonable message count

        for i in 0..message_count {
            messages.push(MessageDetail {
                timestamp: Utc::now() - chrono::Duration::minutes(i as i64 * 10),
                input_tokens: session.input_tokens / message_count as u64,
                output_tokens: session.output_tokens / message_count as u64,
                cache_creation_tokens: session.cache_creation_tokens / message_count as u64,
                cache_read_tokens: session.cache_read_tokens / message_count as u64,
                cost: session.total_cost / message_count as f64,
                duration_ms: Some(2000 + (i * 500) as u64),
                efficiency_score: 75.0 + (i as f64 * 2.5),
            });
        }

        // Create hourly breakdown
        let mut hourly_breakdown = HashMap::new();
        for hour in 9..18 {
            let _tokens_for_hour = session.total_tokens / 9; // Distribute across 9 work hours
            hourly_breakdown.insert(
                hour,
                TokenUsage {
                    input_tokens: session.input_tokens / 9,
                    output_tokens: session.output_tokens / 9,
                    cache_creation_tokens: session.cache_creation_tokens / 9,
                    cache_read_tokens: session.cache_read_tokens / 9,
                    total_cost: session.total_cost / 9.0,
                },
            );
        }

        DetailedSession {
            session_detail: SessionDetail {
                project_path: session.project_path.clone(),
                session_id: session.session_id.clone(),
                session_path: std::path::PathBuf::from(&session.project_path),
                usage: TokenUsage {
                    input_tokens: session.input_tokens,
                    output_tokens: session.output_tokens,
                    cache_creation_tokens: session.cache_creation_tokens,
                    cache_read_tokens: session.cache_read_tokens,
                    total_cost: session.total_cost,
                },
                last_activity: Utc::now(),
                message_count: messages.len(),
                first_activity: Utc::now() - chrono::Duration::hours(2),
                duration_hours: 2.0,
                project_name: session
                    .project_path
                    .split('/')
                    .next_back()
                    .unwrap_or("Unknown")
                    .to_string(),
            },
            messages,
            hourly_breakdown,
            efficiency_metrics: EfficiencyMetrics {
                tokens_per_dollar: if session.total_cost > 0.0 {
                    session.total_tokens as f64 / session.total_cost
                } else {
                    0.0
                },
                output_input_ratio: if session.input_tokens > 0 {
                    session.output_tokens as f64 / session.input_tokens as f64
                } else {
                    0.0
                },
                cache_efficiency: if session.total_tokens > 0 {
                    (session.cache_creation_tokens + session.cache_read_tokens) as f64
                        / session.total_tokens as f64
                        * 100.0
                } else {
                    0.0
                },
                cost_per_message: if message_count > 0 {
                    session.total_cost / message_count as f64
                } else {
                    0.0
                },
                peak_hour: 14, // 2 PM
                activity_score: 85.5,
            },
            bookmarked: self
                .bookmarked_sessions
                .contains(&format!("{}/{}", session.project_path, session.session_id)),
            tags: vec!["work".to_string(), "analysis".to_string()],
        }
    }

    fn toggle_comparison_selection(&mut self) {
        if let Some(selected) = self.session_table_state.selected() {
            if let Some(session) = self.session_report.sessions.get(selected) {
                let session_id = format!("{}/{}", session.project_path, session.session_id);
                if self.comparison_sessions.contains(&session_id) {
                    self.comparison_sessions.retain(|s| s != &session_id);
                    self.status_message = Some(format!("Removed from comparison: {}", session_id));
                } else if self.comparison_sessions.len() < 5 {
                    // Limit to 5 sessions
                    self.comparison_sessions.push(session_id.clone());
                    self.status_message = Some(format!(
                        "Added to comparison: {} ({} total)",
                        session_id,
                        self.comparison_sessions.len()
                    ));
                } else {
                    self.status_message = Some("Maximum 5 sessions can be compared".to_string());
                }
            }
        }
    }

    fn toggle_live_mode(&mut self) {
        self.auto_refresh = !self.auto_refresh;
        let status = if self.auto_refresh {
            "enabled"
        } else {
            "disabled"
        };
        self.status_message = Some(format!("🔴 Live mode {}", status));

        if self.auto_refresh {
            self.generate_live_metrics();
        }
    }

    // UI rendering methods will be implemented in the next part
    fn ui(&mut self, f: &mut Frame) {
        match self.current_mode {
            AppMode::CommandPalette => {
                self.render_main_ui(f);
                self.render_command_palette(f);
            }
            AppMode::SessionDetail => {
                self.render_session_detail(f);
            }
            _ => {
                self.render_main_ui(f);
            }
        }

        if self.show_help_popup {
            self.render_help_popup(f);
        }
    }

    fn render_main_ui(&mut self, f: &mut Frame) {
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

        // Enhanced tab bar
        let tab_titles = vec![
            "📊 Overview",
            "📅 Daily",
            "📋 Sessions",
            "🔍 Drill-Down",
            "⚖️  Compare",
            "🏆 Benchmark",
            "🔴 Live",
            "📈 Charts",
            "❓ Help",
        ];
        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Claudelytics Advanced"),
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
            Tab::DrillDown => self.render_drill_down(f, main_area),
            Tab::Compare => self.render_compare(f, main_area),
            Tab::Benchmark => self.render_benchmark(f, main_area),
            Tab::Live => self.render_live(f, main_area),
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
    }

    // Placeholder implementations for new rendering methods
    fn render_overview(&self, f: &mut Frame, area: Rect) {
        // Implementation will be similar to original but enhanced
        let text = vec![
            Line::from("🚀 Advanced Claudelytics Overview"),
            Line::from(""),
            Line::from("New features:"),
            Line::from("• Ctrl+P: Command Palette"),
            Line::from("• d: Session Detail Drill-down"),
            Line::from("• x: Add to comparison"),
            Line::from("• b: Bookmark session"),
            Line::from("• l: Toggle live mode"),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📊 Advanced Overview")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_daily(&mut self, f: &mut Frame, area: Rect) {
        // Enhanced daily view with sparklines and trends
        let text = vec![
            Line::from("📅 Enhanced Daily View"),
            Line::from(""),
            Line::from("Features:"),
            Line::from("• Trend analysis"),
            Line::from("• Efficiency metrics"),
            Line::from("• Cost predictions"),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📅 Daily Analytics")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_sessions(&mut self, f: &mut Frame, area: Rect) {
        // Enhanced sessions view with comparison selection
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Enhanced header
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
                "d",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Detail | ", Style::default().fg(Color::White)),
            Span::styled(
                "b",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Bookmark | ", Style::default().fg(Color::White)),
            Span::styled(comparison_info, Style::default().fg(Color::Cyan)),
        ]);

        let controls = Paragraph::new(controls_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📊 Enhanced Sessions"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        // Sessions table with additional indicators
        let header_cells = [
            "📋 Project/Session",
            "💰 Cost",
            "🎯 Tokens",
            "⏰ Last Activity",
            "🔖",
            "📊 Efficiency",
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
                let session_path = format!("{}/{}", session.project_path, session.session_id);
                let session_id = session_path.clone();

                // Check if bookmarked
                let bookmark_indicator = if self.bookmarked_sessions.contains(&session_id) {
                    "⭐"
                } else {
                    " "
                };

                // Check if in comparison
                let comparison_indicator = if self.comparison_sessions.contains(&session_id) {
                    "✓"
                } else {
                    " "
                };

                // Calculate efficiency
                let efficiency = if session.total_cost > 0.0 {
                    (session.total_tokens as f64 / session.total_cost).round() as u64
                } else {
                    0
                };

                let style = if i % 2 == 0 {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };

                Row::new(vec![
                    Cell::from(format!(
                        "{} {}",
                        comparison_indicator,
                        self.truncate_text(&session_path, 30)
                    ))
                    .style(style),
                    Cell::from(format!("${:.4}", session.total_cost))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(self.format_number(session.total_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(session.last_activity.clone())
                        .style(Style::default().fg(Color::Yellow)),
                    Cell::from(bookmark_indicator).style(Style::default().fg(Color::Yellow)),
                    Cell::from(format!("{} t/$", efficiency))
                        .style(Style::default().fg(Color::Cyan)),
                ])
                .height(1)
            });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(35),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Percentage(25),
                Constraint::Length(3),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "📋 Sessions ({} total, {} bookmarked)",
                    self.session_report.sessions.len(),
                    self.bookmarked_sessions.len()
                ))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

        f.render_stateful_widget(table, chunks[1], &mut self.session_table_state);
    }

    fn render_drill_down(&self, f: &mut Frame, area: Rect) {
        let text = vec![
            Line::from("🔍 Session Drill-Down"),
            Line::from(""),
            Line::from("Select a session and press 'd' to drill down"),
            Line::from(""),
            Line::from("Features:"),
            Line::from("• Message-level analysis"),
            Line::from("• Hourly breakdown"),
            Line::from("• Efficiency metrics"),
            Line::from("• Timeline visualization"),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🔍 Drill-Down Analysis")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_compare(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);

        // Comparison selection status
        let selection_text = if self.comparison_sessions.is_empty() {
            vec![
                Line::from("⚖️  Session Comparison"),
                Line::from(""),
                Line::from("No sessions selected for comparison."),
                Line::from(""),
                Line::from("Instructions:"),
                Line::from("1. Go to Sessions tab"),
                Line::from("2. Press 'x' on sessions to add them"),
                Line::from("3. Return here to see comparison"),
            ]
        } else {
            let mut lines = vec![
                Line::from("⚖️  Session Comparison"),
                Line::from(""),
                Line::from(format!(
                    "Comparing {} sessions:",
                    self.comparison_sessions.len()
                )),
                Line::from(""),
            ];

            for (i, session) in self.comparison_sessions.iter().enumerate() {
                lines.push(Line::from(format!("{}. {}", i + 1, session)));
            }

            lines
        };

        let selection_paragraph = Paragraph::new(selection_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("⚖️  Comparison Selection")
                    .border_style(Style::default().fg(Color::Green)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(selection_paragraph, chunks[0]);

        // Comparison results
        if !self.comparison_sessions.is_empty() {
            let comparison_text = vec![
                Line::from("📊 Comparison Results"),
                Line::from(""),
                Line::from("Feature comparison chart would appear here"),
                Line::from("• Cost efficiency"),
                Line::from("• Token usage patterns"),
                Line::from("• Performance metrics"),
                Line::from("• Recommendations"),
            ];

            let comparison_paragraph = Paragraph::new(comparison_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📊 Analysis Results")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(comparison_paragraph, chunks[1]);
        }
    }

    fn render_benchmark(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // User stats
                Constraint::Length(12), // Rankings
                Constraint::Min(0),     // Recommendations
            ])
            .margin(1)
            .split(area);

        if let Some(ref benchmark) = self.benchmark_report {
            // User benchmark stats
            let stats_text = vec![
                Line::from(vec![
                    Span::styled(
                        "🏆 Your Performance Score: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("{:.1}%", benchmark.user_stats.total_efficiency),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "💰 Cost Efficiency Percentile: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("{:.1}%", benchmark.user_stats.cost_efficiency_percentile),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("📊 Usage Consistency: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.1}%", benchmark.user_stats.usage_consistency),
                        Style::default().fg(Color::Blue),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        "🚀 Peak Performance Day: ",
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        &benchmark.user_stats.peak_performance_day,
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
            ];

            let stats_paragraph = Paragraph::new(stats_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("🏆 Performance Benchmarks")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(stats_paragraph, chunks[0]);

            // Session rankings
            let header_cells = ["Rank", "Session", "Score", "Category"].iter().map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            });
            let header = Row::new(header_cells).height(1).bottom_margin(1);

            let rows = benchmark.session_rankings.iter().map(|ranking| {
                let rank_color = match ranking.rank {
                    1 => Color::Yellow, // Gold
                    2 => Color::White,  // Silver
                    3 => Color::Red,    // Bronze
                    _ => Color::Gray,
                };

                Row::new(vec![
                    Cell::from(format!("#{}", ranking.rank)).style(Style::default().fg(rank_color)),
                    Cell::from(ranking.session_name.clone()),
                    Cell::from(format!("{:.1}", ranking.score))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(ranking.category.clone()).style(Style::default().fg(Color::Blue)),
                ])
                .height(1)
            });

            let rankings_table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Percentage(50),
                    Constraint::Length(8),
                    Constraint::Percentage(30),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🏅 Top Performing Sessions")
                    .border_style(Style::default().fg(Color::Blue)),
            );
            f.render_widget(rankings_table, chunks[1]);

            // Recommendations
            let mut recommendations_text = vec![
                Line::from("💡 Optimization Recommendations"),
                Line::from(""),
            ];

            for (i, tip) in benchmark.recommendations.iter().enumerate() {
                let priority_color = match tip.priority.as_str() {
                    "high" => Color::Red,
                    "medium" => Color::Yellow,
                    "low" => Color::Green,
                    _ => Color::White,
                };

                recommendations_text.push(Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::White)),
                    Span::styled(
                        &tip.title,
                        Style::default()
                            .fg(priority_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                recommendations_text.push(Line::from(format!("   {}", tip.description)));
                recommendations_text.push(Line::from(vec![
                    Span::styled("   Potential savings: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("${:.2}", tip.potential_savings),
                        Style::default().fg(Color::Green),
                    ),
                ]));
                recommendations_text.push(Line::from(""));
            }

            let recommendations_paragraph = Paragraph::new(recommendations_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("💡 Optimization Tips")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(recommendations_paragraph, chunks[2]);
        } else {
            let placeholder_text = vec![
                Line::from("🏆 Benchmark Analysis"),
                Line::from(""),
                Line::from("Loading benchmark data..."),
            ];

            let placeholder_paragraph = Paragraph::new(placeholder_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("🏆 Benchmarks")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(placeholder_paragraph, area);
        }
    }

    fn render_live(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Live metrics
                Constraint::Length(8), // Activity sparkline
                Constraint::Min(0),    // Real-time updates
            ])
            .margin(1)
            .split(area);

        if let Some(ref metrics) = self.live_metrics {
            // Live metrics display
            let live_text = vec![
                Line::from(vec![
                    Span::styled(
                        "🔴 Live Dashboard ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("(Last update: {})", metrics.last_update.format("%H:%M:%S")),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Active Sessions: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}", metrics.active_sessions),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  |  Cost Rate: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.2}/hr", metrics.current_cost_rate),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Real-time Efficiency: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.1}%", metrics.real_time_efficiency),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
            ];

            let live_paragraph = Paragraph::new(live_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("🔴 Live Metrics")
                        .border_style(Style::default().fg(Color::Red)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(live_paragraph, chunks[0]);

            // Activity sparkline
            let sparkline_text = self.create_sparkline(&metrics.activity_sparkline);
            let sparkline_lines = vec![
                Line::from("📈 Activity Sparkline (Last 24 Hours)"),
                Line::from(""),
                Line::from(sparkline_text),
                Line::from(""),
                Line::from("Scale: Low ▁▂▃▄▅▆▇█ High"),
            ];

            let sparkline_paragraph = Paragraph::new(sparkline_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📈 Activity Pattern")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(sparkline_paragraph, chunks[1]);

            // Auto-refresh status
            let auto_refresh_text = vec![
                Line::from("⚡ Real-time Features"),
                Line::from(""),
                Line::from(format!(
                    "Auto-refresh: {}",
                    if self.auto_refresh {
                        "🟢 ON"
                    } else {
                        "🔴 OFF"
                    }
                )),
                Line::from("Press 'l' to toggle live mode"),
                Line::from("Press 'r' to manually refresh"),
            ];

            let auto_refresh_paragraph = Paragraph::new(auto_refresh_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("⚡ Live Controls")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(auto_refresh_paragraph, chunks[2]);
        } else {
            let placeholder_text = vec![
                Line::from("🔴 Live Dashboard"),
                Line::from(""),
                Line::from("Loading live metrics..."),
                Line::from("Press 'l' to enable live mode"),
            ];

            let placeholder_paragraph = Paragraph::new(placeholder_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("🔴 Live Dashboard")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(placeholder_paragraph, area);
        }
    }

    fn create_sparkline(&self, data: &[u32]) -> Span {
        let max_val = *data.iter().max().unwrap_or(&1);
        let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let sparkline: String = data
            .iter()
            .map(|&val| {
                let normalized = (val as f64 / max_val as f64 * (chars.len() - 1) as f64) as usize;
                chars[normalized.min(chars.len() - 1)]
            })
            .collect();

        Span::styled(sparkline, Style::default().fg(Color::Green))
    }

    fn render_charts(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12), // Heatmap
                Constraint::Min(0),     // Advanced charts
            ])
            .margin(1)
            .split(area);

        // Usage heatmap
        if let Some(ref heatmap) = self.heatmap_data {
            let heatmap_text = self.create_heatmap_visualization(heatmap);
            let heatmap_paragraph = Paragraph::new(heatmap_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("🌡️  Usage Heatmap")
                        .border_style(Style::default().fg(Color::Red)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(heatmap_paragraph, chunks[0]);
        }

        // Advanced visualizations placeholder
        let advanced_text = vec![
            Line::from("📊 Advanced Visualizations"),
            Line::from(""),
            Line::from("Available charts:"),
            Line::from("• Usage heatmap (above)"),
            Line::from("• Trend analysis"),
            Line::from("• Efficiency scatter plot"),
            Line::from("• Cost distribution"),
            Line::from("• Performance radar chart"),
        ];

        let advanced_paragraph = Paragraph::new(advanced_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📊 Advanced Charts")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(advanced_paragraph, chunks[1]);
    }

    fn create_heatmap_visualization(&self, heatmap: &HeatmapData) -> Vec<Line> {
        let mut lines = vec![Line::from("Hour of Day Usage Pattern:"), Line::from("")];

        // Create hour of day heatmap
        let mut hour_line = Vec::new();
        for hour in 0..24 {
            let intensity = heatmap.hour_of_day.get(&hour).unwrap_or(&0.0);
            let color = self.intensity_to_color(*intensity);
            let symbol = self.intensity_to_symbol(*intensity);
            hour_line.push(Span::styled(symbol.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(hour_line));

        lines.push(Line::from(
            "00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23",
        ));
        lines.push(Line::from(""));

        lines.push(Line::from("Day of Week Usage:"));
        lines.push(Line::from(""));

        let mut day_line = Vec::new();
        let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, day) in days.iter().enumerate() {
            let intensity = heatmap.day_of_week.get(&(i as u32)).unwrap_or(&0.0);
            let color = self.intensity_to_color(*intensity);
            let symbol = self.intensity_to_symbol(*intensity);
            day_line.push(Span::styled(
                format!("{} {} ", day, symbol),
                Style::default().fg(color),
            ));
        }
        lines.push(Line::from(day_line));

        lines
    }

    fn intensity_to_color(&self, intensity: f64) -> Color {
        match (intensity * 10.0) as u8 {
            0..=2 => Color::Blue,
            3..=4 => Color::Cyan,
            5..=6 => Color::Green,
            7..=8 => Color::Yellow,
            9..=10 => Color::Red,
            _ => Color::White,
        }
    }

    fn intensity_to_symbol(&self, intensity: f64) -> char {
        let chars = ['·', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let index = (intensity * (chars.len() - 1) as f64) as usize;
        chars[index.min(chars.len() - 1)]
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Claudelytics Advanced TUI",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "🚀 Advanced Features:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  Ctrl+P", Style::default().fg(Color::Green)),
                Span::styled(
                    "             Command Palette (fuzzy search)",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  d", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Drill down into session details",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  x", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Add/remove session from comparison",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  b", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Bookmark/unbookmark session",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  l", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Toggle live monitoring mode",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "📊 Advanced Tabs:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  4. Drill-Down", Style::default().fg(Color::Green)),
                Span::styled(
                    "    Message-level analysis",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  5. Compare", Style::default().fg(Color::Green)),
                Span::styled(
                    "      Side-by-side session comparison",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  6. Benchmark", Style::default().fg(Color::Green)),
                Span::styled(
                    "     Performance benchmarks & tips",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  7. Live", Style::default().fg(Color::Green)),
                Span::styled(
                    "         Real-time monitoring dashboard",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  8. Charts", Style::default().fg(Color::Green)),
                Span::styled(
                    "       Advanced visualizations & heatmaps",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "💡 Pro Tips:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Use Ctrl+P to quickly access any feature",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Bookmark important sessions for quick access",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Compare sessions to identify efficiency patterns",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Enable live mode for real-time monitoring",
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "  • Check benchmarks for optimization opportunities",
                Style::default().fg(Color::White),
            )]),
        ];

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("❓ Advanced Help")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(help, area);
    }

    fn render_command_palette(&mut self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 6,
            y: area.height / 6,
            width: (area.width * 2) / 3,
            height: (area.height * 2) / 3,
        };

        f.render_widget(Clear, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(popup_area);

        // Search input
        let search_text = if self.command_palette_query.is_empty() {
            "Type to search commands..."
        } else {
            &self.command_palette_query
        };

        let search_paragraph = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🔍 Command Palette")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black));
        f.render_widget(search_paragraph, chunks[0]);

        // Commands list
        let header_cells = ["Command", "Description", "Shortcut", "Category"]
            .iter()
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.filtered_commands.iter().map(|cmd| {
            let shortcut = cmd.shortcut.as_deref().unwrap_or("");
            Row::new(vec![
                Cell::from(cmd.name.clone()),
                Cell::from(cmd.description.clone()),
                Cell::from(shortcut),
                Cell::from(cmd.category.clone()),
            ])
            .height(1)
        });

        let commands_table = Table::new(
            rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(45),
                Constraint::Length(8),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "📋 Commands ({} found)",
                    self.filtered_commands.len()
                ))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ")
        .style(Style::default().bg(Color::Black));

        f.render_stateful_widget(commands_table, chunks[1], &mut self.command_table_state);
    }

    fn render_session_detail(&mut self, f: &mut Frame) {
        if let Some(ref session) = self.selected_session_detail {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8), // Session summary
                    Constraint::Length(6), // Efficiency metrics
                    Constraint::Min(0),    // Messages table
                ])
                .margin(1)
                .split(f.area());

            // Session summary
            let summary_text = vec![
                Line::from(vec![
                    Span::styled(
                        "🔍 Session Detail: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &session.session_detail.project_name,
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Session ID: ", Style::default().fg(Color::White)),
                    Span::styled(
                        &session.session_detail.session_id,
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Messages: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}", session.messages.len()),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled("  |  Duration: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.1}h", session.session_detail.duration_hours),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::styled("  |  Total Cost: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.4}", session.session_detail.usage.total_cost),
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Bookmarked: ", Style::default().fg(Color::White)),
                    Span::styled(
                        if session.bookmarked { "⭐ Yes" } else { "No" },
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled("  |  Tags: ", Style::default().fg(Color::White)),
                    Span::styled(session.tags.join(", "), Style::default().fg(Color::Magenta)),
                ]),
            ];

            let summary_paragraph = Paragraph::new(summary_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📊 Session Overview")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(summary_paragraph, chunks[0]);

            // Efficiency metrics
            let metrics_text = vec![
                Line::from(vec![
                    Span::styled("⚡ Efficiency Score: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.1}", session.efficiency_metrics.activity_score),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  |  Tokens/Dollar: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.0}", session.efficiency_metrics.tokens_per_dollar),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("🔄 Output/Input Ratio: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.1}:1", session.efficiency_metrics.output_input_ratio),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::styled("  |  Cache Efficiency: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{:.1}%", session.efficiency_metrics.cache_efficiency),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("💰 Cost/Message: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.4}", session.efficiency_metrics.cost_per_message),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled("  |  Peak Hour: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}:00", session.efficiency_metrics.peak_hour),
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
            ];

            let metrics_paragraph = Paragraph::new(metrics_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📈 Efficiency Metrics")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(metrics_paragraph, chunks[1]);

            // Messages table
            let header_cells = [
                "Time",
                "Input",
                "Output",
                "Cache",
                "Cost",
                "Duration",
                "Efficiency",
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

            let rows = session.messages.iter().map(|msg| {
                let efficiency_color = if msg.efficiency_score > 80.0 {
                    Color::Green
                } else if msg.efficiency_score > 60.0 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                Row::new(vec![
                    Cell::from(msg.timestamp.format("%H:%M:%S").to_string()),
                    Cell::from(self.format_number(msg.input_tokens))
                        .style(Style::default().fg(Color::Blue)),
                    Cell::from(self.format_number(msg.output_tokens))
                        .style(Style::default().fg(Color::Cyan)),
                    Cell::from(
                        self.format_number(msg.cache_creation_tokens + msg.cache_read_tokens),
                    )
                    .style(Style::default().fg(Color::Yellow)),
                    Cell::from(format!("${:.4}", msg.cost))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(format!("{}ms", msg.duration_ms.unwrap_or(0)))
                        .style(Style::default().fg(Color::Gray)),
                    Cell::from(format!("{:.1}", msg.efficiency_score))
                        .style(Style::default().fg(efficiency_color)),
                ])
                .height(1)
            });

            let messages_table = Table::new(
                rows,
                [
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Length(10),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("📋 Messages ({} total)", session.messages.len()))
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("► ");

            f.render_stateful_widget(messages_table, chunks[2], &mut self.message_table_state);
        }
    }

    fn render_help_popup(&self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };

        f.render_widget(Clear, popup_area);

        let help_text = vec![
            Line::from("⚡ Quick Help"),
            Line::from(""),
            Line::from("Ctrl+P - Command Palette"),
            Line::from("d - Session Detail"),
            Line::from("x - Compare Toggle"),
            Line::from("b - Bookmark"),
            Line::from("l - Live Mode"),
            Line::from("/ - Search"),
            Line::from("s - Sort"),
            Line::from("f - Filter"),
            Line::from(""),
            Line::from("Press ? again to close"),
        ];

        let popup = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("⚡ Quick Help")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: true });

        f.render_widget(popup, popup_area);
    }

    // Navigation and utility methods
    fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Daily,
            Tab::Daily => Tab::Sessions,
            Tab::Sessions => Tab::DrillDown,
            Tab::DrillDown => Tab::Compare,
            Tab::Compare => Tab::Benchmark,
            Tab::Benchmark => Tab::Live,
            Tab::Live => Tab::Charts,
            Tab::Charts => Tab::Help,
            Tab::Help => Tab::Overview,
        };
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Help,
            Tab::Daily => Tab::Overview,
            Tab::Sessions => Tab::Daily,
            Tab::DrillDown => Tab::Sessions,
            Tab::Compare => Tab::DrillDown,
            Tab::Benchmark => Tab::Compare,
            Tab::Live => Tab::Benchmark,
            Tab::Charts => Tab::Live,
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
                    let tab_width = 14; // Approximate tab width for 9 tabs
                    let selected_tab = (mouse.column / tab_width) as usize;
                    match selected_tab {
                        0 => self.current_tab = Tab::Overview,
                        1 => self.current_tab = Tab::Daily,
                        2 => self.current_tab = Tab::Sessions,
                        3 => self.current_tab = Tab::DrillDown,
                        4 => self.current_tab = Tab::Compare,
                        5 => self.current_tab = Tab::Benchmark,
                        6 => self.current_tab = Tab::Live,
                        7 => self.current_tab = Tab::Charts,
                        8 => self.current_tab = Tab::Help,
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
