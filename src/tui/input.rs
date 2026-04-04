use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::{AppMode, Tab, TuiApp};
use crate::tui_visuals::{AnimationStyle, ToastNotification};

impl TuiApp {
    pub(crate) fn handle_normal_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<()> {
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
                self.half_page_down();
            }
            KeyCode::PageUp => {
                self.half_page_up();
            }
            // Enhanced vim navigation
            KeyCode::Char('g') => {
                if modifiers.contains(KeyModifiers::NONE) {
                    if self.g_pressed {
                        self.jump_to_top();
                        self.g_pressed = false;
                        self.status_message = Some("Jumped to top".to_string());
                    } else {
                        self.g_pressed = true;
                        self.status_message = Some("Press 'g' again to jump to top".to_string());
                    }
                }
            }
            KeyCode::Char('G') => {
                self.jump_to_bottom();
            }
            KeyCode::Char('d') => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.half_page_down();
                }
            }
            KeyCode::Char('u') => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.half_page_up();
                }
            }
            KeyCode::Char('0') => {
                self.jump_to_line_start();
            }
            KeyCode::Char('$') => {
                self.jump_to_line_end();
            }
            KeyCode::Char('v') => {
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

    pub(crate) fn handle_search_input(&mut self, key: KeyCode) -> Result<()> {
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
                self.search_query.insert(self.search_cursor_position, c);
                self.search_cursor_position += 1;
                self.update_search_status();
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn handle_visual_mode_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
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
                self.bookmark_visual_selections();
            }
            KeyCode::Char('e') => {
                self.export_visual_selections()?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn update_visual_selection(&mut self) {
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

    pub(crate) fn bookmark_visual_selections(&mut self) {
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
        self.toggle_visual_mode();
    }

    pub(crate) fn export_visual_selections(&mut self) -> Result<()> {
        self.status_message = Some(format!(
            "Would export {} selected items",
            self.visual_mode_selections.len()
        ));
        self.toggle_visual_mode();
        Ok(())
    }

    pub(crate) fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row <= 2 {
                    let tab_width = 16;
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

    pub(crate) fn handle_enter(&mut self) {
        if self.current_tab == Tab::Sessions
            && let Some(selected) = self.session_table_state.selected()
            && let Some(session) = self.session_report.sessions.get(selected)
        {
            let info = format!(
                "Project: {}, Session: {}, Cost: ${:.2}, Tokens: {}",
                session.project_path, session.session_id, session.total_cost, session.total_tokens
            );

            if let Ok(mut ctx) = ClipboardContext::new() {
                if ctx.set_contents(info.clone()).is_ok() {
                    self.status_message =
                        Some("\u{1f4cb} Copied session info to clipboard".to_string());
                } else {
                    self.status_message = Some("\u{274c} Failed to copy to clipboard".to_string());
                }
            } else {
                self.status_message = Some("\u{274c} Clipboard not available".to_string());
            }
        }
    }
}
