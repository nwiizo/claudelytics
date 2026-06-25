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
use std::sync::mpsc::{Receiver, TryRecvError};

use super::{AppMode, ExportDialogState, ExportFormat, Tab, TuiApp};
use crate::billing_blocks::BillingBlockManager;
use crate::models::{Command, CommandAction, DailyReport, ModelReport, SessionReport};
use crate::pricing_cache::PricingCache;
use crate::tui::PricingCacheStatus;
use crate::tui_visuals::VisualEffectsManager;

use ratatui::widgets::{ScrollbarState, TableState};

impl TuiApp {
    pub fn new(
        daily_report: DailyReport,
        session_report: SessionReport,
        model_report: ModelReport,
        billing_manager: BillingBlockManager,
    ) -> Self {
        let mut daily_table_state = TableState::default();
        daily_table_state.select(Some(0));

        let mut session_table_state = TableState::default();
        session_table_state.select(Some(0));

        let mut model_table_state = TableState::default();
        model_table_state.select(Some(0));

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
            model_report,
            model_report_loading: false,
            model_report_error: None,
            model_report_receiver: None,
            original_daily_report: daily_report,
            original_session_report: session_report,
            current_tab: Tab::Overview,
            current_mode: AppMode::Normal,
            daily_table_state,
            session_table_state,
            model_table_state,
            command_table_state: TableState::default(),
            session_scroll_state,
            command_scroll_state: ScrollbarState::new(0),
            should_quit: false,
            search_mode: false,
            search_query: String::new(),
            sort_mode: super::SortMode::Date,
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

    pub fn set_model_report_receiver(
        &mut self,
        receiver: Receiver<std::result::Result<ModelReport, String>>,
    ) {
        self.model_report_loading = true;
        self.model_report_error = None;
        self.model_report_receiver = Some(receiver);
        self.status_message = Some("Loading model usage in background...".to_string());
    }

    pub(crate) fn poll_model_report(&mut self) {
        let Some(result) = self.model_report_receiver.as_ref().map(|rx| rx.try_recv()) else {
            return;
        };

        match result {
            Ok(Ok(report)) => {
                self.model_report = report;
                self.model_table_state
                    .select(if self.model_report.models.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                self.model_report_loading = false;
                self.model_report_error = None;
                self.model_report_receiver = None;
                self.status_message = Some("Model usage loaded".to_string());
            }
            Ok(Err(error)) => {
                self.model_report_loading = false;
                self.model_report_error = Some(error.clone());
                self.model_report_receiver = None;
                self.status_message = Some(format!("Model usage failed: {error}"));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.model_report_loading = false;
                self.model_report_error =
                    Some("Model usage loader disconnected unexpectedly".to_string());
                self.model_report_receiver = None;
            }
        }
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
            3 => Tab::Models,
            4 => Tab::Sessions,
            5 => Tab::Cache,
            6 => Tab::BillingBlocks,
            7 => Tab::Help,
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
                name: "Switch to Weekly".to_string(),
                description: "Go to weekly usage tab".to_string(),
                shortcut: Some("3".to_string()),
                action: CommandAction::SwitchTab(2),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Models".to_string(),
                description: "Go to model and agent breakdown tab".to_string(),
                shortcut: Some("4".to_string()),
                action: CommandAction::SwitchTab(3),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Sessions".to_string(),
                description: "Go to sessions tab".to_string(),
                shortcut: Some("5".to_string()),
                action: CommandAction::SwitchTab(4),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Cache".to_string(),
                description: "Go to cache analysis tab".to_string(),
                shortcut: Some("6".to_string()),
                action: CommandAction::SwitchTab(5),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Billing".to_string(),
                description: "Go to billing blocks tab".to_string(),
                shortcut: Some("7".to_string()),
                action: CommandAction::SwitchTab(6),
                category: "Navigation".to_string(),
            },
            Command {
                name: "Switch to Help".to_string(),
                description: "Go to help tab".to_string(),
                shortcut: Some("h".to_string()),
                action: CommandAction::SwitchTab(7),
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

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        B::Error: std::error::Error + Send + Sync + 'static,
    {
        loop {
            self.poll_model_report();

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
                                let terminal_size = terminal.size()?;
                                let key_str = if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    format!("Ctrl+{}", c)
                                } else {
                                    c.to_string()
                                };
                                let effect_pos = Rect {
                                    x: (terminal_size.width / 2).saturating_sub(2),
                                    y: terminal_size.height.saturating_sub(5),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DailyUsage, ModelUsage, SessionUsage, TokenUsageTotals};
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::{Duration, Instant};

    fn empty_daily_report() -> DailyReport {
        DailyReport {
            daily: Vec::new(),
            totals: TokenUsageTotals {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 0,
                total_cost: 0.0,
            },
        }
    }

    fn empty_session_report() -> SessionReport {
        SessionReport {
            sessions: Vec::new(),
            totals: TokenUsageTotals {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 0,
                total_cost: 0.0,
            },
        }
    }

    fn empty_model_report() -> ModelReport {
        ModelReport {
            models: Vec::new(),
            totals: TokenUsageTotals {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 0,
                total_cost: 0.0,
            },
        }
    }

    fn audit_app() -> TuiApp {
        let daily_report = DailyReport {
            daily: vec![
                DailyUsage {
                    date: "2026-07-26".to_string(),
                    input_tokens: 1_000,
                    output_tokens: 200,
                    cache_creation_tokens: 100,
                    cache_read_tokens: 400,
                    total_tokens: 1_700,
                    total_cost: 1.25,
                },
                DailyUsage {
                    date: "2026-07-10".to_string(),
                    input_tokens: 500,
                    output_tokens: 100,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 100,
                    total_tokens: 700,
                    total_cost: 0.75,
                },
            ],
            totals: empty_daily_report().totals,
        };
        let session_report = SessionReport {
            sessions: vec![
                SessionUsage {
                    project_path: "-github-com-example-project".to_string(),
                    session_id: "claude-session".to_string(),
                    input_tokens: 700,
                    output_tokens: 100,
                    cache_creation_tokens: 100,
                    cache_read_tokens: 200,
                    total_tokens: 1_100,
                    total_cost: 0.85,
                    last_activity: "2026-07-26".to_string(),
                },
                SessionUsage {
                    project_path: "Codex".to_string(),
                    session_id: "codex-session".to_string(),
                    input_tokens: 800,
                    output_tokens: 200,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 300,
                    total_tokens: 1_300,
                    total_cost: 1.15,
                    last_activity: "2026-07-10".to_string(),
                },
            ],
            totals: empty_session_report().totals,
        };
        let model_report = ModelReport {
            models: vec![
                ModelUsage {
                    agent: "Claude Code".to_string(),
                    model: "claude-fable-5".to_string(),
                    family: "fable".to_string(),
                    input_tokens: 700,
                    output_tokens: 100,
                    cache_creation_tokens: 100,
                    cache_read_tokens: 200,
                    total_tokens: 1_100,
                    total_cost: 0.85,
                },
                ModelUsage {
                    agent: "Codex".to_string(),
                    model: "gpt-5.6-sol".to_string(),
                    family: "openai".to_string(),
                    input_tokens: 800,
                    output_tokens: 200,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 300,
                    total_tokens: 1_300,
                    total_cost: 1.15,
                },
            ],
            totals: TokenUsageTotals {
                input_tokens: 1_500,
                output_tokens: 300,
                cache_creation_tokens: 100,
                cache_read_tokens: 500,
                total_tokens: 2_400,
                total_cost: 2.0,
            },
        };
        TuiApp::new(
            daily_report,
            session_report,
            model_report,
            BillingBlockManager::new(),
        )
    }

    fn rendered_text(app: &mut TuiApp, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        terminal
            .draw(|frame| app.ui(frame))
            .expect("overview should render");
        terminal
            .backend()
            .buffer()
            .content()
            .chunks(width as usize)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn overview_is_readable_at_supported_terminal_sizes() {
        for (width, height) in [
            (80, 24),
            (96, 28),
            (100, 28),
            (104, 28),
            (112, 30),
            (120, 32),
            (160, 40),
        ] {
            let text = rendered_text(&mut audit_app(), width, height);
            assert!(text.contains("Audit overview"), "{width}x{height}");
            assert!(text.contains("Agent split"), "{width}x{height}");
            assert!(text.contains("Claude Code"), "{width}x{height}");
            assert!(text.contains("Codex"), "{width}x{height}");
            assert!(text.contains("Top models"), "{width}x{height}");
            assert!(text.contains("Daily cost trend"), "{width}x{height}");
        }
    }

    #[test]
    fn model_view_keeps_both_agents_visible_and_explains_cache_writes() {
        for (width, height) in [
            (80, 24),
            (96, 28),
            (100, 28),
            (104, 28),
            (112, 30),
            (120, 32),
            (160, 40),
        ] {
            let mut app = audit_app();
            app.current_tab = Tab::Models;
            let text = rendered_text(&mut app, width, height);
            assert!(text.contains("Claude Code"), "{width}x{height}");
            assert!(text.contains("Codex"), "{width}x{height}");
            assert!(text.contains("claude-fable-5"), "{width}x{height}");
            assert!(text.contains("gpt-5.6-sol"), "{width}x{height}");
            assert!(text.contains("TOTAL"), "{width}x{height}");
            assert!(text.contains("Codex W — if unreported"), "{width}x{height}");
            if width >= 112 {
                assert!(text.contains("Cache W"), "{width}x{height}");
                assert!(text.contains("Cache R"), "{width}x{height}");
            }
        }
    }

    #[test]
    fn test_model_report_receiver_does_not_block_tui_polling() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut app = TuiApp::new(
            empty_daily_report(),
            empty_session_report(),
            empty_model_report(),
            BillingBlockManager::new(),
        );
        app.set_model_report_receiver(receiver);

        let start = Instant::now();
        app.poll_model_report();
        let elapsed = start.elapsed();

        assert!(app.model_report_loading);
        assert!(
            elapsed < Duration::from_millis(10),
            "empty model receiver poll took {elapsed:?}"
        );

        sender
            .send(Ok(ModelReport {
                models: vec![ModelUsage {
                    agent: "Codex".to_string(),
                    model: "gpt-5-codex".to_string(),
                    family: "openai".to_string(),
                    input_tokens: 10,
                    output_tokens: 5,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 2,
                    total_tokens: 17,
                    total_cost: 0.0001,
                }],
                totals: TokenUsageTotals {
                    input_tokens: 10,
                    output_tokens: 5,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 2,
                    total_tokens: 17,
                    total_cost: 0.0001,
                },
            }))
            .expect("Failed to send model report");

        app.poll_model_report();

        assert!(!app.model_report_loading);
        assert!(app.model_report_error.is_none());
        assert_eq!(app.model_report.models.len(), 1);
        assert_eq!(app.model_table_state.selected(), Some(0));
    }
}
