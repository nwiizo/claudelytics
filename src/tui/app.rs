use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        poll,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::Rect,
};
use std::io;

use super::{AppMode, ExportDialogState, ExportFormat, Tab, TuiApp};
use crate::billing_blocks::BillingBlockManager;
use crate::models::{Command, CommandAction, DailyReport, SessionReport};
use crate::pricing_cache::PricingCache;
use crate::tui::PricingCacheStatus;
use crate::tui_visuals::VisualEffectsManager;

use ratatui::widgets::{ScrollbarState, TableState};

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
            sort_mode: super::SortMode::Date,
            time_filter: super::TimeFilter::All,
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
        self.status_message = Some("\u{2728} Previous session state restored".to_string());
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
                                    x: (terminal.size()?.width / 2).saturating_sub(2),
                                    y: terminal.size()?.height.saturating_sub(5),
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
}
