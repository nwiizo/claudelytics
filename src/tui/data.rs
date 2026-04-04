use anyhow::Result;
use ratatui::widgets::ScrollbarState;

use super::{SortMode, Tab, TimeFilter, TuiApp};

impl TuiApp {
    pub(crate) fn refresh_data(&mut self) -> Result<()> {
        self.status_message = Some("Filters reset to original data".to_string());

        // Reset to original data (does not re-read from disk)
        self.daily_report = self.original_daily_report.clone();
        self.session_report = self.original_session_report.clone();
        self.apply_filters();
        Ok(())
    }

    pub(crate) fn cycle_sort_mode(&mut self) {
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
        self.status_message = Some(format!("\u{1f4ca} Sorted by: {}", mode_str));
    }

    pub(crate) fn cycle_time_filter(&mut self) {
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
        self.status_message = Some(format!("\u{1f4c5} Filter: {}", filter_str));
    }

    pub(crate) fn apply_filters(&mut self) {
        // Reset to original data
        self.daily_report = self.original_daily_report.clone();
        self.session_report = self.original_session_report.clone();

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

    pub(crate) fn bookmark_selected_session(&mut self) {
        if let Some(selected) = self.session_table_state.selected()
            && let Some(session) = self.session_report.sessions.get(selected)
        {
            let session_id = format!("{}/{}", session.project_path, session.session_id);
            if !self.bookmarked_sessions.contains(&session_id) {
                self.bookmarked_sessions.push(session_id.clone());
                self.status_message = Some(format!("\u{1f516} Bookmarked session: {}", session_id));
            } else {
                self.bookmarked_sessions.retain(|s| s != &session_id);
                self.status_message = Some(format!("\u{1f4cc} Removed bookmark: {}", session_id));
            }
        }
    }

    pub(crate) fn toggle_comparison_selection(&mut self) {
        if let Some(selected) = self.session_table_state.selected()
            && let Some(session) = self.session_report.sessions.get(selected)
        {
            let session_id = format!("{}/{}", session.project_path, session.session_id);
            if self.comparison_sessions.contains(&session_id) {
                self.comparison_sessions.retain(|s| s != &session_id);
                self.status_message = Some(format!("Removed from comparison: {}", session_id));
            } else if self.comparison_sessions.len() < 5 {
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

    pub(crate) fn get_current_selected_index(&self) -> Option<usize> {
        match self.current_tab {
            Tab::Daily => self.daily_table_state.selected(),
            Tab::Weekly => self.weekly_table_state.selected(),
            Tab::Sessions => self.session_table_state.selected(),
            Tab::BillingBlocks => self.billing_blocks_table_state.selected(),
            Tab::Cache => self.cache_table_state.selected(),
            _ => None,
        }
    }
}
