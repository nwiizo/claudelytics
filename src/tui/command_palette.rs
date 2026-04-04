use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::ScrollbarState;

use super::{AppMode, Tab, TuiApp};
use crate::models::CommandAction;

impl TuiApp {
    pub(crate) fn handle_command_palette_input(
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
}
