use super::helpers::{format_currency, format_number};
use super::summary::display_summary_card;
use crate::models::WeeklyReport;
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

pub fn display_weekly_report_enhanced(report: &WeeklyReport) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{}", "📊 Claude Code Weekly Analytics".bright_blue().bold());
    println!(
        "{} Generated at {}",
        "🕐".bright_yellow(),
        timestamp.to_string().dimmed()
    );
    println!();

    display_summary_card(&report.totals, report.weekly.len());
    println!();

    if !report.weekly.is_empty() {
        println!("{}", "📋 Weekly Usage Breakdown".bright_green().bold());
        display_weekly_table(report);
    }
}

pub fn display_weekly_report_table(report: &WeeklyReport) {
    println!("{}", "Weekly Usage Report".bold());
    display_weekly_table(report);
}

fn display_weekly_table(report: &WeeklyReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Week").fg(Color::Cyan),
            Cell::new("Days Active").fg(Color::Cyan),
            Cell::new("Input Tokens").fg(Color::Green),
            Cell::new("Output Tokens").fg(Color::Yellow),
            Cell::new("Total Tokens").fg(Color::White),
            Cell::new("Total Cost").fg(Color::Red),
            Cell::new("Avg Daily Cost").fg(Color::DarkRed),
        ]);

    for entry in &report.weekly {
        table.add_row(vec![
            Cell::new(format!("{} ~ {}", &entry.week_start, &entry.week_end)),
            Cell::new(entry.days_active),
            Cell::new(format_number(entry.input_tokens)).fg(Color::Green),
            Cell::new(format_number(entry.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(entry.total_tokens)),
            Cell::new(format_currency(entry.total_cost)).fg(Color::Red),
            Cell::new(format_currency(entry.avg_daily_cost)).fg(Color::DarkRed),
        ]);
    }

    if !report.weekly.is_empty() {
        let total_days: u32 = report.weekly.iter().map(|w| w.days_active).sum();
        table.add_row(vec![
            Cell::new("Total").fg(Color::Yellow),
            Cell::new(total_days).fg(Color::Yellow),
            Cell::new(format_number(report.totals.input_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.total_tokens)).fg(Color::Yellow),
            Cell::new(format_currency(report.totals.total_cost)).fg(Color::Yellow),
            Cell::new(if total_days > 0 {
                format_currency(report.totals.total_cost / total_days as f64)
            } else {
                format_currency(0.0)
            })
            .fg(Color::Yellow),
        ]);
    }

    println!("{table}");
}
