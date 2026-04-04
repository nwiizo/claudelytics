use super::{AppMode, Tab, TuiApp};

impl TuiApp {
    pub(crate) fn next_tab(&mut self) {
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

    pub(crate) fn previous_tab(&mut self) {
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

    pub(crate) fn next_item(&mut self) {
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

    pub(crate) fn previous_item(&mut self) {
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

    pub(crate) fn jump_to_top(&mut self) {
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

    pub(crate) fn jump_to_bottom(&mut self) {
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

    pub(crate) fn half_page_down(&mut self) {
        let half_page = 10;
        for _ in 0..half_page {
            self.next_item();
        }
    }

    pub(crate) fn half_page_up(&mut self) {
        let half_page = 10;
        for _ in 0..half_page {
            self.previous_item();
        }
    }

    pub(crate) fn jump_to_line_start(&mut self) {
        match self.current_tab {
            Tab::Daily | Tab::Weekly | Tab::Sessions | Tab::BillingBlocks | Tab::Cache => {
                self.status_message = Some("At beginning of line".to_string());
            }
            _ => {}
        }
    }

    pub(crate) fn jump_to_line_end(&mut self) {
        match self.current_tab {
            Tab::Daily | Tab::Weekly | Tab::Sessions | Tab::BillingBlocks | Tab::Cache => {
                self.status_message = Some("At end of line".to_string());
            }
            _ => {}
        }
    }

    pub(crate) fn toggle_visual_mode(&mut self) {
        if self.current_mode == AppMode::Visual {
            self.current_mode = AppMode::Normal;
            self.visual_mode_start = None;
            self.visual_mode_selections.clear();
            self.status_message = Some("Visual mode OFF".to_string());
        } else {
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
    pub(crate) fn next_word_position(&self) -> usize {
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

    pub(crate) fn prev_word_position(&self) -> usize {
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

    pub(crate) fn update_search_status(&mut self) {
        let mut display_query = self.search_query.clone();
        if self.search_cursor_position <= display_query.len() {
            display_query.insert(self.search_cursor_position, '|');
        }
        self.status_message = Some(format!("Search: {} (Press Esc to cancel)", display_query));
    }
}
