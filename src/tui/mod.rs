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

mod app;
mod command_palette;
mod data;
mod export;
mod helpers;
mod input;
mod navigation;
mod render;
mod tabs;

use crate::billing_blocks::BillingBlockManager;
use crate::cache_analysis::CacheAnalysis;
use crate::models::{Command, DailyReport, SessionReport, WeeklyReport};
use crate::tui_visuals::VisualEffectsManager;

use ratatui::widgets::{ScrollbarState, TableState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Tab {
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
pub(crate) enum AppMode {
    Normal,
    CommandPalette,
    Search,
    Visual,
    ExportDialog,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SortMode {
    Date,
    Cost,
    Tokens,
    Project,
    Efficiency,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TimeFilter {
    All,
    Today,
    LastWeek,
    LastMonth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExportFormat {
    Csv,
    Json,
    Markdown,
    Text,
}

#[derive(Debug)]
pub(crate) struct ExportDialogState {
    selected_format: ExportFormat,
    show_success_message: bool,
    success_message: String,
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PricingCacheStatus {
    exists: bool,
    valid: bool,
    last_updated: String,
    model_count: usize,
}

#[derive(Debug)]
pub struct TuiApp {
    pub(crate) daily_report: DailyReport,
    pub(crate) session_report: SessionReport,
    pub(crate) original_daily_report: DailyReport,
    pub(crate) original_session_report: SessionReport,
    pub(crate) current_tab: Tab,
    pub(crate) current_mode: AppMode,
    pub(crate) daily_table_state: TableState,
    pub(crate) session_table_state: TableState,
    pub(crate) command_table_state: TableState,
    pub(crate) session_scroll_state: ScrollbarState,
    pub(crate) command_scroll_state: ScrollbarState,
    pub(crate) should_quit: bool,
    pub(crate) search_mode: bool,
    pub(crate) search_query: String,
    pub(crate) sort_mode: SortMode,
    pub(crate) time_filter: TimeFilter,
    pub(crate) status_message: Option<String>,
    pub(crate) show_help_popup: bool,
    // Command palette
    pub(crate) command_palette_query: String,
    pub(crate) available_commands: Vec<Command>,
    pub(crate) filtered_commands: Vec<Command>,
    // Enhanced features
    pub(crate) bookmarked_sessions: Vec<String>,
    pub(crate) comparison_sessions: Vec<String>,
    // Billing blocks
    pub(crate) billing_manager: BillingBlockManager,
    pub(crate) billing_blocks_table_state: TableState,
    pub(crate) billing_blocks_scroll_state: ScrollbarState,
    pub(crate) show_billing_summary: bool,
    // Pricing cache status
    pub(crate) pricing_cache_status: Option<PricingCacheStatus>,
    // Visual mode selection
    pub(crate) visual_mode_start: Option<usize>,
    pub(crate) visual_mode_selections: Vec<usize>,
    // Search mode cursor position for word navigation
    pub(crate) search_cursor_position: usize,
    // Track if 'g' was pressed for 'gg' command
    pub(crate) g_pressed: bool,
    // Export dialog state
    pub(crate) export_dialog_state: ExportDialogState,
    // Visual effects manager
    pub(crate) visual_effects: VisualEffectsManager,
    // Previous mode (for returning from dialogs)
    pub(crate) previous_mode: Option<AppMode>,
    // Weekly report (computed lazily)
    pub(crate) weekly_report: Option<WeeklyReport>,
    pub(crate) weekly_table_state: TableState,
    // Cache analysis (computed lazily)
    pub(crate) cache_analysis: Option<CacheAnalysis>,
    pub(crate) cache_table_state: TableState,
}
