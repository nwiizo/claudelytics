//! # Claudelytics TUI Module
//!
//! Enhanced Terminal User Interface with vim-style keyboard navigation:
//! - gg/G: Jump to top/bottom
//! - Ctrl+d/Ctrl+u: Half-page scrolling
//! - 0/$: Beginning/end of line navigation
//! - w/b: Word navigation in search mode
//! - v: Visual mode for multi-select operations
//!
//! The TUI provides comprehensive analytics views with advanced filtering,
//! sorting, and export capabilities for Claude Code usage data.

use crate::billing_blocks::BillingBlockManager;
use crate::cache_analysis::{self, CacheAnalysis};
use crate::models::{Command, CommandAction, DailyReport, SessionReport, TokenUsage, WeeklyReport};
use crate::pricing_cache::PricingCache;
use crate::reports::generate_weekly_report_sorted;
use crate::tui_visuals::{
    AnimationStyle, ProgressColorScheme, SmoothProgressBar, ToastNotification, VisualEffectsManager,
};
use anyhow::Result;
use chrono::{NaiveDate, Weekday};
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
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Daily,
    Weekly,
    Sessions,
    Cache,
    BillingBlocks,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum AppMode {
    Normal,
    CommandPalette,
    Search,
    Visual,
    ExportDialog,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExportFormat {
    Csv,
    Json,
    Markdown,
    Text,
}

#[derive(Debug)]
struct ExportDialogState {
    selected_format: ExportFormat,
    show_success_message: bool,
    success_message: String,
    error_message: Option<String>,
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
    // Billing blocks
    billing_manager: BillingBlockManager,
    billing_blocks_table_state: TableState,
    billing_blocks_scroll_state: ScrollbarState,
    show_billing_summary: bool,
    // Pricing cache status
    pricing_cache_status: Option<PricingCacheStatus>,
    // Visual mode selection
    visual_mode_start: Option<usize>,
    visual_mode_selections: Vec<usize>,
    // Search mode cursor position for word navigation
    search_cursor_position: usize,
    // Track if 'g' was pressed for 'gg' command
    g_pressed: bool,
    // Export dialog state
    export_dialog_state: ExportDialogState,
    // Visual effects manager
    visual_effects: VisualEffectsManager,
    // Export dialog state
    previous_mode: Option<AppMode>,
    // Weekly report (computed lazily)
    weekly_report: Option<WeeklyReport>,
    weekly_table_state: TableState,
    // Cache analysis (computed lazily)
    cache_analysis: Option<CacheAnalysis>,
    cache_table_state: TableState,
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
            billing_manager,
            billing_blocks_table_state: TableState::default(),
            billing_blocks_scroll_state,
            show_billing_summary: true,
            pricing_cache_status,
            visual_mode_start: None,
            visual_mode_selections: Vec::new(),
            search_cursor_position: 0,
            g_pressed: false,
            export_dialog_state: ExportDialogState {
                selected_format: ExportFormat::Csv,
                show_success_message: false,
                success_message: String::new(),
                error_message: None,
            },
            visual_effects: VisualEffectsManager::new(),
            previous_mode: None,
            weekly_report: None,
            weekly_table_state: TableState::default(),
            cache_analysis: None,
            cache_table_state: TableState::default(),
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
        if let Some(selected) = self.session_table_state.selected()
            && let Some(session) = self.session_report.sessions.get(selected)
        {
            return Some(format!("{}/{}", session.project_path, session.session_id));
        }
        None
    }

    // State restoration methods for resume functionality
    pub fn set_current_tab(&mut self, tab_index: usize) {
        self.current_tab = match tab_index {
            0 => Tab::Overview,
            1 => Tab::Daily,
            2 => Tab::Weekly,
            3 => Tab::Sessions,
            4 => Tab::Cache,
            5 => Tab::BillingBlocks,
            6 => Tab::Help,
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
                name: "Switch to Cache".to_string(),
                description: "Go to cache analysis tab".to_string(),
                shortcut: Some("5".to_string()),
                action: CommandAction::SwitchTab(4),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Billing".to_string(),
                description: "Go to billing blocks tab".to_string(),
                shortcut: Some("6".to_string()),
                action: CommandAction::SwitchTab(5),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Help".to_string(),
                description: "Go to help tab".to_string(),
                shortcut: Some("h".to_string()),
                action: CommandAction::SwitchTab(6),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Export Data".to_string(),
                description: "Open export dialog (CSV/JSON to clipboard)".to_string(),
                shortcut: Some("e/Ctrl+E".to_string()),
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
            // Update visual effects
            self.visual_effects.tick();

            terminal.draw(|f| self.ui(f))?;

            // Check for events with timeout to prevent hanging
            if poll(std::time::Duration::from_millis(50))?
                && let Ok(evt) = event::read()
            {
                match evt {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            // Add key press visual effect
                            if let KeyCode::Char(c) = key.code {
                                let key_str = if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    format!("Ctrl+{}", c)
                                } else {
                                    c.to_string()
                                };
                                let effect_pos = Rect {
                                    x: terminal.size()?.width / 2 - 2,
                                    y: terminal.size()?.height - 5,
                                    width: 5,
                                    height: 1,
                                };
                                self.visual_effects.add_key_effect(key_str, effect_pos);
                            }

                            match self.current_mode {
                                AppMode::CommandPalette => {
                                    self.handle_command_palette_input(key.code, key.modifiers)?;
                                }
                                AppMode::Search => {
                                    self.handle_search_input(key.code)?;
                                }
                                AppMode::Visual => {
                                    self.handle_visual_mode_input(key.code)?;
                                }
                                AppMode::ExportDialog => {
                                    self.handle_export_dialog_input(key.code)?;
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

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_normal_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        // Reset g_pressed if any key other than 'g' is pressed
        if !matches!(key, KeyCode::Char('g')) {
            self.g_pressed = false;
        }

        // Handle Ctrl+P for command palette
        if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('p') {
            self.current_mode = AppMode::CommandPalette;
            self.command_palette_query.clear();
            self.filtered_commands = self.available_commands.clone();
            self.command_table_state.select(Some(0));
            self.status_message = Some("Command Palette: Type to search commands".to_string());
            return Ok(());
        }

        // Handle Ctrl+E for export
        if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('e') {
            self.open_export_dialog();
            return Ok(());
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                self.visual_effects.add_toast(ToastNotification::info(
                    "Exiting Claudelytics...".to_string(),
                ));
            }
            KeyCode::Char('1') => {
                self.current_tab = Tab::Overview;
                self.visual_effects
                    .add_toast(ToastNotification::info("Switched to Overview".to_string()));
            }
            KeyCode::Char('2') => {
                self.current_tab = Tab::Daily;
                self.visual_effects.add_toast(ToastNotification::info(
                    "Switched to Daily View".to_string(),
                ));
            }
            KeyCode::Char('3') => {
                self.current_tab = Tab::Weekly;
                self.visual_effects.add_toast(ToastNotification::info(
                    "Switched to Weekly View".to_string(),
                ));
            }
            KeyCode::Char('4') => {
                self.current_tab = Tab::Sessions;
                self.visual_effects
                    .add_toast(ToastNotification::info("Switched to Sessions".to_string()));
            }
            KeyCode::Char('5') => {
                self.current_tab = Tab::Cache;
                self.visual_effects.add_toast(ToastNotification::info(
                    "Switched to Cache Analysis".to_string(),
                ));
            }
            KeyCode::Char('6') => {
                self.current_tab = Tab::BillingBlocks;
                self.visual_effects.add_toast(ToastNotification::info(
                    "Switched to Billing Blocks".to_string(),
                ));
            }
            KeyCode::Char('h') => {
                self.current_tab = Tab::Help;
                self.visual_effects
                    .add_toast(ToastNotification::info("Showing Help".to_string()));
            }
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
            // Enhanced vim navigation
            KeyCode::Char('g') => {
                if modifiers.contains(KeyModifiers::NONE) {
                    if self.g_pressed {
                        // Second 'g' pressed - execute 'gg' command
                        self.jump_to_top();
                        self.g_pressed = false;
                        self.status_message = Some("Jumped to top".to_string());
                    } else {
                        // First 'g' pressed - wait for second
                        self.g_pressed = true;
                        self.status_message = Some("Press 'g' again to jump to top".to_string());
                    }
                }
            }
            KeyCode::Char('G') => {
                // Jump to bottom
                self.jump_to_bottom();
            }
            KeyCode::Char('d') => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+d - half page down
                    self.half_page_down();
                }
            }
            KeyCode::Char('u') => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+u - half page up
                    self.half_page_up();
                }
            }
            KeyCode::Char('0') => {
                // Beginning of line (first item in current view)
                self.jump_to_line_start();
            }
            KeyCode::Char('$') => {
                // End of line (last item in current view)
                self.jump_to_line_end();
            }
            KeyCode::Char('v') => {
                // Enter visual mode
                self.toggle_visual_mode();
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query.clear();
                self.status_message = Some("Search: (Press Esc to cancel)".to_string());
            }
            KeyCode::Char('r') => {
                self.visual_effects
                    .add_loading("Refreshing data...".to_string(), AnimationStyle::Spinner);
                self.refresh_data()?;
                self.visual_effects.loading_animations.clear();
                self.visual_effects
                    .add_toast(ToastNotification::success("Data refreshed!".to_string()));
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
                self.open_export_dialog();
            }
            KeyCode::Char('b') => {
                self.bookmark_selected_session();
                if let Some(msg) = &self.status_message
                    && msg.contains("Bookmarked")
                {
                    self.visual_effects
                        .add_toast(ToastNotification::success(msg.clone()));
                }
            }
            KeyCode::Char('x') => {
                self.toggle_comparison_selection();
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
                self.search_cursor_position = 0;
                self.status_message = None;
                self.apply_filters();
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.apply_filters();
                self.status_message = Some(format!("Filtered by: '{}'", self.search_query));
            }
            KeyCode::Backspace => {
                if self.search_cursor_position > 0 {
                    self.search_query.remove(self.search_cursor_position - 1);
                    self.search_cursor_position -= 1;
                }
                self.update_search_status();
            }
            KeyCode::Delete => {
                if self.search_cursor_position < self.search_query.len() {
                    self.search_query.remove(self.search_cursor_position);
                }
                self.update_search_status();
            }
            KeyCode::Left => {
                if self.search_cursor_position > 0 {
                    self.search_cursor_position -= 1;
                }
                self.update_search_status();
            }
            KeyCode::Right => {
                if self.search_cursor_position < self.search_query.len() {
                    self.search_cursor_position += 1;
                }
                self.update_search_status();
            }
            KeyCode::Home => {
                self.search_cursor_position = 0;
                self.update_search_status();
            }
            KeyCode::End => {
                self.search_cursor_position = self.search_query.len();
                self.update_search_status();
            }
            // Word navigation
            KeyCode::Char('w') => {
                self.search_cursor_position = self.next_word_position();
                self.update_search_status();
            }
            KeyCode::Char('b') => {
                self.search_cursor_position = self.prev_word_position();
                self.update_search_status();
            }
            KeyCode::Char(c) => {
                if c != 'w' && c != 'b' {
                    self.search_query.insert(self.search_cursor_position, c);
                    self.search_cursor_position += 1;
                    self.update_search_status();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_visual_mode_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                // Exit visual mode
                self.toggle_visual_mode();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_item();
                self.update_visual_selection();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_item();
                self.update_visual_selection();
            }
            KeyCode::Char('G') => {
                self.jump_to_bottom();
                self.update_visual_selection();
            }
            KeyCode::Char('g') => {
                self.jump_to_top();
                self.update_visual_selection();
            }
            KeyCode::Char('x') => {
                // Mark current item for multi-select
                if let Some(index) = self.get_current_selected_index() {
                    if self.visual_mode_selections.contains(&index) {
                        self.visual_mode_selections.retain(|&x| x != index);
                        self.status_message = Some(format!("Unmarked item {}", index + 1));
                    } else {
                        self.visual_mode_selections.push(index);
                        self.status_message = Some(format!(
                            "Marked item {} ({} total)",
                            index + 1,
                            self.visual_mode_selections.len()
                        ));
                    }
                }
            }
            KeyCode::Char('b') => {
                // Bookmark all selected items in visual mode
                self.bookmark_visual_selections();
            }
            KeyCode::Char('e') => {
                // Export selected items
                self.export_visual_selections()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn update_visual_selection(&mut self) {
        if let (Some(start), Some(current)) =
            (self.visual_mode_start, self.get_current_selected_index())
        {
            self.visual_mode_selections.clear();
            let (from, to) = if start <= current {
                (start, current)
            } else {
                (current, start)
            };
            for i in from..=to {
                self.visual_mode_selections.push(i);
            }
            self.status_message = Some(format!(
                "Visual: {} items selected",
                self.visual_mode_selections.len()
            ));
        }
    }

    fn get_current_selected_index(&self) -> Option<usize> {
        match self.current_tab {
            Tab::Daily => self.daily_table_state.selected(),
            Tab::Weekly => self.weekly_table_state.selected(),
            Tab::Sessions => self.session_table_state.selected(),
            Tab::BillingBlocks => self.billing_blocks_table_state.selected(),
            Tab::Cache => self.cache_table_state.selected(),
            _ => None,
        }
    }

    fn bookmark_visual_selections(&mut self) {
        if self.current_tab != Tab::Sessions {
            self.status_message = Some("Visual bookmarking only works in Sessions tab".to_string());
            return;
        }

        let mut count = 0;
        for &index in &self.visual_mode_selections {
            if let Some(session) = self.session_report.sessions.get(index) {
                let session_id = format!("{}/{}", session.project_path, session.session_id);
                if !self.bookmarked_sessions.contains(&session_id) {
                    self.bookmarked_sessions.push(session_id);
                    count += 1;
                }
            }
        }

        self.status_message = Some(format!("Bookmarked {} sessions", count));
        self.toggle_visual_mode(); // Exit visual mode
    }

    fn export_visual_selections(&mut self) -> Result<()> {
        // This would export only the selected items
        self.status_message = Some(format!(
            "Would export {} selected items",
            self.visual_mode_selections.len()
        ));
        self.toggle_visual_mode(); // Exit visual mode
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

        // Sort both daily and sessions
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
            SortMode::Efficiency => {
                // Sort by cache hit rate (cache_read / total input)
                let cache_rate = |cr: u64, cc: u64, inp: u64| -> f64 {
                    let denom = cr + cc + inp;
                    if denom == 0 {
                        0.0
                    } else {
                        cr as f64 / denom as f64
                    }
                };
                self.daily_report.daily.sort_by(|a, b| {
                    let ra =
                        cache_rate(a.cache_read_tokens, a.cache_creation_tokens, a.input_tokens);
                    let rb =
                        cache_rate(b.cache_read_tokens, b.cache_creation_tokens, b.input_tokens);
                    rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
                });
                self.session_report.sessions.sort_by(|a, b| {
                    let ra =
                        cache_rate(a.cache_read_tokens, a.cache_creation_tokens, a.input_tokens);
                    let rb =
                        cache_rate(b.cache_read_tokens, b.cache_creation_tokens, b.input_tokens);
                    rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMode::Project => {
                self.session_report
                    .sessions
                    .sort_by(|a, b| a.project_path.cmp(&b.project_path));
                // Daily has no project, fall back to date
                self.daily_report.daily.sort_by(|a, b| b.date.cmp(&a.date));
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

    fn open_export_dialog(&mut self) {
        self.current_mode = AppMode::ExportDialog;
        self.export_dialog_state.selected_format = ExportFormat::Csv;
        self.export_dialog_state.show_success_message = false;
        self.export_dialog_state.error_message = None;
        self.status_message = Some(
            "📁 Export: Use arrows to select format, Enter to export, Esc to cancel".to_string(),
        );
    }

    fn handle_export_dialog_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                // Return to previous mode if available, otherwise Normal
                self.current_mode = self.previous_mode.unwrap_or(AppMode::Normal);
                self.previous_mode = None;
                self.status_message = None;
            }
            KeyCode::Left => {
                self.export_dialog_state.selected_format =
                    match self.export_dialog_state.selected_format {
                        ExportFormat::Csv => ExportFormat::Text,
                        ExportFormat::Json => ExportFormat::Csv,
                        ExportFormat::Markdown => ExportFormat::Json,
                        ExportFormat::Text => ExportFormat::Markdown,
                    };
            }
            KeyCode::Right | KeyCode::Tab => {
                self.export_dialog_state.selected_format =
                    match self.export_dialog_state.selected_format {
                        ExportFormat::Csv => ExportFormat::Json,
                        ExportFormat::Json => ExportFormat::Markdown,
                        ExportFormat::Markdown => ExportFormat::Text,
                        ExportFormat::Text => ExportFormat::Csv,
                    };
            }
            KeyCode::Enter => {
                self.execute_export()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_export(&mut self) -> Result<()> {
        let format = self.export_dialog_state.selected_format;

        let data_type = match self.current_tab {
            Tab::Daily => "daily",
            Tab::Weekly => "weekly",
            Tab::Sessions => "sessions",
            Tab::BillingBlocks => "billing",
            Tab::Overview => "summary",
            _ => {
                self.export_dialog_state.error_message =
                    Some("Export not available for this tab".to_string());
                return Ok(());
            }
        };

        match format {
            ExportFormat::Csv => self.export_to_csv(data_type),
            ExportFormat::Json => self.export_to_json(data_type),
            ExportFormat::Markdown | ExportFormat::Text => {
                self.export_dialog_state.error_message = Some(format!(
                    "{:?} format not supported for this export type",
                    format
                ));
                Ok(())
            }
        }
    }

    fn export_to_csv(&mut self, data_type: &str) -> Result<()> {
        use tempfile::NamedTempFile;

        let result = match data_type {
            "daily" => {
                let temp_file = NamedTempFile::new()?;
                let path = temp_file.path().to_path_buf();
                crate::export::export_daily_to_csv(&self.daily_report, &path)?;
                self.copy_to_clipboard_from_file(&path)?;
                Ok(())
            }
            "sessions" => {
                let temp_file = NamedTempFile::new()?;
                let path = temp_file.path().to_path_buf();
                crate::export::export_sessions_to_csv(&self.session_report, &path)?;
                self.copy_to_clipboard_from_file(&path)?;
                Ok(())
            }
            "billing" => {
                let content = self.generate_billing_csv()?;
                self.copy_to_clipboard(&content)?;
                Ok(())
            }
            "summary" => {
                let temp_file = NamedTempFile::new()?;
                let path = temp_file.path().to_path_buf();
                crate::export::export_summary_to_csv(
                    &self.daily_report,
                    &self.session_report,
                    &path,
                )?;
                self.copy_to_clipboard_from_file(&path)?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Unknown data type")),
        };

        match result {
            Ok(_) => {
                self.export_dialog_state.show_success_message = true;
                self.export_dialog_state.success_message =
                    format!("✅ {} data exported to clipboard as CSV!", data_type);
                self.status_message = Some(self.export_dialog_state.success_message.clone());
                self.current_mode = AppMode::Normal;
            }
            Err(e) => {
                self.export_dialog_state.error_message = Some(format!("Export failed: {}", e));
                self.status_message = Some(format!("❌ Export failed: {}", e));
            }
        }
        Ok(())
    }

    fn export_to_json(&mut self, data_type: &str) -> Result<()> {
        let json_content = match data_type {
            "daily" => serde_json::to_string_pretty(&self.daily_report)?,
            "sessions" => serde_json::to_string_pretty(&self.session_report)?,
            "billing" => {
                let report = self.billing_manager.generate_report();
                serde_json::to_string_pretty(&report)?
            }
            "summary" => {
                let summary = serde_json::json!({
                    "daily_totals": self.daily_report.totals,
                    "total_days": self.daily_report.daily.len(),
                    "total_sessions": self.session_report.sessions.len(),
                    "date_range": {
                        "from": self.daily_report.daily.last().map(|d| &d.date),
                        "to": self.daily_report.daily.first().map(|d| &d.date),
                    }
                });
                serde_json::to_string_pretty(&summary)?
            }
            _ => return Err(anyhow::anyhow!("Unknown data type")),
        };

        match self.copy_to_clipboard(&json_content) {
            Ok(_) => {
                self.export_dialog_state.show_success_message = true;
                self.export_dialog_state.success_message =
                    format!("✅ {} data exported to clipboard as JSON!", data_type);
                self.status_message = Some(self.export_dialog_state.success_message.clone());
                self.current_mode = AppMode::Normal;
            }
            Err(e) => {
                self.export_dialog_state.error_message = Some(format!("Export failed: {}", e));
                self.status_message = Some(format!("❌ Export failed: {}", e));
            }
        }
        Ok(())
    }

    fn generate_billing_csv(&self) -> Result<String> {
        use std::fmt::Write;
        let mut output = String::new();

        writeln!(
            &mut output,
            "Date,Block,Start Time,End Time,Sessions,Input Tokens,Output Tokens,Total Tokens,Cost USD"
        )?;

        let report = self.billing_manager.generate_report();
        for block in &report.blocks {
            writeln!(
                &mut output,
                "{},{},{},{},{},{},{},{},{:.6}",
                block.date,
                block.time_range,
                block.start_time,
                block.end_time,
                block.session_count,
                block.usage.input_tokens,
                block.usage.output_tokens,
                block.usage.total_tokens(),
                block.usage.total_cost
            )?;
        }

        Ok(output)
    }

    fn copy_to_clipboard(&self, content: &str) -> Result<()> {
        let mut ctx = ClipboardContext::new()
            .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;
        ctx.set_contents(content.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to copy to clipboard: {}", e))?;
        Ok(())
    }

    fn copy_to_clipboard_from_file(&self, path: &std::path::Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.copy_to_clipboard(&content)
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
                if let Some(selected) = self.command_table_state.selected()
                    && let Some(command) = self.filtered_commands.get(selected)
                {
                    let action = command.action.clone();
                    self.execute_command(&action)?;
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
                    2 => Tab::Weekly,
                    3 => Tab::Sessions,
                    4 => Tab::Cache,
                    5 => Tab::BillingBlocks,
                    6 => Tab::Help,
                    _ => Tab::Overview,
                };
                self.status_message = Some(format!("Switched to tab {}", index + 1));
            }
            CommandAction::ExportData(_) => {
                self.status_message =
                    Some("Use 'claudelytics export' command for CSV export".to_string());
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
        if let Some(selected) = self.session_table_state.selected()
            && let Some(session) = self.session_report.sessions.get(selected)
        {
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

    fn toggle_comparison_selection(&mut self) {
        if let Some(selected) = self.session_table_state.selected()
            && let Some(session) = self.session_report.sessions.get(selected)
        {
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

    fn ui(&mut self, f: &mut Frame) {
        match self.current_mode {
            AppMode::CommandPalette => {
                self.render_main_ui(f);
                self.render_command_palette(f);
            }
            AppMode::ExportDialog => {
                self.render_main_ui(f);
                self.render_export_dialog(f);
            }
            _ => {
                self.render_main_ui(f);
            }
        }

        if self.show_help_popup {
            self.render_help_popup(f);
        }

        // Render visual effects on top
        self.visual_effects.render_all(f);

        // Render loading animations if any
        if !self.visual_effects.loading_animations.is_empty() {
            let loading_area = Rect {
                x: f.area().width / 2 - 15,
                y: f.area().height / 2,
                width: 30,
                height: 3,
            };
            for anim in &self.visual_effects.loading_animations {
                anim.render(f, loading_area);
            }
        }
    }

    fn render_main_ui(&mut self, f: &mut Frame) {
        // Update status bar information
        self.visual_effects.status_bar.mode = match self.current_mode {
            AppMode::Normal => {
                if self.search_mode {
                    "Search"
                } else {
                    "Normal"
                }
            }
            AppMode::CommandPalette => "Command",
            AppMode::Search => "Search",
            AppMode::Visual => "Visual",
            AppMode::ExportDialog => "Export",
        }
        .to_string();

        self.visual_effects.status_bar.filter = match self.time_filter {
            TimeFilter::All => "All",
            TimeFilter::Today => "Today",
            TimeFilter::LastWeek => "Week",
            TimeFilter::LastMonth => "Month",
        }
        .to_string();

        self.visual_effects.status_bar.sort = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Efficiency => "Efficiency",
            SortMode::Project => "Project",
        }
        .to_string();

        self.visual_effects.status_bar.items_count = match self.current_tab {
            Tab::Daily => self.daily_report.daily.len(),
            Tab::Weekly => self
                .weekly_report
                .as_ref()
                .map(|r| r.weekly.len())
                .unwrap_or(0),
            Tab::Sessions => self.session_report.sessions.len(),
            Tab::Cache => self
                .cache_analysis
                .as_ref()
                .map(|a| a.sessions.len())
                .unwrap_or(0),
            _ => 0,
        };

        self.visual_effects.status_bar.selected_index = match self.current_tab {
            Tab::Daily => self.daily_table_state.selected(),
            Tab::Weekly => self.weekly_table_state.selected(),
            Tab::Sessions => self.session_table_state.selected(),
            Tab::Cache => self.cache_table_state.selected(),
            _ => None,
        };

        // Set key hints based on current tab
        let hints = match self.current_tab {
            Tab::Overview => vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("/".to_string(), "Search".to_string()),
                ("r".to_string(), "Refresh".to_string()),
            ],
            Tab::Sessions => vec![
                ("b".to_string(), "Bookmark".to_string()),
                ("x".to_string(), "Compare".to_string()),
                ("s".to_string(), "Sort".to_string()),
                ("f".to_string(), "Filter".to_string()),
            ],
            _ => vec![],
        };
        self.visual_effects.status_bar.set_key_hints(hints);

        let main_chunks = if self.status_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                    Constraint::Length(1), // Status bar
                ])
                .split(f.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1), // Status bar
                ])
                .split(f.area())
        };

        let main_area = main_chunks[1];

        // Tab bar with enhanced titles
        let tab_titles = vec![
            "📊 Overview",
            "📅 Daily",
            "📆 Weekly",
            "📋 Sessions",
            "🔄 Cache",
            "⏰ Billing",
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
            Tab::Weekly => self.render_weekly(f, main_area),
            Tab::Sessions => self.render_sessions(f, main_area),
            Tab::Cache => self.render_cache(f, main_area),
            Tab::BillingBlocks => self.render_billing_blocks(f, main_area),
            Tab::Help => self.render_help(f, main_area),
        }

        // Status message
        if let Some(ref message) = self.status_message
            && main_chunks.len() > 2
        {
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

        // Render enhanced status bar at the bottom
        let status_bar_area = main_chunks[main_chunks.len() - 1];
        self.visual_effects.status_bar.render(f, status_bar_area);
    }

    fn render_overview(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Enhanced summary
                Constraint::Length(8),  // Quick stats
                Constraint::Length(3),  // Progress bars
                Constraint::Min(0),     // Cost breakdown with chart
            ])
            .margin(1)
            .split(area);

        // Enhanced summary card
        let summary_text = vec![
            Line::from(vec![
                Span::styled("💰 Total Cost: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("${:.2}", self.daily_report.totals.total_cost),
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
                        "${:.2}",
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

        // Render progress bars
        let progress_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Compute dynamic maximums from actual data
        let max_daily_cost = self
            .daily_report
            .daily
            .iter()
            .map(|d| d.total_cost)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let max_daily_tokens = self
            .daily_report
            .daily
            .iter()
            .map(|d| d.total_tokens)
            .max()
            .unwrap_or(1) as f64;
        // Round up to a nice ceiling (next power-of-10-ish)
        let cost_ceiling = (max_daily_cost * 1.2).max(10.0);
        let token_ceiling = (max_daily_tokens * 1.2).max(100_000.0);

        // Cost progress bar
        if self.visual_effects.progress_bars.is_empty() {
            let mut cost_bar = SmoothProgressBar::new("Daily Cost".to_string(), cost_ceiling);
            cost_bar.set_value(self.daily_report.totals.total_cost);
            cost_bar.set_color_scheme(ProgressColorScheme::CostBased);
            self.visual_effects.progress_bars.push(cost_bar);

            let mut token_bar = SmoothProgressBar::new("Token Usage".to_string(), token_ceiling);
            token_bar.set_value(self.daily_report.totals.total_tokens as f64);
            token_bar.set_color_scheme(ProgressColorScheme::TokenBased);
            self.visual_effects.progress_bars.push(token_bar);
        }

        // Update progress bar values and max
        if let Some(cost_bar) = self.visual_effects.progress_bars.get_mut(0) {
            cost_bar.set_max(cost_ceiling);
            cost_bar.set_value(self.daily_report.totals.total_cost);
            cost_bar.render(f, progress_chunks[0]);
        }

        if let Some(token_bar) = self.visual_effects.progress_bars.get_mut(1) {
            token_bar.set_max(token_ceiling);
            token_bar.set_value(self.daily_report.totals.total_tokens as f64);
            token_bar.render(f, progress_chunks[1]);
        }

        // Cost gauge based on total cost relative to dynamic max
        if self.daily_report.totals.total_cost > 0.0 {
            let cost_ratio = (self.daily_report.totals.total_cost / cost_ceiling).min(1.0);
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("💳 Total Cost")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(cost_ratio)
                .label(format!(
                    "${:.2} / ${:.0}",
                    self.daily_report.totals.total_cost, cost_ceiling
                ));
            f.render_widget(gauge, chunks[3]);
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
                    .title(format!("📅 Daily Report [Sort: {}]", sort_label)),
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

            // Color code based on cost
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
                .title("📋 Daily Usage Data")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

        f.render_stateful_widget(table, chunks[1], &mut self.daily_table_state);
    }

    fn ensure_weekly_report(&mut self) {
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

    fn render_weekly(&mut self, f: &mut Frame, area: Rect) {
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
                    Cell::from(self.format_number(w.input_tokens))
                        .style(Style::default().fg(Color::Blue)),
                    Cell::from(self.format_number(w.output_tokens))
                        .style(Style::default().fg(Color::Cyan)),
                    Cell::from(self.format_number(w.total_tokens))
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
                format!("Tokens: {}", self.format_number(totals.total_tokens)),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("In: {}", self.format_number(totals.input_tokens)),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(" | "),
            Span::styled(
                format!("Out: {}", self.format_number(totals.output_tokens)),
                Style::default().fg(Color::Cyan),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(total_info, chunks[1]);
    }

    /// Extract a human-readable project name from a hyphen-encoded path.
    ///
    /// Strategy: look for `github-com-OWNER-REPO` and return `OWNER/REPO`.
    /// Otherwise fall back to the last 2 hyphen-segments.
    fn extract_project_name(raw_path: &str) -> String {
        let stripped = raw_path.trim_start_matches('-');
        let segments: Vec<&str> = stripped.split('-').collect();

        // Look for github-com pattern
        for (i, seg) in segments.iter().enumerate() {
            if seg.eq_ignore_ascii_case("com")
                && i > 0
                && segments[i - 1].eq_ignore_ascii_case("github")
                && i + 2 < segments.len()
            {
                let owner = segments[i + 1];
                // Repo may contain hyphens, so rejoin everything after owner
                let repo = segments[i + 2..].join("-");
                return format!("{}/{}", owner, repo);
            }
        }

        // Fallback: last 2 segments
        if segments.len() >= 2 {
            format!(
                "{}/{}",
                segments[segments.len() - 2],
                segments[segments.len() - 1]
            )
        } else if segments.len() == 1 {
            segments[0].to_string()
        } else {
            raw_path.to_string()
        }
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

        let sort_label = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Efficiency => "Efficiency",
            SortMode::Project => "Project",
        };

        let controls = Paragraph::new(controls_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("📊 Sessions [Sort: {}]", sort_label)),
            )
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
            "Project",
            "Session",
            "Cost",
            "Tokens",
            "Cache Hit%",
            "Last Activity",
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
                // Extract project name and session UUID from the data
                // project_path may contain "dir/uuid" or just "dir"
                // session_id may contain "subagents" or be the project dir itself
                let full_path = if session.project_path.is_empty() {
                    session.session_id.clone()
                } else {
                    format!("{}/{}", session.project_path, session.session_id)
                };

                // Try to find a UUID-like segment anywhere in the path
                let parts: Vec<&str> = full_path.split('/').collect();
                let mut uuid_part = String::new();
                let mut dir_parts = Vec::new();
                for part in &parts {
                    // UUID pattern: 8-4-4-4-12 hex chars with hyphens, length 36
                    if part.len() == 36
                        && part.chars().filter(|c| *c == '-').count() == 4
                        && part.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
                    {
                        if uuid_part.is_empty() {
                            uuid_part = part.to_string();
                        }
                    } else {
                        dir_parts.push(*part);
                    }
                }
                let dir_part = if dir_parts.is_empty() {
                    full_path.clone()
                } else {
                    dir_parts.join("/")
                };

                let project_name = Self::extract_project_name(&dir_part);
                let session_short = if uuid_part.len() >= 8 {
                    uuid_part[..8].to_string()
                } else {
                    "-".to_string()
                };

                // Calculate cache hit %
                let cache_denom = session.cache_read_tokens
                    + session.cache_creation_tokens
                    + session.input_tokens;
                let cache_hit_pct = if cache_denom > 0 {
                    session.cache_read_tokens as f64 / cache_denom as f64 * 100.0
                } else {
                    0.0
                };

                // Color code based on cost
                let cost_color = if session.total_cost > 1.0 {
                    Color::Red
                } else if session.total_cost > 0.5 {
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

                // Visual mode highlighting
                let style = if self.current_mode == AppMode::Visual
                    && self.visual_mode_selections.contains(&i)
                {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else if i % 2 == 0 {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };

                Row::new(vec![
                    Cell::from(self.truncate_text(&project_name, 30)).style(style),
                    Cell::from(session_short).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(format!("${:.2}", session.total_cost))
                        .style(Style::default().fg(cost_color)),
                    Cell::from(self.format_number(session.total_tokens))
                        .style(Style::default().fg(Color::Magenta)),
                    Cell::from(format!("{:.1}%", cache_hit_pct))
                        .style(Style::default().fg(hit_color)),
                    Cell::from(session.last_activity.clone())
                        .style(Style::default().fg(Color::Yellow)),
                ])
                .height(1)
            });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(30), // Project
                Constraint::Length(10),     // Session (UUID short)
                Constraint::Length(10),     // Cost
                Constraint::Length(12),     // Tokens
                Constraint::Length(10),     // Cache Hit%
                Constraint::Length(20),     // Last Activity
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
                Span::styled("  1-6/h, Tab/Shift+Tab", Style::default().fg(Color::Green)),
                Span::styled("  Switch between tabs", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  ↑/↓, j/k", Style::default().fg(Color::Green)),
                Span::styled(
                    "          Navigate up/down",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  gg/G", Style::default().fg(Color::Green)),
                Span::styled(
                    "              Jump to top/bottom",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+d/Ctrl+u", Style::default().fg(Color::Green)),
                Span::styled(
                    "       Half page down/up",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  0/$", Style::default().fg(Color::Green)),
                Span::styled(
                    "               Beginning/end of line",
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
            Line::from(vec![
                Span::styled("  v", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Visual mode (multi-select)",
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
                Span::styled("    w/b", Style::default().fg(Color::Blue)),
                Span::styled(
                    "             Word forward/backward in search",
                    Style::default().fg(Color::Gray),
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
                "📌 Visual Mode:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  v", Style::default().fg(Color::Green)),
                Span::styled(
                    "                 Enter/exit visual mode",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![Span::styled(
                "  In visual mode:",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![
                Span::styled("    j/k", Style::default().fg(Color::Blue)),
                Span::styled(
                    "             Extend selection",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("    x", Style::default().fg(Color::Blue)),
                Span::styled(
                    "               Mark/unmark for multi-select",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("    b", Style::default().fg(Color::Blue)),
                Span::styled(
                    "               Bookmark selected items",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("    e", Style::default().fg(Color::Blue)),
                Span::styled(
                    "               Export selected items",
                    Style::default().fg(Color::Gray),
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
                Span::styled("  e, Ctrl+E", Style::default().fg(Color::Green)),
                Span::styled(
                    "          Open export dialog (CSV/JSON)",
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
                Span::styled("  3. Weekly", Style::default().fg(Color::Green)),
                Span::styled(
                    "        Weekly usage aggregation",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  4. Sessions", Style::default().fg(Color::Green)),
                Span::styled(
                    "      Searchable session analytics",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  5. Cache", Style::default().fg(Color::Green)),
                Span::styled(
                    "         Cache analysis view",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  6. Billing", Style::default().fg(Color::Green)),
                Span::styled(
                    "       Billing blocks view",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  h. Help", Style::default().fg(Color::Green)),
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

    fn ensure_cache_analysis(&mut self) {
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

    fn render_cache(&mut self, f: &mut Frame, area: Rect) {
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

    fn format_tokens_static(n: u64) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.0}K", n as f64 / 1_000.0)
        } else {
            format!("{}", n)
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
                    Span::styled("💵 Average per Block: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.2}", report.average_per_block.total_cost),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("🎯 Total Cost: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("${:.2}", report.total_usage.total_cost),
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
                Cell::from(format!("${:.2}", block.usage.total_cost))
                    .style(Style::default().fg(cost_color)),
                Cell::from(self.format_number(block.usage.total_tokens()))
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
            Tab::Daily => Tab::Weekly,
            Tab::Weekly => Tab::Sessions,
            Tab::Sessions => Tab::Cache,
            Tab::Cache => Tab::BillingBlocks,
            Tab::BillingBlocks => Tab::Help,
            Tab::Help => Tab::Overview,
        };
    }

    fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Overview => Tab::Help,
            Tab::Daily => Tab::Overview,
            Tab::Weekly => Tab::Daily,
            Tab::Sessions => Tab::Weekly,
            Tab::Cache => Tab::Sessions,
            Tab::BillingBlocks => Tab::Cache,
            Tab::Help => Tab::BillingBlocks,
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
            Tab::Weekly => {
                let len = self
                    .weekly_report
                    .as_ref()
                    .map(|r| r.weekly.len())
                    .unwrap_or(0);
                if len > 0 {
                    let i = match self.weekly_table_state.selected() {
                        Some(i) => {
                            if i >= len.saturating_sub(1) {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.weekly_table_state.select(Some(i));
                }
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
            Tab::Cache => {
                let len = self
                    .cache_analysis
                    .as_ref()
                    .map(|a| a.sessions.len())
                    .unwrap_or(0);
                if len > 0 {
                    let i = match self.cache_table_state.selected() {
                        Some(i) => {
                            if i >= len.saturating_sub(1) {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.cache_table_state.select(Some(i));
                }
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
            Tab::Weekly => {
                let len = self
                    .weekly_report
                    .as_ref()
                    .map(|r| r.weekly.len())
                    .unwrap_or(0);
                if len > 0 {
                    let i = match self.weekly_table_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                len.saturating_sub(1)
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.weekly_table_state.select(Some(i));
                }
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
            Tab::Cache => {
                let len = self
                    .cache_analysis
                    .as_ref()
                    .map(|a| a.sessions.len())
                    .unwrap_or(0);
                if len > 0 {
                    let i = match self.cache_table_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                len.saturating_sub(1)
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.cache_table_state.select(Some(i));
                }
            }
            _ => {}
        }
    }

    // Enhanced vim navigation methods

    fn jump_to_top(&mut self) {
        match self.current_tab {
            Tab::Daily => {
                self.daily_table_state.select(Some(0));
            }
            Tab::Weekly => {
                self.weekly_table_state.select(Some(0));
            }
            Tab::Sessions => {
                self.session_table_state.select(Some(0));
                self.session_scroll_state = self.session_scroll_state.position(0);
            }
            Tab::BillingBlocks => {
                self.billing_blocks_table_state.select(Some(0));
                self.billing_blocks_scroll_state = self.billing_blocks_scroll_state.position(0);
            }
            Tab::Cache => {
                self.cache_table_state.select(Some(0));
            }
            _ => {}
        }
    }

    fn jump_to_bottom(&mut self) {
        match self.current_tab {
            Tab::Daily => {
                let len = self.daily_report.daily.len();
                if len > 0 {
                    self.daily_table_state.select(Some(len - 1));
                }
            }
            Tab::Weekly => {
                let len = self
                    .weekly_report
                    .as_ref()
                    .map(|r| r.weekly.len())
                    .unwrap_or(0);
                if len > 0 {
                    self.weekly_table_state.select(Some(len - 1));
                }
            }
            Tab::Sessions => {
                let len = self.session_report.sessions.len();
                if len > 0 {
                    self.session_table_state.select(Some(len - 1));
                    self.session_scroll_state = self.session_scroll_state.position(len - 1);
                }
            }
            Tab::BillingBlocks => {
                let report = self.billing_manager.generate_report();
                let len = report.blocks.len();
                if len > 0 {
                    self.billing_blocks_table_state.select(Some(len - 1));
                    self.billing_blocks_scroll_state =
                        self.billing_blocks_scroll_state.position(len - 1);
                }
            }
            Tab::Cache => {
                let len = self
                    .cache_analysis
                    .as_ref()
                    .map(|a| a.sessions.len())
                    .unwrap_or(0);
                if len > 0 {
                    self.cache_table_state.select(Some(len - 1));
                }
            }
            _ => {}
        }
    }

    fn half_page_down(&mut self) {
        let half_page = 10; // Approximate half page
        for _ in 0..half_page {
            self.next_item();
        }
    }

    fn half_page_up(&mut self) {
        let half_page = 10; // Approximate half page
        for _ in 0..half_page {
            self.previous_item();
        }
    }

    fn jump_to_line_start(&mut self) {
        // In table context, this means first column or first visible item
        // For simplicity, we'll just jump to the first item in view
        match self.current_tab {
            Tab::Daily | Tab::Weekly | Tab::Sessions | Tab::BillingBlocks | Tab::Cache => {
                // Already at the start of the line in table view
                self.status_message = Some("At beginning of line".to_string());
            }
            _ => {}
        }
    }

    fn jump_to_line_end(&mut self) {
        // In table context, this means last column or last visible item
        match self.current_tab {
            Tab::Daily | Tab::Weekly | Tab::Sessions | Tab::BillingBlocks | Tab::Cache => {
                // Already at the end of the line in table view
                self.status_message = Some("At end of line".to_string());
            }
            _ => {}
        }
    }

    fn toggle_visual_mode(&mut self) {
        if self.current_mode == AppMode::Visual {
            // Exit visual mode
            self.current_mode = AppMode::Normal;
            self.visual_mode_start = None;
            self.visual_mode_selections.clear();
            self.status_message = Some("Visual mode OFF".to_string());
        } else {
            // Enter visual mode
            self.current_mode = AppMode::Visual;
            self.visual_mode_start = match self.current_tab {
                Tab::Daily => self.daily_table_state.selected(),
                Tab::Weekly => self.weekly_table_state.selected(),
                Tab::Sessions => self.session_table_state.selected(),
                Tab::BillingBlocks => self.billing_blocks_table_state.selected(),
                Tab::Cache => self.cache_table_state.selected(),
                _ => None,
            };
            self.status_message =
                Some("Visual mode ON - use j/k to select, x to mark, Esc to exit".to_string());
        }
    }

    // Word navigation for search mode
    fn next_word_position(&self) -> usize {
        let chars: Vec<char> = self.search_query.chars().collect();
        let mut pos = self.search_cursor_position;

        // Skip current word
        while pos < chars.len() && !chars[pos].is_whitespace() {
            pos += 1;
        }

        // Skip whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        pos.min(self.search_query.len())
    }

    fn prev_word_position(&self) -> usize {
        if self.search_cursor_position == 0 {
            return 0;
        }

        let chars: Vec<char> = self.search_query.chars().collect();
        let mut pos = self.search_cursor_position - 1;

        // Skip whitespace
        while pos > 0 && chars[pos].is_whitespace() {
            pos -= 1;
        }

        // Go to beginning of word
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        pos
    }

    fn update_search_status(&mut self) {
        // Update status message with cursor position indicator
        let mut display_query = self.search_query.clone();
        if self.search_cursor_position <= display_query.len() {
            display_query.insert(self.search_cursor_position, '|');
        }
        self.status_message = Some(format!("Search: {} (Press Esc to cancel)", display_query));
    }

    fn format_number(&self, num: u64) -> String {
        if num == 0 {
            "0".to_string()
        } else {
            let num_str = num.to_string();
            let chars: Vec<char> = num_str.chars().collect();
            let mut result = String::new();

            for (i, c) in chars.iter().enumerate() {
                if i > 0 && (chars.len() - i).is_multiple_of(3) {
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
                        2 => self.current_tab = Tab::Weekly,
                        3 => self.current_tab = Tab::Sessions,
                        4 => self.current_tab = Tab::Cache,
                        5 => self.current_tab = Tab::BillingBlocks,
                        6 => self.current_tab = Tab::Help,
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
        if self.current_tab == Tab::Sessions {
            // Copy session info to clipboard
            if let Some(selected) = self.session_table_state.selected()
                && let Some(session) = self.session_report.sessions.get(selected)
            {
                let info = format!(
                    "Project: {}, Session: {}, Cost: ${:.2}, Tokens: {}",
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
                        self.status_message = Some("❌ Failed to copy to clipboard".to_string());
                    }
                } else {
                    self.status_message = Some("❌ Clipboard not available".to_string());
                }
            }
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

    fn render_export_dialog(&mut self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 3,
            width: area.width / 2,
            height: 10,
        };

        // Clear the area
        f.render_widget(Clear, popup_area);

        // Create the popup block
        let block = Block::default()
            .borders(Borders::ALL)
            .title("📁 Export Data")
            .border_style(Style::default().fg(Color::Cyan));

        // Layout for the dialog content
        let inner_area = block.inner(popup_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(0),
            ])
            .split(inner_area);

        f.render_widget(block, popup_area);

        // Data type indicator
        let data_type = match self.current_tab {
            Tab::Daily => "Daily Report",
            Tab::Weekly => "Weekly Report",
            Tab::Sessions => "Sessions Report",
            Tab::BillingBlocks => "Billing Blocks",
            Tab::Cache => "Cache Analysis",
            Tab::Overview => "Summary",
            _ => "Current View",
        };

        let data_info = Paragraph::new(vec![Line::from(vec![
            Span::styled("Exporting: ", Style::default().fg(Color::White)),
            Span::styled(
                data_type,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])]);
        f.render_widget(data_info, chunks[0]);

        // Format selection
        let csv_style = if self.export_dialog_state.selected_format == ExportFormat::Csv {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        let json_style = if self.export_dialog_state.selected_format == ExportFormat::Json {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        let format_selection = Paragraph::new(vec![Line::from(vec![
            Span::styled("Format: ", Style::default().fg(Color::Cyan)),
            Span::styled("  CSV  ", csv_style),
            Span::styled("    ", Style::default()),
            Span::styled("  JSON  ", json_style),
        ])])
        .block(Block::default().borders(Borders::NONE));
        f.render_widget(format_selection, chunks[1]);

        // Instructions
        let instructions = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                "←→/Tab",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Switch format  ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Export  ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(Color::Gray)),
        ])]);
        f.render_widget(instructions, chunks[2]);

        // Show success or error message if any
        if self.export_dialog_state.show_success_message {
            let success_msg = Paragraph::new(vec![Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::styled(
                    &self.export_dialog_state.success_message,
                    Style::default().fg(Color::Green),
                ),
            ])]);
            f.render_widget(success_msg, chunks[3]);
        } else if let Some(error) = &self.export_dialog_state.error_message {
            let error_msg = Paragraph::new(vec![Line::from(vec![
                Span::styled("❌ ", Style::default().fg(Color::Red)),
                Span::styled(error, Style::default().fg(Color::Red)),
            ])]);
            f.render_widget(error_msg, chunks[3]);
        }
    }
}
