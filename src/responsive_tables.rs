use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

use crate::billing_blocks::BillingBlock;
use crate::models::{DailyReport, SessionReport, TokenUsageTotals};
use crate::terminal::Terminal;

/// Table display configuration based on terminal width
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableMode {
    /// Ultra compact mode for very narrow terminals (< 60 chars)
    UltraCompact,
    /// Compact mode for narrow terminals (60-80 chars)
    Compact,
    /// Normal mode for standard terminals (80-120 chars)
    Normal,
    /// Wide mode for wide terminals (120-160 chars)
    Wide,
    /// Full mode for very wide terminals (> 160 chars)
    Full,
}

impl TableMode {
    /// Detect appropriate table mode based on terminal width
    pub fn detect() -> Self {
        let width = Terminal::width();
        match width {
            0..=59 => TableMode::UltraCompact,
            60..=79 => TableMode::Compact,
            80..=119 => TableMode::Normal,
            120..=159 => TableMode::Wide,
            _ => TableMode::Full,
        }
    }

    /// Get minimum required width for this mode
    #[allow(dead_code)]
    pub fn min_width(&self) -> u16 {
        match self {
            TableMode::UltraCompact => 50,
            TableMode::Compact => 60,
            TableMode::Normal => 80,
            TableMode::Wide => 120,
            TableMode::Full => 160,
        }
    }
}

/// Column definition with priority and formatting options
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TableColumn {
    /// Column identifier
    pub id: &'static str,
    /// Display header (can be abbreviated in compact modes)
    pub header: &'static str,
    /// Abbreviated header for compact display
    pub header_short: &'static str,
    /// Column priority (1 = highest, always show)
    pub priority: u8,
    /// Minimum width required for this column
    pub min_width: u16,
    /// Whether this column can be merged with another
    pub can_merge: bool,
    /// Column to merge with (if any)
    pub merge_with: Option<&'static str>,
}

/// Responsive table builder
pub struct ResponsiveTable {
    mode: TableMode,
    #[allow(dead_code)]
    columns: Vec<TableColumn>,
}

impl ResponsiveTable {
    /// Create a new responsive table with auto-detected mode
    pub fn new() -> Self {
        Self {
            mode: TableMode::detect(),
            columns: Vec::new(),
        }
    }

    /// Create a new responsive table with specific mode
    #[allow(dead_code)]
    pub fn with_mode(mode: TableMode) -> Self {
        Self {
            mode,
            columns: Vec::new(),
        }
    }

    /// Define columns for daily report
    pub fn daily_columns() -> Vec<TableColumn> {
        vec![
            TableColumn {
                id: "date",
                header: "Date",
                header_short: "Date",
                priority: 1,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "cost",
                header: "Cost (USD)",
                header_short: "Cost",
                priority: 1,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "total_tokens",
                header: "Total Tokens",
                header_short: "Tokens",
                priority: 2,
                min_width: 12,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "input_tokens",
                header: "Input Tokens",
                header_short: "Input",
                priority: 3,
                min_width: 12,
                can_merge: true,
                merge_with: Some("output_tokens"),
            },
            TableColumn {
                id: "output_tokens",
                header: "Output Tokens",
                header_short: "Output",
                priority: 3,
                min_width: 12,
                can_merge: true,
                merge_with: Some("input_tokens"),
            },
            TableColumn {
                id: "cache_tokens",
                header: "Cache Tokens",
                header_short: "Cache",
                priority: 4,
                min_width: 12,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "efficiency",
                header: "Efficiency",
                header_short: "Eff",
                priority: 5,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "ratio",
                header: "O/I Ratio",
                header_short: "Ratio",
                priority: 5,
                min_width: 8,
                can_merge: false,
                merge_with: None,
            },
        ]
    }

    /// Define columns for session report
    pub fn session_columns() -> Vec<TableColumn> {
        vec![
            TableColumn {
                id: "session",
                header: "Session Path",
                header_short: "Session",
                priority: 1,
                min_width: 20,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "cost",
                header: "Cost (USD)",
                header_short: "Cost",
                priority: 1,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "total_tokens",
                header: "Total Tokens",
                header_short: "Tokens",
                priority: 2,
                min_width: 12,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "input_tokens",
                header: "Input Tokens",
                header_short: "Input",
                priority: 3,
                min_width: 12,
                can_merge: true,
                merge_with: Some("output_tokens"),
            },
            TableColumn {
                id: "output_tokens",
                header: "Output Tokens",
                header_short: "Output",
                priority: 3,
                min_width: 12,
                can_merge: true,
                merge_with: Some("input_tokens"),
            },
            TableColumn {
                id: "last_activity",
                header: "Last Activity",
                header_short: "Activity",
                priority: 4,
                min_width: 19,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "cache_tokens",
                header: "Cache Tokens",
                header_short: "Cache",
                priority: 5,
                min_width: 12,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "efficiency",
                header: "Efficiency",
                header_short: "Eff",
                priority: 6,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
        ]
    }

    /// Define columns for billing blocks report
    pub fn billing_columns() -> Vec<TableColumn> {
        vec![
            TableColumn {
                id: "date",
                header: "Date",
                header_short: "Date",
                priority: 1,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "block",
                header: "Time Block",
                header_short: "Block",
                priority: 1,
                min_width: 11,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "cost",
                header: "Cost (USD)",
                header_short: "Cost",
                priority: 1,
                min_width: 10,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "total_tokens",
                header: "Total Tokens",
                header_short: "Tokens",
                priority: 2,
                min_width: 12,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "sessions",
                header: "Sessions",
                header_short: "Sess",
                priority: 3,
                min_width: 8,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "io_tokens",
                header: "I/O Tokens",
                header_short: "I/O",
                priority: 4,
                min_width: 20,
                can_merge: false,
                merge_with: None,
            },
            TableColumn {
                id: "cache_tokens",
                header: "Cache Tokens",
                header_short: "Cache",
                priority: 5,
                min_width: 12,
                can_merge: false,
                merge_with: None,
            },
        ]
    }

    /// Get visible columns based on terminal width and priorities
    fn get_visible_columns(&self, all_columns: &[TableColumn]) -> Vec<TableColumn> {
        let terminal_width = Terminal::width();
        let mut visible_columns = Vec::new();
        let mut used_width = 0u16;

        // Always include priority 1 columns
        for col in all_columns.iter().filter(|c| c.priority == 1) {
            used_width += col.min_width + 3; // Add padding
            visible_columns.push(col.clone());
        }

        // Add columns by priority until we run out of space
        for priority in 2..=6 {
            let priority_columns: Vec<_> = all_columns
                .iter()
                .filter(|c| c.priority == priority)
                .collect();

            // Check if we can fit all columns at this priority
            let priority_width: u16 = priority_columns.iter().map(|c| c.min_width + 3).sum();

            if used_width + priority_width <= terminal_width {
                // Add all columns at this priority
                for col in priority_columns {
                    visible_columns.push(col.clone());
                    used_width += col.min_width + 3;
                }
            } else if self.mode == TableMode::Compact || self.mode == TableMode::UltraCompact {
                // In compact modes, check if we can merge columns
                let mergeable: Vec<_> = priority_columns.iter().filter(|c| c.can_merge).collect();

                if mergeable.len() >= 2 {
                    // Create a merged column
                    let merged = TableColumn {
                        id: "io_merged",
                        header: "I/O Tokens",
                        header_short: "I/O",
                        priority,
                        min_width: 15,
                        can_merge: false,
                        merge_with: None,
                    };

                    if used_width + merged.min_width + 3 <= terminal_width {
                        used_width += merged.min_width + 3;
                        visible_columns.push(merged);
                    }
                }
            }
        }

        visible_columns
    }

    /// Display daily report with responsive layout
    pub fn display_daily_report(&self, report: &DailyReport) {
        let columns = Self::daily_columns();
        let visible_columns = self.get_visible_columns(&columns);

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS);

        // Set headers based on mode
        let headers: Vec<Cell> = visible_columns
            .iter()
            .map(|col| {
                let header_text =
                    if self.mode == TableMode::UltraCompact || self.mode == TableMode::Compact {
                        col.header_short
                    } else {
                        col.header
                    };
                Cell::new(header_text).fg(Color::Cyan)
            })
            .collect();

        table.set_header(headers);

        // Add data rows
        for daily in &report.daily {
            let mut row: Vec<Cell> = Vec::new();

            for col in &visible_columns {
                match col.id {
                    "date" => row.push(Cell::new(&daily.date)),
                    "cost" => {
                        row.push(Cell::new(format_currency(daily.total_cost)).fg(Color::Green))
                    }
                    "total_tokens" => {
                        row.push(Cell::new(format_number(daily.total_tokens)).fg(Color::Magenta))
                    }
                    "input_tokens" => {
                        row.push(Cell::new(format_number(daily.input_tokens)).fg(Color::Blue))
                    }
                    "output_tokens" => {
                        row.push(Cell::new(format_number(daily.output_tokens)).fg(Color::Cyan))
                    }
                    "cache_tokens" => {
                        let cache_total = daily.cache_creation_tokens + daily.cache_read_tokens;
                        row.push(Cell::new(format_number(cache_total)).fg(Color::Yellow));
                    }
                    "efficiency" => {
                        let eff = if daily.total_cost > 0.0 {
                            daily.total_tokens as f64 / daily.total_cost
                        } else {
                            0.0
                        };
                        row.push(Cell::new(format!("{:.0} tok/$", eff)).fg(Color::Green));
                    }
                    "ratio" => {
                        let ratio = if daily.input_tokens > 0 {
                            daily.output_tokens as f64 / daily.input_tokens as f64
                        } else {
                            0.0
                        };
                        row.push(Cell::new(format!("{:.1}:1", ratio)).fg(Color::Yellow));
                    }
                    "io_merged" => {
                        // Merged I/O column for compact display
                        let text = format!(
                            "{}/{}",
                            format_number_short(daily.input_tokens),
                            format_number_short(daily.output_tokens)
                        );
                        row.push(Cell::new(text).fg(Color::Blue));
                    }
                    _ => {}
                }
            }

            table.add_row(row);
        }

        // Add totals row if not ultra compact
        if self.mode != TableMode::UltraCompact && !report.daily.is_empty() {
            let mut totals_row: Vec<Cell> = Vec::new();

            for col in &visible_columns {
                match col.id {
                    "date" => totals_row.push(Cell::new("Total").fg(Color::Yellow)),
                    "cost" => totals_row.push(
                        Cell::new(format_currency(report.totals.total_cost)).fg(Color::Yellow),
                    ),
                    "total_tokens" => totals_row.push(
                        Cell::new(format_number(report.totals.total_tokens)).fg(Color::Yellow),
                    ),
                    "input_tokens" => totals_row.push(
                        Cell::new(format_number(report.totals.input_tokens)).fg(Color::Yellow),
                    ),
                    "output_tokens" => totals_row.push(
                        Cell::new(format_number(report.totals.output_tokens)).fg(Color::Yellow),
                    ),
                    "cache_tokens" => {
                        let cache_total =
                            report.totals.cache_creation_tokens + report.totals.cache_read_tokens;
                        totals_row.push(Cell::new(format_number(cache_total)).fg(Color::Yellow));
                    }
                    "efficiency" => {
                        let eff = if report.totals.total_cost > 0.0 {
                            report.totals.total_tokens as f64 / report.totals.total_cost
                        } else {
                            0.0
                        };
                        totals_row.push(Cell::new(format!("{:.0} tok/$", eff)).fg(Color::Yellow));
                    }
                    "ratio" => {
                        let ratio = if report.totals.input_tokens > 0 {
                            report.totals.output_tokens as f64 / report.totals.input_tokens as f64
                        } else {
                            0.0
                        };
                        totals_row.push(Cell::new(format!("{:.1}:1", ratio)).fg(Color::Yellow));
                    }
                    "io_merged" => {
                        let text = format!(
                            "{}/{}",
                            format_number_short(report.totals.input_tokens),
                            format_number_short(report.totals.output_tokens)
                        );
                        totals_row.push(Cell::new(text).fg(Color::Yellow));
                    }
                    _ => {}
                }
            }

            table.add_row(totals_row);
        }

        println!("{}", table);
    }

    /// Display session report with responsive layout
    pub fn display_session_report(&self, report: &SessionReport) {
        let columns = Self::session_columns();
        let visible_columns = self.get_visible_columns(&columns);

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS);

        // Set headers based on mode
        let headers: Vec<Cell> = visible_columns
            .iter()
            .map(|col| {
                let header_text =
                    if self.mode == TableMode::UltraCompact || self.mode == TableMode::Compact {
                        col.header_short
                    } else {
                        col.header
                    };
                Cell::new(header_text).fg(Color::Cyan)
            })
            .collect();

        table.set_header(headers);

        // Add data rows
        for session in &report.sessions {
            let mut row: Vec<Cell> = Vec::new();

            for col in &visible_columns {
                match col.id {
                    "session" => {
                        let path = format!("{}/{}", session.project_path, session.session_id);
                        let truncated = truncate_path(&path, col.min_width as usize);
                        row.push(Cell::new(truncated));
                    }
                    "cost" => {
                        row.push(Cell::new(format_currency(session.total_cost)).fg(Color::Green))
                    }
                    "total_tokens" => {
                        row.push(Cell::new(format_number(session.total_tokens)).fg(Color::Magenta))
                    }
                    "input_tokens" => {
                        row.push(Cell::new(format_number(session.input_tokens)).fg(Color::Blue))
                    }
                    "output_tokens" => {
                        row.push(Cell::new(format_number(session.output_tokens)).fg(Color::Cyan))
                    }
                    "last_activity" => {
                        row.push(Cell::new(&session.last_activity).fg(Color::Yellow))
                    }
                    "cache_tokens" => {
                        let cache_total = session.cache_creation_tokens + session.cache_read_tokens;
                        row.push(Cell::new(format_number(cache_total)).fg(Color::Yellow));
                    }
                    "efficiency" => {
                        let eff = if session.total_cost > 0.0 {
                            session.total_tokens as f64 / session.total_cost
                        } else {
                            0.0
                        };
                        row.push(Cell::new(format!("{:.0} tok/$", eff)).fg(Color::Green));
                    }
                    "io_merged" => {
                        let text = format!(
                            "{}/{}",
                            format_number_short(session.input_tokens),
                            format_number_short(session.output_tokens)
                        );
                        row.push(Cell::new(text).fg(Color::Blue));
                    }
                    _ => {}
                }
            }

            table.add_row(row);
        }

        println!("{}", table);
    }

    /// Display billing blocks with responsive layout
    pub fn display_billing_blocks(&self, blocks: &[(chrono::NaiveDate, &BillingBlock)]) {
        let columns = Self::billing_columns();
        let visible_columns = self.get_visible_columns(&columns);

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS);

        // Set headers based on mode
        let headers: Vec<Cell> = visible_columns
            .iter()
            .map(|col| {
                let header_text =
                    if self.mode == TableMode::UltraCompact || self.mode == TableMode::Compact {
                        col.header_short
                    } else {
                        col.header
                    };
                Cell::new(header_text).fg(Color::Cyan)
            })
            .collect();

        table.set_header(headers);

        // Add data rows
        for (date, block) in blocks {
            let mut row: Vec<Cell> = Vec::new();

            for col in &visible_columns {
                match col.id {
                    "date" => row.push(Cell::new(date.to_string())),
                    "block" => row.push(Cell::new(block.label()).fg(Color::Blue)),
                    "cost" => row
                        .push(Cell::new(format_currency(block.usage.total_cost)).fg(Color::Green)),
                    "total_tokens" => row.push(
                        Cell::new(format_number(block.usage.total_tokens())).fg(Color::Magenta),
                    ),
                    "sessions" => {
                        row.push(Cell::new(block.session_count.to_string()).fg(Color::Yellow))
                    }
                    "io_tokens" => {
                        let text = format!(
                            "{}/{}",
                            format_number_short(block.usage.input_tokens),
                            format_number_short(block.usage.output_tokens)
                        );
                        row.push(Cell::new(text).fg(Color::Blue));
                    }
                    "cache_tokens" => {
                        let cache_total =
                            block.usage.cache_creation_tokens + block.usage.cache_read_tokens;
                        row.push(Cell::new(format_number(cache_total)).fg(Color::Yellow));
                    }
                    _ => {}
                }
            }

            table.add_row(row);
        }

        println!("{}", table);
    }
}

/// Display responsive summary card that adapts to terminal width
pub fn display_responsive_summary(totals: &TokenUsageTotals, context_info: &str) {
    let mode = TableMode::detect();
    let terminal_width = Terminal::width();

    match mode {
        TableMode::UltraCompact => display_ultra_compact_summary(totals, context_info),
        TableMode::Compact => display_compact_summary(totals, context_info),
        _ => display_normal_summary(totals, context_info, terminal_width),
    }
}

/// Ultra compact summary for very narrow terminals
fn display_ultra_compact_summary(totals: &TokenUsageTotals, context_info: &str) {
    println!("{}", "Summary".bright_yellow().bold());
    println!("{}", "-".repeat(40));
    println!(
        "Cost: {}",
        format_currency(totals.total_cost).bright_green()
    );
    println!(
        "Tokens: {}",
        format_number_short(totals.total_tokens).bright_magenta()
    );
    println!("{}", context_info);
    println!("{}", "-".repeat(40));
}

/// Compact summary for narrow terminals
fn display_compact_summary(totals: &TokenUsageTotals, context_info: &str) {
    let cost_str = format_currency(totals.total_cost);
    let tokens_str = format_number_short(totals.total_tokens);

    println!("{}", "━".repeat(60).bright_black());
    println!(
        "{} {}",
        "💰 Summary".bright_yellow().bold(),
        context_info.dimmed()
    );
    println!("{}", "─".repeat(60).bright_black());
    println!(
        "Cost: {} | Tokens: {}",
        cost_str.bright_green().bold(),
        tokens_str.bright_magenta().bold()
    );
    println!(
        "I/O: {}/{}",
        format_number_short(totals.input_tokens).green(),
        format_number_short(totals.output_tokens).blue()
    );
    println!("{}", "━".repeat(60).bright_black());
}

/// Normal summary for standard and wide terminals
fn display_normal_summary(totals: &TokenUsageTotals, context_info: &str, width: u16) {
    let separator = "═".repeat(width as usize);

    println!("{}", separator.bright_black());
    println!(
        "{}  {}",
        "💰 USAGE SUMMARY".bright_yellow().bold(),
        context_info.dimmed()
    );
    println!("{}", separator.bright_black());

    // Calculate metrics
    let efficiency = if totals.total_cost > 0.0 {
        totals.total_tokens as f64 / totals.total_cost
    } else {
        0.0
    };

    let output_input_ratio = if totals.input_tokens > 0 {
        totals.output_tokens as f64 / totals.input_tokens as f64
    } else {
        0.0
    };

    let cache_efficiency = if (totals.cache_read_tokens + totals.input_tokens) > 0 {
        totals.cache_read_tokens as f64 / (totals.cache_read_tokens + totals.input_tokens) as f64
            * 100.0
    } else {
        0.0
    };

    println!(
        "Total Cost: {} | Total Tokens: {} | Efficiency: {} tok/$",
        format_currency(totals.total_cost).bright_green().bold(),
        format_number(totals.total_tokens).bright_magenta().bold(),
        format!("{:.0}", efficiency).bright_cyan().bold()
    );

    if width >= 100 {
        println!(
            "Input: {} | Output: {} | Cache: {} | O/I Ratio: {:.1}:1 | Cache Hit: {:.1}%",
            format_number(totals.input_tokens).green(),
            format_number(totals.output_tokens).blue(),
            format_number(totals.cache_creation_tokens + totals.cache_read_tokens).yellow(),
            output_input_ratio,
            cache_efficiency
        );
    }

    println!("{}", separator.bright_black());
}

/// Helper functions for formatting
fn format_number(num: u64) -> String {
    if num == 0 {
        "0".to_string()
    } else {
        let num_str = num.to_string();
        let chars: Vec<char> = num_str.chars().collect();
        let mut result = String::new();

        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*c);
        }

        result
    }
}

/// Format number in short form (K, M, B)
fn format_number_short(num: u64) -> String {
    if num >= 1_000_000_000 {
        format!("{:.1}B", num as f64 / 1_000_000_000.0)
    } else if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

fn format_currency(amount: f64) -> String {
    if amount >= 100.0 {
        format!("${:.2}", amount)
    } else {
        format!("${:.4}", amount)
    }
}

fn truncate_path(path: &str, max_length: usize) -> String {
    if path.len() <= max_length {
        path.to_string()
    } else {
        format!("...{}", &path[path.len().saturating_sub(max_length - 3)..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_mode_detection() {
        // Test mode detection based on width
        assert_eq!(TableMode::detect().min_width() <= Terminal::width(), true);
    }

    #[test]
    fn test_format_number_short() {
        assert_eq!(format_number_short(0), "0");
        assert_eq!(format_number_short(999), "999");
        assert_eq!(format_number_short(1_000), "1.0K");
        assert_eq!(format_number_short(1_500), "1.5K");
        assert_eq!(format_number_short(1_000_000), "1.0M");
        assert_eq!(format_number_short(1_500_000), "1.5M");
        assert_eq!(format_number_short(1_000_000_000), "1.0B");
    }

    #[test]
    fn test_column_priorities() {
        let daily_cols = ResponsiveTable::daily_columns();

        // Check that date and cost have highest priority
        assert_eq!(
            daily_cols.iter().find(|c| c.id == "date").unwrap().priority,
            1
        );
        assert_eq!(
            daily_cols.iter().find(|c| c.id == "cost").unwrap().priority,
            1
        );

        // Check that cache tokens have lower priority
        assert!(
            daily_cols
                .iter()
                .find(|c| c.id == "cache_tokens")
                .unwrap()
                .priority
                > 3
        );
    }

    #[test]
    fn test_column_merging() {
        let cols = ResponsiveTable::daily_columns();

        // Check that input/output tokens can be merged
        let input_col = cols.iter().find(|c| c.id == "input_tokens").unwrap();
        let output_col = cols.iter().find(|c| c.id == "output_tokens").unwrap();

        assert!(input_col.can_merge);
        assert!(output_col.can_merge);
        assert_eq!(input_col.merge_with, Some("output_tokens"));
        assert_eq!(output_col.merge_with, Some("input_tokens"));
    }
}
