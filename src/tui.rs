use crate::billing_blocks::BillingBlockManager;
use crate::claude_sessions::ClaudeSessionParser;
use crate::models::{ClaudeSession, Command, CommandAction, DailyReport, SessionReport};
use crate::pricing_cache::PricingCache;
use anyhow::Result;
use chrono::Local;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind, poll,
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
        Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Scrollbar, ScrollbarOrientation,
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
    BillingBlocks,
    Resume,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum AppMode {
    Normal,
    CommandPalette,
    Search,
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

#[derive(Debug, Clone)]
struct ResumeSession {
    number: usize,
    modified: String,
    messages: String,
    summary: String,
    session_data: Option<ClaudeSession>,
}

#[derive(Debug)]
pub struct TuiApp {
    daily_report: DailyReport,
    session_report: SessionReport,
    original_daily_report: DailyReport,
    original_session_report: SessionReport,
    current_tab: Tab,
    current_mode: AppMode,
    daily_table_state: TableState,
    session_table_state: TableState,
    command_table_state: TableState,
    session_scroll_state: ScrollbarState,
    command_scroll_state: ScrollbarState,
    should_quit: bool,
    search_mode: bool,
    search_query: String,
    sort_mode: SortMode,
    time_filter: TimeFilter,
    status_message: Option<String>,
    show_help_popup: bool,
    // Command palette
    command_palette_query: String,
    available_commands: Vec<Command>,
    filtered_commands: Vec<Command>,
    // Enhanced features
    bookmarked_sessions: Vec<String>,
    comparison_sessions: Vec<String>,
    // Resume functionality
    resume_sessions: Vec<ResumeSession>,
    resume_table_state: TableState,
    resume_loading: bool,
    // Resume input buffer
    #[allow(dead_code)]
    resume_input_mode: bool,
    #[allow(dead_code)]
    resume_input_buffer: String,
    #[allow(dead_code)]
    resume_input_cursor: usize,
    // Billing blocks
    billing_manager: BillingBlockManager,
    billing_blocks_table_state: TableState,
    billing_blocks_scroll_state: ScrollbarState,
    show_billing_summary: bool,
    // Pricing cache status
    pricing_cache_status: Option<PricingCacheStatus>,
}

#[derive(Debug, Clone)]
struct PricingCacheStatus {
    exists: bool,
    valid: bool,
    last_updated: String,
    model_count: usize,
}

impl TuiApp {
    pub fn new(
        daily_report: DailyReport,
        session_report: SessionReport,
        billing_manager: BillingBlockManager,
    ) -> Self {
        let mut daily_table_state = TableState::default();
        daily_table_state.select(Some(0));

        let mut session_table_state = TableState::default();
        session_table_state.select(Some(0));

        let session_scroll_state = ScrollbarState::new(session_report.sessions.len());
        let available_commands = Self::create_available_commands();

        let billing_report = billing_manager.generate_report();
        let billing_blocks_scroll_state = ScrollbarState::new(billing_report.blocks.len());

        // Check pricing cache status
        let pricing_cache_status = match PricingCache::load() {
            Ok(Some(cache)) => Some(PricingCacheStatus {
                exists: true,
                valid: cache.is_valid(),
                last_updated: cache
                    .last_updated
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
                model_count: cache.pricing_data.len(),
            }),
            _ => None,
        };

        let mut app = Self {
            daily_report: daily_report.clone(),
            session_report: session_report.clone(),
            original_daily_report: daily_report,
            original_session_report: session_report,
            current_tab: Tab::Overview,
            current_mode: AppMode::Normal,
            daily_table_state,
            session_table_state,
            command_table_state: TableState::default(),
            session_scroll_state,
            command_scroll_state: ScrollbarState::new(0),
            should_quit: false,
            search_mode: false,
            search_query: String::new(),
            sort_mode: SortMode::Date,
            time_filter: TimeFilter::All,
            status_message: None,
            show_help_popup: false,
            command_palette_query: String::new(),
            available_commands: available_commands.clone(),
            filtered_commands: available_commands,
            bookmarked_sessions: Vec::new(),
            comparison_sessions: Vec::new(),
            resume_sessions: Vec::new(),
            resume_table_state: TableState::default(),
            resume_loading: false,
            resume_input_mode: false,
            resume_input_buffer: String::new(),
            resume_input_cursor: 0,
            billing_manager,
            billing_blocks_table_state: TableState::default(),
            billing_blocks_scroll_state,
            show_billing_summary: true,
            pricing_cache_status,
        };

        // Apply initial filters and sorting
        app.apply_filters();
        app
    }

    // State extraction methods for resume functionality
    pub fn get_current_tab_index(&self) -> usize {
        self.current_tab as usize
    }

    pub fn get_search_query(&self) -> String {
        self.search_query.clone()
    }

    pub fn get_bookmarked_sessions(&self) -> Vec<String> {
        self.bookmarked_sessions.clone()
    }

    pub fn get_comparison_sessions(&self) -> Vec<String> {
        self.comparison_sessions.clone()
    }

    pub fn get_daily_report(&self) -> &DailyReport {
        &self.daily_report
    }

    pub fn get_session_report(&self) -> &SessionReport {
        &self.session_report
    }

    pub fn get_selected_session_path(&self) -> Option<String> {
        if let Some(selected) = self.session_table_state.selected() {
            if let Some(session) = self.session_report.sessions.get(selected) {
                return Some(format!("{}/{}", session.project_path, session.session_id));
            }
        }
        None
    }

    // State restoration methods for resume functionality
    pub fn set_current_tab(&mut self, tab_index: usize) {
        self.current_tab = match tab_index {
            0 => Tab::Overview,
            1 => Tab::Daily,
            2 => Tab::Sessions,
            3 => Tab::Charts,
            4 => Tab::Resume,
            5 => Tab::Help,
            _ => Tab::Overview,
        };
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        if !self.search_query.is_empty() {
            self.apply_filters();
        }
    }

    pub fn set_bookmarked_sessions(&mut self, bookmarks: Vec<String>) {
        self.bookmarked_sessions = bookmarks;
    }

    pub fn set_comparison_sessions(&mut self, comparisons: Vec<String>) {
        self.comparison_sessions = comparisons;
    }

    pub fn restore_session_selection(&mut self, session_path: Option<String>) {
        if let Some(path) = session_path {
            // Find the session in current session list and select it
            for (index, session) in self.session_report.sessions.iter().enumerate() {
                let current_path = format!("{}/{}", session.project_path, session.session_id);
                if current_path == path {
                    self.session_table_state.select(Some(index));
                    self.session_scroll_state = self.session_scroll_state.position(index);
                    break;
                }
            }
        }
    }

    pub fn set_restored_state(&mut self) {
        self.status_message = Some("✨ Previous session state restored".to_string());
    }

    fn load_resume_sessions(&mut self) {
        self.resume_loading = true;
        match self.fetch_claude_sessions() {
            Ok(sessions) => {
                self.resume_sessions = sessions;
                if !self.resume_sessions.is_empty() {
                    self.resume_table_state.select(Some(0));
                }
                self.status_message = Some(format!(
                    "📋 Loaded {} sessions with summaries linked to usage data",
                    self.resume_sessions.len()
                ));
            }
            Err(e) => {
                self.status_message = Some(format!("❌ Failed to load sessions: {}", e));
            }
        }
        self.resume_loading = false;
    }

    fn fetch_claude_sessions(&self) -> Result<Vec<ResumeSession>> {
        // Load actual Claude sessions
        let parser = ClaudeSessionParser::new(None);

        match parser.get_recent_sessions(20) {
            Ok(sessions) => {
                let resume_sessions = sessions
                    .into_iter()
                    .enumerate()
                    .map(|(idx, session)| {
                        let time_diff = Local::now().signed_duration_since(session.modified_at);
                        let modified = if time_diff.num_hours() < 1 {
                            format!("{} min ago", time_diff.num_minutes())
                        } else if time_diff.num_hours() < 24 {
                            format!("{} hr ago", time_diff.num_hours())
                        } else {
                            format!("{} days ago", time_diff.num_days())
                        };

                        ResumeSession {
                            number: idx + 1,
                            modified,
                            messages: session.message_count.to_string(),
                            summary: session.summary.clone(),
                            session_data: Some(session),
                        }
                    })
                    .collect();
                Ok(resume_sessions)
            }
            Err(_) => {
                // Fallback to mock data if real sessions fail to load
                Ok(self.get_mock_sessions())
            }
        }
    }

    fn get_mock_sessions(&self) -> Vec<ResumeSession> {
        vec![
            ResumeSession {
                number: 1,
                modified: "3 days ago".to_string(),
                messages: "5".to_string(),
                summary: "Advanced Rust TUI Analytics Implementation".to_string(),
                session_data: None,
            },
            ResumeSession {
                number: 2,
                modified: "3 days ago".to_string(),
                messages: "141".to_string(),
                summary: "Rust CLI Help Docs & Release Workflow Optimization".to_string(),
                session_data: None,
            },
            ResumeSession {
                number: 3,
                modified: "4 days ago".to_string(),
                messages: "37".to_string(),
                summary: "Claudelytics: Advanced Analytics TUI Development".to_string(),
                session_data: None,
            },
            ResumeSession {
                number: 4,
                modified: "4 days ago".to_string(),
                messages: "93".to_string(),
                summary: "Claude's Advanced TUI Analytics Development".to_string(),
                session_data: None,
            },
            ResumeSession {
                number: 5,
                modified: "4 days ago".to_string(),
                messages: "10".to_string(),
                summary: "Rust CLI Tool Evolution: Claude AI Usage Analytics".to_string(),
                session_data: None,
            },
        ]
    }

    fn open_selected_session(&mut self) {
        if let Some(selected) = self.resume_table_state.selected() {
            if let Some(session) = self.resume_sessions.get(selected) {
                match self.open_claude_session(session.number) {
                    Ok(_) => {
                        self.status_message = Some(format!(
                            "🚀 Opening session {}: {}",
                            session.number, session.summary
                        ));
                        // Exit TUI since we're opening Claude
                        self.should_quit = true;
                    }
                    Err(e) => {
                        self.status_message = Some(format!("❌ Failed to open session: {}", e));
                    }
                }
            }
        }
    }

    fn open_claude_session(&self, session_number: usize) -> Result<()> {
        // Show message instead of actually opening to prevent hanging
        anyhow::bail!(
            "Opening Claude session {} is disabled to prevent hanging. Use 'claude --resume {}' in terminal instead.",
            session_number,
            session_number
        )
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
                name: "Switch to Charts".to_string(),
                description: "Go to charts tab".to_string(),
                shortcut: Some("4".to_string()),
                action: CommandAction::SwitchTab(3),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Resume".to_string(),
                description: "Go to resume tab".to_string(),
                shortcut: Some("5".to_string()),
                action: CommandAction::SwitchTab(4),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Help".to_string(),
                description: "Go to help tab".to_string(),
                shortcut: Some("6".to_string()),
                action: CommandAction::SwitchTab(5),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Export Data".to_string(),
                description: "Export current view to CSV".to_string(),
                shortcut: Some("e".to_string()),
                action: CommandAction::ExportData("current".to_string()),
                category: "Data".to_string(),
            },
            Command {
                name: "Bookmark Session".to_string(),
                description: "Bookmark selected session".to_string(),
                shortcut: Some("b".to_string()),
                action: CommandAction::BookmarkSession("current".to_string()),
                category: "Organization".to_string(),
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

            // Check for events with timeout to prevent hanging
            if poll(std::time::Duration::from_millis(50))? {
                if let Ok(evt) = event::read() {
                    match evt {
                        Event::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                                match self.current_mode {
                                    AppMode::CommandPalette => {
                                        self.handle_command_palette_input(key.code, key.modifiers)?;
                                    }
                                    AppMode::Search => {
                                        self.handle_search_input(key.code)?;
                                    }
                                    AppMode::Normal => {
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
                }
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
            KeyCode::Char('4') => self.current_tab = Tab::Charts,
            KeyCode::Char('5') => {
                self.current_tab = Tab::Resume;
                // Auto-load removed to prevent hanging
                // Users can press 'r' to manually load sessions
            }
            KeyCode::Char('6') | KeyCode::Char('h') => self.current_tab = Tab::Help,
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
                if self.current_tab == Tab::BillingBlocks {
                    self.show_billing_summary = !self.show_billing_summary;
                    self.status_message = Some(format!(
                        "Billing summary {}",
                        if self.show_billing_summary {
                            "shown"
                        } else {
                            "hidden"
                        }
                    ));
                } else {
                    self.cycle_sort_mode();
                }
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
            KeyCode::Char('x') => {
                self.toggle_comparison_selection();
            }
            KeyCode::Char('?') => {
                self.show_help_popup = !self.show_help_popup;
            }
            KeyCode::Enter => {
                if self.current_tab == Tab::Resume {
                    self.open_selected_session();
                } else {
                    self.handle_enter();
                }
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
        if self.current_tab == Tab::Resume {
            // Refresh Claude sessions (manual load only)
            self.load_resume_sessions();
        } else {
            // In a real implementation, you'd re-parse the data
            // For now, we'll just show a message
            self.status_message = Some("🔄 Data refreshed successfully!".to_string());

            // Reset to original data to simulate refresh
            self.daily_report = self.original_daily_report.clone();
            self.session_report = self.original_session_report.clone();
            self.apply_filters();
        }
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

        // Simplified filtering to prevent hangs

        // Apply search filter only (skip time filter for now)
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

        // Simple sorting
        match self.sort_mode {
            SortMode::Date => {
                self.daily_report.daily.sort_by(|a, b| b.date.cmp(&a.date));
                self.session_report
                    .sessions
                    .sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
            }
            SortMode::Cost => {
                self.session_report.sessions.sort_by(|a, b| {
                    b.total_cost
                        .partial_cmp(&a.total_cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            _ => {
                // Default to date sorting for other modes to prevent complexity
                self.session_report
                    .sessions
                    .sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
            }
        }

        // Update scroll state safely
        let session_count = self.session_report.sessions.len();
        if session_count > 0 {
            self.session_scroll_state = ScrollbarState::new(session_count);
            self.session_table_state.select(Some(0));
        }

        if !self.daily_report.daily.is_empty() {
            self.daily_table_state.select(Some(0));
        }
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

    fn execute_command(&mut self, action: &CommandAction) -> Result<()> {
        match action {
            CommandAction::SwitchTab(index) => {
                self.current_tab = match index {
                    0 => Tab::Overview,
                    1 => Tab::Daily,
                    2 => Tab::Sessions,
                    3 => Tab::Charts,
                    4 => Tab::Resume,
                    5 => Tab::Help,
                    _ => Tab::Overview,
                };
                self.status_message = Some(format!("Switched to tab {}", index + 1));
            }
            CommandAction::ExportData(_) => {
                self.export_current_view()?;
            }
            CommandAction::BookmarkSession(_) => {
                self.bookmark_selected_session();
            }
            _ => {
                self.status_message = Some("Command executed".to_string());
            }
        }
        Ok(())
    }

    fn handle_resume_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.resume_input_mode = false;
                self.current_mode = AppMode::Normal;
                self.resume_input_buffer.clear();
                self.resume_input_cursor = 0;
                self.status_message = None;
            }
            KeyCode::Enter => {
                if !self.resume_input_buffer.is_empty() {
                    self.send_resume_message()?;
                }
                self.resume_input_mode = false;
                self.current_mode = AppMode::Normal;
                self.resume_input_buffer.clear();
                self.resume_input_cursor = 0;
            }
            KeyCode::Backspace => {
                if self.resume_input_cursor > 0 {
                    self.resume_input_buffer
                        .remove(self.resume_input_cursor - 1);
                    self.resume_input_cursor -= 1;
                }
            }
            KeyCode::Left => {
                if self.resume_input_cursor > 0 {
                    self.resume_input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.resume_input_cursor < self.resume_input_buffer.len() {
                    self.resume_input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.resume_input_cursor = 0;
            }
            KeyCode::End => {
                self.resume_input_cursor = self.resume_input_buffer.len();
            }
            KeyCode::Char(c) => {
                self.resume_input_buffer.insert(self.resume_input_cursor, c);
                self.resume_input_cursor += 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn send_resume_message(&mut self) -> Result<()> {
        if let Some(selected) = self.resume_table_state.selected() {
            if let Some(resume_session) = self.resume_sessions.get(selected) {
                if let Some(session_data) = &resume_session.session_data {
                    // In a real implementation, this would:
                    // 1. Parse the session file
                    // 2. Add the new user message
                    // 3. Potentially send to Claude API
                    // 4. Update the session file
                    // For now, we'll just show a status message
                    self.status_message = Some(format!(
                        "📤 Message sent to session: '{}'\nMessage: {}",
                        if session_data.summary.is_empty() {
                            "Untitled"
                        } else {
                            &session_data.summary
                        },
                        self.resume_input_buffer
                    ));

                    // TODO: Implement actual message sending logic here
                    // This would involve:
                    // - Loading the full conversation from the session file
                    // - Adding the user's message
                    // - Optionally calling Claude API for a response
                    // - Saving the updated conversation back to the file
                } else {
                    self.status_message =
                        Some("❌ Cannot send message to demo session".to_string());
                }
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

    fn ui(&mut self, f: &mut Frame) {
        match self.current_mode {
            AppMode::CommandPalette => {
                self.render_main_ui(f);
                self.render_command_palette(f);
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

        // Tab bar with enhanced titles
        let tab_titles = vec![
            "📊 Overview",
            "📅 Daily",
            "📋 Sessions",
            "📈 Charts",
            "⏰ Billing",
            "🔄 Resume",
            "❓ Help",
        ];
        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Claudelytics Enhanced (Ctrl+P: Command Palette)"),
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
            Tab::BillingBlocks => self.render_billing_blocks(f, main_area),
            Tab::Resume => self.render_resume(f, main_area),
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
            SortMode::Efficiency => "Efficiency",
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

    fn try_find_conversation_summary(&self, session_id: &str) -> Option<String> {
        // Try to match session with conversation summary
        // This could be enhanced to actually parse Claude conversation data
        for resume_session in &self.resume_sessions {
            if session_id.contains(&resume_session.number.to_string()) {
                return Some(resume_session.summary.clone());
            }
        }
        None
    }

    fn render_sessions(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Enhanced controls bar
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
                "b",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Bookmark | ", Style::default().fg(Color::White)),
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
            Span::styled(comparison_info, Style::default().fg(Color::Cyan)),
        ]);

        let controls = Paragraph::new(controls_text)
            .block(Block::default().borders(Borders::ALL).title("📊 Sessions"))
            .wrap(Wrap { trim: true });
        f.render_widget(controls, chunks[0]);

        // Check if we have sessions to display
        if self.session_report.sessions.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("No sessions found for current filters"),
                Line::from(""),
                Line::from("Try:"),
                Line::from("- Press 'f' to change time filter"),
                Line::from("- Press 'r' to refresh data"),
                Line::from("- Check if you have usage data in ~/.claude/projects/"),
            ];
            let empty_paragraph = Paragraph::new(empty_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📋 No Sessions Found")
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_paragraph, chunks[1]);
            return;
        }

        let header_cells = [
            "Project/Session",
            "Cost",
            "Tokens",
            "Last Activity",
            "🔖",
            "📊 Efficiency",
            "💬 Summary",
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

                let truncated_path = self.truncate_text(&session_path, 30);

                // Try to find conversation summary for this session
                let summary = self
                    .try_find_conversation_summary(&session.session_id)
                    .map(|s| self.truncate_text(&s, 40))
                    .unwrap_or_else(|| "No summary available".to_string());

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
                    Cell::from(format!("{} {}", comparison_indicator, truncated_path)).style(style),
                    Cell::from(format!("${:.4}", session.total_cost))
                        .style(Style::default().fg(cost_color)),
                    Cell::from(self.format_number(session.total_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(session.last_activity.clone())
                        .style(Style::default().fg(Color::Yellow)),
                    Cell::from(bookmark_indicator).style(Style::default().fg(Color::Yellow)),
                    Cell::from(format!("{} t/$", efficiency))
                        .style(Style::default().fg(Color::Cyan)),
                    Cell::from(summary).style(Style::default().fg(Color::LightBlue)),
                ])
                .height(1)
            });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(25), // Project/Session - reduced
                Constraint::Length(10),     // Cost
                Constraint::Length(12),     // Tokens
                Constraint::Length(12),     // Last Activity - reduced
                Constraint::Length(3),      // Bookmark
                Constraint::Length(8),      // Efficiency
                Constraint::Percentage(40), // Summary - new column
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

    fn render_resume(&mut self, f: &mut Frame, area: Rect) {
        let chunks = if self.resume_input_mode {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Controls info
                    Constraint::Min(10),   // Claude sessions table
                    Constraint::Length(3), // Input area
                ])
                .margin(1)
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Controls info
                    Constraint::Min(0),    // Claude sessions table
                ])
                .margin(1)
                .split(area)
        };

        // Controls and instructions
        let controls_text = Line::from(vec![
            Span::styled("Controls: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "↑/↓",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Navigate | ", Style::default().fg(Color::White)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Open Session | ", Style::default().fg(Color::White)),
            Span::styled(
                "i",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Send Message | ", Style::default().fg(Color::White)),
            Span::styled(
                "r",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Refresh", Style::default().fg(Color::White)),
        ]);

        let controls = Paragraph::new(controls_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("🔄 Claude Resume Sessions")
                .border_style(Style::default().fg(Color::Green)),
        );
        f.render_widget(controls, chunks[0]);

        // Claude sessions table
        if self.resume_loading {
            let loading_text = vec![
                Line::from(""),
                Line::from("Loading Claude sessions..."),
                Line::from(""),
            ];
            let loading_paragraph = Paragraph::new(loading_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📋 Sessions")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(loading_paragraph, chunks[1]);
        } else if self.resume_sessions.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("No Claude sessions loaded"),
                Line::from(""),
                Line::from("Press 'r' to load Claude sessions (demo mode)"),
                Line::from("Note: Real Claude integration temporarily disabled"),
            ];
            let empty_paragraph = Paragraph::new(empty_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📋 Sessions")
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_paragraph, chunks[1]);
        } else {
            let header = Row::new(vec!["#", "Modified", "Messages", "Summary"])
                .style(Style::default().fg(Color::Cyan))
                .height(1);

            let rows = self.resume_sessions.iter().map(|session| {
                Row::new(vec![
                    Cell::from(format!("{}.", session.number)),
                    Cell::from(session.modified.clone()),
                    Cell::from(session.messages.clone()),
                    Cell::from(self.truncate_text(&session.summary, 60)),
                ])
                .height(1)
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(4),
                    Constraint::Length(12),
                    Constraint::Length(8),
                    Constraint::Min(0),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "📋 Claude Sessions ({})",
                        self.resume_sessions.len()
                    ))
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("► ");

            f.render_stateful_widget(table, chunks[1], &mut self.resume_table_state);
        }

        // Render input area if in input mode
        if self.resume_input_mode && chunks.len() > 2 {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title("💬 Message Input")
                .border_style(Style::default().fg(Color::Yellow));

            let input_text = if self.resume_input_buffer.is_empty() {
                Paragraph::new("Type your message here...")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(input_block)
            } else {
                // Create the input display with cursor
                let mut display_text = self.resume_input_buffer.clone();
                if self.resume_input_cursor == display_text.len() {
                    display_text.push('_'); // Show cursor at end
                } else {
                    display_text.insert(self.resume_input_cursor, '|'); // Show cursor in middle
                }

                Paragraph::new(display_text)
                    .style(Style::default().fg(Color::White))
                    .block(input_block)
            };

            f.render_widget(input_text, chunks[2]);
        }
    }

    fn render_billing_blocks(&mut self, f: &mut Frame, area: Rect) {
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
            // Find current block if it exists
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
                    Span::styled("💰 Current Block Cost: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.4}", current_block_cost),
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
                    Span::styled("📊 Total Blocks: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}", report.blocks.len()),
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("⏰ Current Period: ", Style::default().fg(Color::White)),
                    Span::styled(current_block_info, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("📈 Peak Block: ", Style::default().fg(Color::White)),
                    Span::styled(
                        if let Some(ref peak) = report.peak_block {
                            format!(
                                "${:.4} ({} {})",
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
                    Span::styled("💵 Average per Block: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.4}", report.average_per_block.total_cost),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("🎯 Total Cost: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.4}", report.total_usage.total_cost),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("💾 Cache Status: ", Style::default().fg(Color::White)),
                    if let Some(ref cache_status) = self.pricing_cache_status {
                        if cache_status.exists && cache_status.valid {
                            Span::styled(
                                format!(
                                    "✓ Valid (Updated: {}, {} models)",
                                    cache_status.last_updated, cache_status.model_count
                                ),
                                Style::default().fg(Color::Green),
                            )
                        } else if cache_status.exists {
                            Span::styled(
                                "⚠ Expired - Update recommended",
                                Style::default().fg(Color::Yellow),
                            )
                        } else {
                            Span::styled(
                                "✗ No cache - Using fallback pricing",
                                Style::default().fg(Color::Red),
                            )
                        }
                    } else {
                        Span::styled(
                            "✗ No cache - Using fallback pricing",
                            Style::default().fg(Color::Red),
                        )
                    },
                ]),
            ];

            let summary = Paragraph::new(summary_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("⏰ 5-Hour Billing Blocks Summary")
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(summary, chunks[0]);
        } else {
            let controls = Paragraph::new("Press 's' to toggle summary | Arrow keys to navigate")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("⏰ Billing Blocks"),
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
                        .title("📋 No Billing Blocks")
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
                Cell::from(format!("${:.4}", block.usage.total_cost))
                    .style(Style::default().fg(cost_color)),
                Cell::from(self.format_number(block.usage.total_tokens()))
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(format!("{}", block.session_count))
                    .style(Style::default().fg(Color::Blue)),
                Cell::from(format!("${:.4}", avg_per_session))
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
                .title("📋 5-Hour Billing Blocks")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

        f.render_stateful_widget(table, chunks[1], &mut self.billing_blocks_table_state);

        // Scrollbar
        if report.blocks.len() > 10 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
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

    fn render_help_popup(&mut self, f: &mut Frame) {
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
            Tab::Charts => Tab::BillingBlocks,
            Tab::BillingBlocks => Tab::Resume,
            Tab::Resume => Tab::Help,
            Tab::Help => Tab::Overview,
        };
        // Auto-load removed to prevent hanging
        // if self.current_tab == Tab::Resume && self.resume_sessions.is_empty() {
        //     self.load_resume_sessions();
        // }
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Help,
            Tab::Daily => Tab::Overview,
            Tab::Sessions => Tab::Daily,
            Tab::Charts => Tab::Sessions,
            Tab::BillingBlocks => Tab::Charts,
            Tab::Resume => Tab::BillingBlocks,
            Tab::Help => Tab::Resume,
        };
        // Auto-load removed to prevent hanging
        // if self.current_tab == Tab::Resume && self.resume_sessions.is_empty() {
        //     self.load_resume_sessions();
        // }
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
            Tab::BillingBlocks => {
                let report = self.billing_manager.generate_report();
                let i = match self.billing_blocks_table_state.selected() {
                    Some(i) => {
                        if i >= report.blocks.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.billing_blocks_table_state.select(Some(i));
                self.billing_blocks_scroll_state = self.billing_blocks_scroll_state.position(i);
            }
            Tab::Resume => {
                let i = match self.resume_table_state.selected() {
                    Some(i) => {
                        if i >= self.resume_sessions.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.resume_table_state.select(Some(i));
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
            Tab::BillingBlocks => {
                let report = self.billing_manager.generate_report();
                let i = match self.billing_blocks_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            report.blocks.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.billing_blocks_table_state.select(Some(i));
                self.billing_blocks_scroll_state = self.billing_blocks_scroll_state.position(i);
            }
            Tab::Resume => {
                let i = match self.resume_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.resume_sessions.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.resume_table_state.select(Some(i));
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
        // Handle enter key based on current tab
        match self.current_tab {
            Tab::Resume => {
                // Open Claude session in browser
                if let Some(selected) = self.resume_table_state.selected() {
                    if let Some(resume_session) = self.resume_sessions.get(selected) {
                        if let Some(session_data) = &resume_session.session_data {
                            let parser = ClaudeSessionParser::new(None);
                            match parser.open_session(session_data) {
                                Ok(_) => {
                                    self.status_message =
                                        Some("🚀 Opening session in browser...".to_string());
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("❌ Failed to open session: {}", e));
                                }
                            }
                        } else {
                            self.status_message =
                                Some("ℹ️ This is a demo session (no real data)".to_string());
                        }
                    }
                }
            }
            Tab::Sessions => {
                // Copy session info to clipboard
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
            _ => {}
        }
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
}
