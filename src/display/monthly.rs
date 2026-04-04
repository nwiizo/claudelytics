use super::helpers::{format_currency, format_number};
use super::summary::display_summary_card;
use crate::models::MonthlyReport;
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

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
