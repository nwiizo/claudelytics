use crate::models::{DailyReport, MonthlyReport, SessionReport};
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
// use std::io::{self, Write};

pub fn display_daily_report_enhanced(report: &DailyReport) {
    // Header with timestamp
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{}", "📊 Claude Code Usage Analytics".bright_blue().bold());
    println!(
        "{} Generated at {}",
        "🕐".bright_yellow(),
        timestamp.to_string().dimmed()
    );
    println!();

    // Quick summary card
    display_summary_card(&report.totals, report.daily.len());
    println!();

    // Recent activity (last 7 days)
    if !report.daily.is_empty() {
        display_recent_activity(&report.daily);
        println!();
    }

    // Detailed table for more than 3 days
    if report.daily.len() > 3 {
        println!("{}", "📋 Detailed Daily Breakdown".bright_green().bold());
        display_daily_table_compact(report);
    } else if !report.daily.is_empty() {
        println!("{}", "📋 Daily Usage Details".bright_green().bold());
        display_daily_cards(&report.daily);
    }
}

pub fn display_session_report_enhanced(report: &SessionReport) {
    // Header with timestamp
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!(
        "{}",
        "📊 Claude Code Session Analytics".bright_blue().bold()
    );
    println!(
        "{} Generated at {}",
        "🕐".bright_yellow(),
        timestamp.to_string().dimmed()
    );
    println!();

    // Quick summary
    display_session_summary(&report.totals, report.sessions.len());
    println!();

    // Top sessions
    display_top_sessions(&report.sessions);
    println!();

    // Detailed table for many sessions
    if report.sessions.len() > 5 {
        println!("{}", "📋 All Sessions".bright_green().bold());
        display_session_table_compact(report);
    }
}

fn display_summary_card(totals: &crate::models::TokenUsageTotals, days_count: usize) {
    let cost_str = format!("${:.4}", totals.total_cost);
    let tokens_str = format_number(totals.total_tokens);
    let input_str = format_number(totals.input_tokens);
    let output_str = format_number(totals.output_tokens);
    let cache_str = format_number(totals.cache_creation_tokens + totals.cache_read_tokens);

    // Calculate content width (without colors)
    let line1_content = format!(
        "  💰 Total Cost: {}  │  📅 Days: {}  │  🎯 Total Tokens: {}  ",
        cost_str, days_count, tokens_str
    );
    let line2_content = format!(
        "  📥 Input: {}  │  📤 Output: {}  │  🔄 Cache: {}  ",
        input_str, output_str, cache_str
    );

    let content_width =
        std::cmp::max(line1_content.chars().count(), line2_content.chars().count()) + 2;
    let border = "─".repeat(content_width);

    println!("{}", format!("┌{}┐", border).bright_black());

    let line1_padding = content_width.saturating_sub(line1_content.chars().count());
    println!(
        "{}  💰 Total Cost: {}  │  📅 Days: {}  │  🎯 Total Tokens: {}  {}{}│",
        "│".bright_black(),
        cost_str.bright_green().bold(),
        days_count.to_string().bright_blue().bold(),
        tokens_str.bright_magenta().bold(),
        " ".repeat(line1_padding),
        "│".bright_black()
    );

    let line2_padding = content_width.saturating_sub(line2_content.chars().count());
    println!(
        "{}  📥 Input: {}  │  📤 Output: {}  │  🔄 Cache: {}  {}{}│",
        "│".bright_black(),
        input_str.green(),
        output_str.blue(),
        cache_str.yellow(),
        " ".repeat(line2_padding),
        "│".bright_black()
    );

    println!("{}", format!("└{}┘", border).bright_black());
}

fn display_session_summary(totals: &crate::models::TokenUsageTotals, session_count: usize) {
    let cost_str = format!("${:.4}", totals.total_cost);
    let tokens_str = format_number(totals.total_tokens);

    // Calculate content width (without colors)
    let content = format!(
        "  💰 Total Cost: {}  │  📊 Sessions: {}  │  🎯 Total Tokens: {}  ",
        cost_str, session_count, tokens_str
    );
    let content_width = content.chars().count() + 2;
    let border = "─".repeat(content_width);

    println!("{}", format!("┌{}┐", border).bright_black());

    let padding = content_width.saturating_sub(content.chars().count());
    println!(
        "{}  💰 Total Cost: {}  │  📊 Sessions: {}  │  🎯 Total Tokens: {}  {}{}│",
        "│".bright_black(),
        cost_str.bright_green().bold(),
        session_count.to_string().bright_blue().bold(),
        tokens_str.bright_magenta().bold(),
        " ".repeat(padding),
        "│".bright_black()
    );

    println!("{}", format!("└{}┘", border).bright_black());
}

fn display_recent_activity(daily: &[crate::models::DailyUsage]) {
    println!("{}", "🚀 Recent Activity".bright_cyan().bold());
    println!();

    let recent_days = daily.iter().take(7);
    for (i, day) in recent_days.enumerate() {
        let indicator = if i == 0 { "►" } else { " " };
        let date_text = if i == 0 {
            day.date.bright_green().bold()
        } else {
            day.date.bright_black()
        };

        let tokens_str = format_number(day.total_tokens);
        let cost_str = format!("${:.4}", day.total_cost);

        println!(
            "{} {} {} {} {} {}",
            indicator.bright_green(),
            date_text,
            "│".bright_black(),
            format!("{:>12} tokens", tokens_str).bright_white(),
            "│".bright_black(),
            format!("{:>8}", cost_str).bright_green()
        );
    }
}

fn display_top_sessions(sessions: &[crate::models::SessionUsage]) {
    println!("{}", "🏆 Top Sessions by Cost".bright_cyan().bold());
    println!();

    for (i, session) in sessions.iter().take(5).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "🔸",
        };

        let session_path = format!("{}/{}", session.project_path, session.session_id);
        let truncated_path = truncate_path(&session_path, 45);
        let tokens_str = format_number(session.total_tokens);
        let cost_str = format!("${:.4}", session.total_cost);

        println!(
            "{} {:<47} {} {} {} {}",
            medal,
            truncated_path.bright_white(),
            "│".bright_black(),
            format!("{:>12} tokens", tokens_str).bright_cyan(),
            "│".bright_black(),
            format!("{:>8}", cost_str).bright_green()
        );
    }
}

fn display_daily_cards(daily: &[crate::models::DailyUsage]) {
    for (i, day) in daily.iter().enumerate() {
        let is_today = i == 0;
        let date_text = if is_today {
            day.date.bright_green().bold()
        } else {
            day.date.bright_black().bold()
        };
        let title_emoji = if is_today { "📅" } else { "📋" };

        let cost_str = format!("${:.4}", day.total_cost);
        let tokens_str = format_number(day.total_tokens);
        let input_str = format_number(day.input_tokens);
        let output_str = format_number(day.output_tokens);
        let cache_str = format_number(day.cache_creation_tokens + day.cache_read_tokens);
        let ratio = if day.input_tokens > 0 {
            day.output_tokens as f64 / day.input_tokens as f64
        } else {
            0.0
        };

        println!("{} {}", title_emoji, date_text);
        println!(
            "  💰 Cost: {} │ 🎯 Tokens: {} │ 📊 Ratio: {:.1}:1",
            cost_str.bright_green(),
            tokens_str.bright_cyan(),
            ratio
        );
        println!(
            "  📥 In: {} │ 📤 Out: {} │ 🔄 Cache: {}",
            input_str.green(),
            output_str.blue(),
            cache_str.yellow()
        );

        if i < daily.len() - 1 {
            println!();
        }
    }
}

fn display_daily_table_compact(report: &DailyReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Date").fg(Color::Cyan),
            Cell::new("Cost").fg(Color::Cyan),
            Cell::new("Tokens").fg(Color::Cyan),
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Output").fg(Color::Cyan),
            Cell::new("Ratio").fg(Color::Cyan),
        ]);

    for (i, daily) in report.daily.iter().enumerate() {
        let date_color = if i == 0 { Color::Green } else { Color::White };
        let ratio = if daily.input_tokens > 0 {
            daily.output_tokens as f64 / daily.input_tokens as f64
        } else {
            0.0
        };

        table.add_row(vec![
            Cell::new(&daily.date).fg(date_color),
            Cell::new(format!("${:.4}", daily.total_cost)).fg(Color::Green),
            Cell::new(format_number(daily.total_tokens)).fg(Color::Magenta),
            Cell::new(format_number(daily.input_tokens)).fg(Color::Blue),
            Cell::new(format_number(daily.output_tokens)).fg(Color::Cyan),
            Cell::new(format!("{:.1}:1", ratio)).fg(Color::Yellow),
        ]);
    }

    println!("{}", table);
}

fn display_session_table_compact(report: &SessionReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Session").fg(Color::Cyan),
            Cell::new("Cost").fg(Color::Cyan),
            Cell::new("Tokens").fg(Color::Cyan),
            Cell::new("Activity").fg(Color::Cyan),
        ]);

    for session in &report.sessions {
        let session_path = format!("{}/{}", session.project_path, session.session_id);
        let truncated = truncate_path(&session_path, 30);

        table.add_row(vec![
            Cell::new(truncated),
            Cell::new(format!("${:.4}", session.total_cost)).fg(Color::Green),
            Cell::new(format_number(session.total_tokens)).fg(Color::Magenta),
            Cell::new(&session.last_activity).fg(Color::Yellow),
        ]);
    }

    println!("{}", table);
}

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

pub fn display_monthly_report_enhanced(report: &MonthlyReport) {
    // Header with timestamp
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!(
        "{}",
        "📊 Claude Code Monthly Analytics".bright_blue().bold()
    );
    println!(
        "{} Generated at {}",
        "🕐".bright_yellow(),
        timestamp.to_string().dimmed()
    );
    println!();

    // Quick summary card
    display_summary_card(&report.totals, report.monthly.len());
    println!();

    // Monthly breakdown
    if !report.monthly.is_empty() {
        println!("{}", "📋 Monthly Usage Breakdown".bright_green().bold());
        display_monthly_table(report);
    }
}

pub fn display_monthly_report_table(report: &MonthlyReport) {
    println!("{}", "Monthly Usage Report".bold());
    display_monthly_table(report);
}

pub fn display_monthly_report_json(report: &MonthlyReport) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing report to JSON: {}", e),
    }
}

fn display_monthly_table(report: &MonthlyReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Month").fg(Color::Cyan),
            Cell::new("Year").fg(Color::Cyan),
            Cell::new("Days Active").fg(Color::Cyan),
            Cell::new("Input Tokens").fg(Color::Green),
            Cell::new("Output Tokens").fg(Color::Yellow),
            Cell::new("Cache Tokens").fg(Color::Magenta),
            Cell::new("Total Tokens").fg(Color::White),
            Cell::new("Total Cost").fg(Color::Red),
            Cell::new("Avg Daily Cost").fg(Color::DarkRed),
        ]);

    for entry in &report.monthly {
        let cache_tokens = entry.cache_creation_tokens + entry.cache_read_tokens;
        table.add_row(vec![
            Cell::new(&entry.month),
            Cell::new(entry.year),
            Cell::new(entry.days_active),
            Cell::new(format_number(entry.input_tokens)).fg(Color::Green),
            Cell::new(format_number(entry.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(cache_tokens)).fg(Color::Magenta),
            Cell::new(format_number(entry.total_tokens)),
            Cell::new(format_currency(entry.total_cost)).fg(Color::Red),
            Cell::new(format_currency(entry.avg_daily_cost)).fg(Color::DarkRed),
        ]);
    }

    // Add totals row
    if !report.monthly.is_empty() {
        let total_days: u32 = report.monthly.iter().map(|m| m.days_active).sum();
        let cache_tokens = report.totals.cache_creation_tokens + report.totals.cache_read_tokens;
        table.add_row(vec![
            Cell::new("Total").fg(Color::Yellow),
            Cell::new("").fg(Color::Yellow),
            Cell::new(total_days).fg(Color::Yellow),
            Cell::new(format_number(report.totals.input_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(cache_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.total_tokens)).fg(Color::Yellow),
            Cell::new(format_currency(report.totals.total_cost)).fg(Color::Yellow),
            Cell::new("").fg(Color::Yellow),
        ]);
    }

    println!("{table}");
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

pub fn print_info(message: &str) {
    println!("{} {}", "Info:".blue(), message);
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
        assert_eq!(
            truncate_path("this/is/a/very/long/path", 15),
            "...ry/long/path"
        );
        assert_eq!(truncate_path("", 10), "");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 10), "short");
        assert_eq!(truncate_text("very-long-session-id", 10), "very-lo...");
    }
}
