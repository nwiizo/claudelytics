use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

use super::{AppMode, ExportFormat, Tab, TuiApp};

impl TuiApp {
    pub(crate) fn open_export_dialog(&mut self) {
        self.current_mode = AppMode::ExportDialog;
        self.export_dialog_state.selected_format = ExportFormat::Csv;
        self.export_dialog_state.show_success_message = false;
        self.export_dialog_state.error_message = None;
        self.status_message = Some(
            "\u{1f4c1} Export: Use arrows to select format, Enter to export, Esc to cancel"
                .to_string(),
        );
    }

    pub(crate) fn handle_export_dialog_input(
        &mut self,
        key: crossterm::event::KeyCode,
    ) -> Result<()> {
        use crossterm::event::KeyCode;
        match key {
            KeyCode::Esc => {
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
                    format!("\u{2705} {} data exported to clipboard as CSV!", data_type);
                self.status_message = Some(self.export_dialog_state.success_message.clone());
                self.current_mode = AppMode::Normal;
            }
            Err(e) => {
                self.export_dialog_state.error_message = Some(format!("Export failed: {}", e));
                self.status_message = Some(format!("\u{274c} Export failed: {}", e));
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
                    format!("\u{2705} {} data exported to clipboard as JSON!", data_type);
                self.status_message = Some(self.export_dialog_state.success_message.clone());
                self.current_mode = AppMode::Normal;
            }
            Err(e) => {
                self.export_dialog_state.error_message = Some(format!("Export failed: {}", e));
                self.status_message = Some(format!("\u{274c} Export failed: {}", e));
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
}
