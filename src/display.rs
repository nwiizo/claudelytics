use crate::models::{DailyReport, SessionReport};
use colored::*;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

pub fn display_daily_report_table(report: &DailyReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Date").fg(Color::Cyan),
            Cell::new("Input Tokens").fg(Color::Cyan),
            Cell::new("Output Tokens").fg(Color::Cyan),
            Cell::new("Cache Creation").fg(Color::Cyan),
            Cell::new("Cache Read").fg(Color::Cyan),
            Cell::new("Total Tokens").fg(Color::Cyan),
            Cell::new("Cost (USD)").fg(Color::Cyan),
        ]);

    for daily in &report.daily {
        table.add_row(vec![
            Cell::new(&daily.date),
            Cell::new(format_number(daily.input_tokens)),
            Cell::new(format_number(daily.output_tokens)),
            Cell::new(format_number(daily.cache_creation_tokens)),
            Cell::new(format_number(daily.cache_read_tokens)),
            Cell::new(format_number(daily.total_tokens)),
            Cell::new(format_currency(daily.total_cost)),
        ]);
    }

    // Add totals row
    if !report.daily.is_empty() {
        table.add_row(vec![
            Cell::new("Total").fg(Color::Yellow),
            Cell::new(format_number(report.totals.input_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.cache_creation_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.cache_read_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.total_tokens)).fg(Color::Yellow),
            Cell::new(format_currency(report.totals.total_cost)).fg(Color::Yellow),
        ]);
    }

    println!("{}", table);
}

pub fn display_session_report_table(report: &SessionReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Project Path").fg(Color::Cyan),
            Cell::new("Session ID").fg(Color::Cyan),
            Cell::new("Input Tokens").fg(Color::Cyan),
            Cell::new("Output Tokens").fg(Color::Cyan),
            Cell::new("Cache Creation").fg(Color::Cyan),
            Cell::new("Cache Read").fg(Color::Cyan),
            Cell::new("Total Tokens").fg(Color::Cyan),
            Cell::new("Cost (USD)").fg(Color::Cyan),
            Cell::new("Last Activity").fg(Color::Cyan),
        ]);

    for session in &report.sessions {
        table.add_row(vec![
            Cell::new(truncate_path(&session.project_path, 25)),
            Cell::new(truncate_text(&session.session_id, 20)),
            Cell::new(format_number(session.input_tokens)),
            Cell::new(format_number(session.output_tokens)),
            Cell::new(format_number(session.cache_creation_tokens)),
            Cell::new(format_number(session.cache_read_tokens)),
            Cell::new(format_number(session.total_tokens)),
            Cell::new(format_currency(session.total_cost)),
            Cell::new(&session.last_activity),
        ]);
    }

    // Add totals row
    if !report.sessions.is_empty() {
        table.add_row(vec![
            Cell::new("Total").fg(Color::Yellow),
            Cell::new("").fg(Color::Yellow),
            Cell::new(format_number(report.totals.input_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.cache_creation_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.cache_read_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.total_tokens)).fg(Color::Yellow),
            Cell::new(format_currency(report.totals.total_cost)).fg(Color::Yellow),
            Cell::new("").fg(Color::Yellow),
        ]);
    }

    println!("{}", table);
}

pub fn display_daily_report_json(report: &DailyReport) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing report to JSON: {}", e),
    }
}

pub fn display_session_report_json(report: &SessionReport) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing report to JSON: {}", e),
    }
}

fn format_number(num: u64) -> String {
    if num == 0 {
        "0".to_string()
    } else {
        // Manual comma formatting since Rust doesn't support {:,} format
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

fn format_currency(amount: f64) -> String {
    format!("${:.2}", amount)
}

fn truncate_path(path: &str, max_length: usize) -> String {
    if path.len() <= max_length {
        path.to_string()
    } else if path.is_empty() {
        "-".to_string()
    } else {
        format!("...{}", &path[path.len().saturating_sub(max_length - 3)..])
    }
}

fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

pub fn print_warning(message: &str) {
    eprintln!("{} {}", "Warning:".yellow(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "Error:".red(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency(0.0), "$0.00");
        assert_eq!(format_currency(123.45), "$123.45");
        assert_eq!(format_currency(0.1), "$0.10");
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short", 10), "short");
        assert_eq!(truncate_path("this/is/a/very/long/path", 15), "...very/long/path");
        assert_eq!(truncate_path("", 10), "-");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 10), "short");
        assert_eq!(truncate_text("very-long-session-id", 10), "very-lo...");
    }
}