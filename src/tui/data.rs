use anyhow::Result;
use ratatui::widgets::ScrollbarState;

use crate::models::{SessionUsage, TokenUsageTotals};

use super::{SortMode, Tab, TuiApp};

impl TuiApp {
    pub(crate) fn refresh_data(&mut self) -> Result<()> {
        self.status_message = Some("Loaded data reset".to_string());

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

    pub(crate) fn apply_filters(&mut self) {
        // Reset to original data
        self.daily_report = self.original_daily_report.clone();
        self.session_report = self.original_session_report.clone();

        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            self.session_report.sessions.retain(|session| {
                session.project_path.to_lowercase().contains(&query)
                    || session.session_id.to_lowercase().contains(&query)
            });
        }

        self.session_report.totals = session_totals(&self.session_report.sessions);

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
                    .sort_by_key(|entry| std::cmp::Reverse(entry.total_tokens));
                self.session_report
                    .sessions
                    .sort_by_key(|entry| std::cmp::Reverse(entry.total_tokens));
            }
            SortMode::Efficiency => {
                self.daily_report.daily.sort_by(|a, b| {
                    let ra = Self::tokens_per_dollar(a.total_tokens, a.total_cost);
                    let rb = Self::tokens_per_dollar(b.total_tokens, b.total_cost);
                    rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
                });
                self.session_report.sessions.sort_by(|a, b| {
                    let ra = Self::tokens_per_dollar(a.total_tokens, a.total_cost);
                    let rb = Self::tokens_per_dollar(b.total_tokens, b.total_cost);
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
            Tab::Models => self.model_table_state.selected(),
            Tab::Sessions => self.session_table_state.selected(),
            Tab::BillingBlocks => self.billing_blocks_table_state.selected(),
            Tab::Cache => self.cache_table_state.selected(),
            _ => None,
        }
    }
}

fn session_totals(entries: &[SessionUsage]) -> TokenUsageTotals {
    entries.iter().fold(empty_totals(), |mut totals, entry| {
        add_totals(
            &mut totals,
            &TokenUsageTotals {
                input_tokens: entry.input_tokens,
                output_tokens: entry.output_tokens,
                cache_creation_tokens: entry.cache_creation_tokens,
                cache_read_tokens: entry.cache_read_tokens,
                total_tokens: entry.total_tokens,
                total_cost: entry.total_cost,
            },
        );
        totals
    })
}

fn add_totals(totals: &mut TokenUsageTotals, usage: &TokenUsageTotals) {
    totals.input_tokens = totals.input_tokens.saturating_add(usage.input_tokens);
    totals.output_tokens = totals.output_tokens.saturating_add(usage.output_tokens);
    totals.cache_creation_tokens = totals
        .cache_creation_tokens
        .saturating_add(usage.cache_creation_tokens);
    totals.cache_read_tokens = totals
        .cache_read_tokens
        .saturating_add(usage.cache_read_tokens);
    totals.total_tokens = totals.total_tokens.saturating_add(usage.total_tokens);
    totals.total_cost += usage.total_cost;
}

fn empty_totals() -> TokenUsageTotals {
    TokenUsageTotals {
        input_tokens: 0,
        output_tokens: 0,
        cache_creation_tokens: 0,
        cache_read_tokens: 0,
        total_tokens: 0,
        total_cost: 0.0,
    }
}
