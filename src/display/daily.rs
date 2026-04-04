use super::helpers::{format_currency, format_number};
use super::summary::display_enhanced_summary_card;
use crate::burn_rate::BurnRateCalculator;
use crate::models::DailyReport;
use crate::responsive_tables::ResponsiveTable;
use crate::terminal::{DisplayMode, Terminal};
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

/// Compact default output: a simple table like ccusage
pub fn display_daily_report_compact(report: &DailyReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Date").fg(Color::Cyan),
            Cell::new("Input").fg(Color::Green),
            Cell::new("Output").fg(Color::Yellow),
            Cell::new("Cache Write").fg(Color::Magenta),
            Cell::new("Cache Read").fg(Color::Magenta),
            Cell::new("Total Tokens").fg(Color::White),
            Cell::new("Cost (USD)").fg(Color::Red),
        ]);

    for entry in &report.daily {
        table.add_row(vec![
            Cell::new(&entry.date),
            Cell::new(format_number(entry.input_tokens)).fg(Color::Green),
            Cell::new(format_number(entry.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(entry.cache_creation_tokens)).fg(Color::Magenta),
            Cell::new(format_number(entry.cache_read_tokens)).fg(Color::Magenta),
            Cell::new(format_number(entry.total_tokens)),
            Cell::new(format_currency(entry.total_cost)).fg(Color::Red),
        ]);
    }

    // Totals row
    if report.daily.len() > 1 {
        let cache_tokens = report.totals.cache_creation_tokens + report.totals.cache_read_tokens;
        table.add_row(vec![
            Cell::new("Total").fg(Color::Yellow),
            Cell::new(format_number(report.totals.input_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.output_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.cache_creation_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.cache_read_tokens)).fg(Color::Yellow),
            Cell::new(format_number(report.totals.total_tokens)).fg(Color::Yellow),
            Cell::new(format_currency(report.totals.total_cost)).fg(Color::Yellow),
        ]);
        let _ = cache_tokens; // used above in individual cells
    }

    println!("{table}");
}

pub fn display_daily_report_enhanced(report: &DailyReport, _force_compact: bool) {
    // Header with timestamp and separator
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{}", Terminal::separator('═').bright_black());
    println!(
        "{}  {}",
        "📊 Claude Code Usage Analytics".bright_blue().bold(),
        format!("Generated {}", timestamp).dimmed()
    );
    println!("{}", Terminal::separator('═').bright_black());
    println!();

    // Quick summary card with insights
    display_enhanced_summary_card(&report.totals, report.daily.len());
    println!();

    // Display burn rate metrics
    if !report.daily.is_empty() {
        display_burn_rate_metrics(&report.daily);
        println!();
    }

    // Recent activity with better visual separation
    if !report.daily.is_empty() {
        display_enhanced_recent_activity(&report.daily);
        println!();
    }

    // Detailed breakdown with visual separation
    if report.daily.len() > 3 {
        println!("{}", Terminal::separator('─').bright_black());
        println!(
            "{}",
            "📋 Complete Daily Breakdown (Last 30 Days)"
                .bright_green()
                .bold()
        );
        println!("{}", Terminal::separator('─').bright_black());
        // Create a modified report with only the last 30 days
        let limited_report = DailyReport {
            daily: report.daily.iter().take(30).cloned().collect(),
            totals: report.totals.clone(),
        };
        display_daily_table_complete(&limited_report);
    } else if !report.daily.is_empty() {
        println!("{}", Terminal::separator('─').bright_black());
        println!("{}", "📋 Daily Usage Details".bright_green().bold());
        println!("{}", Terminal::separator('─').bright_black());
        display_daily_cards(&report.daily);
    }

    // Footer
    println!();
    println!("{}", Terminal::separator('═').bright_black());
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

/// Display daily report with responsive table layout
pub fn display_daily_report_responsive(report: &DailyReport) {
    use crate::responsive_tables::display_responsive_summary;

    // Header with timestamp and separator
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{}", Terminal::separator('═').bright_black());
    println!(
        "{}  {}",
        "📊 Claude Code Usage Analytics".bright_blue().bold(),
        format!("Generated {}", timestamp).dimmed()
    );
    println!("{}", Terminal::separator('═').bright_black());
    println!();

    // Quick summary using responsive display
    let context = format!("{} days", report.daily.len());
    display_responsive_summary(&report.totals, &context);
    println!();

    // Display burn rate metrics if available
    if !report.daily.is_empty() {
        display_burn_rate_metrics(&report.daily);
        println!();
    }

    // Recent activity
    if !report.daily.is_empty() {
        display_enhanced_recent_activity(&report.daily);
        println!();
    }

    // Responsive table for daily breakdown
    if !report.daily.is_empty() {
        println!("{}", Terminal::separator('─').bright_black());
        println!(
            "{}",
            "📋 Daily Breakdown (Responsive)".bright_green().bold()
        );
        println!("{}", Terminal::separator('─').bright_black());

        let responsive_table = ResponsiveTable::new();
        responsive_table.display_daily_report(report);
    }

    // Footer
    println!();
    println!("{}", Terminal::separator('═').bright_black());
}

fn display_burn_rate_metrics(daily: &[crate::models::DailyUsage]) {
    use crate::models::TokenUsage;
    use chrono::NaiveDate;
    use std::collections::HashMap;

    // Convert daily usage to DailyUsageMap for burn rate calculator
    let mut daily_map: HashMap<NaiveDate, TokenUsage> = HashMap::new();
    for day in daily {
        let usage = TokenUsage {
            input_tokens: day.input_tokens,
            output_tokens: day.output_tokens,
            cache_creation_tokens: day.cache_creation_tokens,
            cache_read_tokens: day.cache_read_tokens,
            total_cost: day.total_cost,
            fast_mode_cost: 0.0,
        };
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") {
            daily_map.insert(date, usage);
        }
    }

    let calculator = BurnRateCalculator::new(daily_map);

    // Calculate burn rate for different time periods
    let burn_rate_24h = calculator.calculate_burn_rate(24);
    let burn_rate_7d = calculator.calculate_burn_rate(24 * 7);

    println!("{}", "🔥 BURN RATE ANALYSIS".bright_red().bold());
    println!("{}", Terminal::separator('─').bright_black());

    if let Some(metrics_24h) = burn_rate_24h {
        let trend_arrow = if metrics_24h.trend_percentage > 0.0 {
            "↑".bright_red()
        } else if metrics_24h.trend_percentage < 0.0 {
            "↓".bright_green()
        } else {
            "→".bright_yellow()
        };

        println!(
            "24h Rate: {} tokens/hr (${:.4}/hr) {} {:.1}%",
            format_number(metrics_24h.tokens_per_hour as u64).bright_cyan(),
            metrics_24h.cost_per_hour,
            trend_arrow,
            metrics_24h.trend_percentage.abs()
        );

        println!(
            "Projected Today: {} tokens (${:.2})",
            format_number(metrics_24h.projected_daily_tokens).bright_magenta(),
            metrics_24h.projected_daily_cost
        );
    }

    if let Some(metrics_7d) = burn_rate_7d {
        println!();
        println!(
            "7-Day Average: {} tokens/hr (${:.4}/hr)",
            format_number(metrics_7d.tokens_per_hour as u64).bright_cyan(),
            metrics_7d.cost_per_hour
        );

        println!(
            "Monthly Projection: {} ({} tokens)",
            format!("${:.2}", metrics_7d.projected_monthly_cost)
                .bright_red()
                .bold(),
            format_number(metrics_7d.projected_monthly_tokens).bright_magenta()
        );
    }

    println!("{}", Terminal::separator('─').bright_black());
}

fn display_enhanced_recent_activity(daily: &[crate::models::DailyUsage]) {
    println!("{}", "📈 RECENT ACTIVITY TREND".bright_cyan().bold());
    println!();

    let recent_days = daily.iter().take(7);
    let mut total_cost_week = 0.0;
    let mut total_tokens_week = 0;

    for (i, day) in recent_days.enumerate() {
        total_cost_week += day.total_cost;
        total_tokens_week += day.total_tokens;

        let indicator = if i == 0 {
            "►".bright_green().bold()
        } else {
            " ".normal()
        };

        let date_text = if i == 0 {
            format!("{} (Today)", day.date).bright_green().bold()
        } else {
            day.date.bright_white()
        };

        let tokens_str = format_number(day.total_tokens);
        let cost_str = format_currency(day.total_cost);

        // Calculate day's efficiency
        let efficiency = if day.total_cost > 0.0 {
            day.total_tokens as f64 / day.total_cost
        } else {
            0.0
        };

        let efficiency_str = format!("{:>8.0} tok/$", efficiency);
        println!(
            "{} {:<18} {} {} {} {} {} {}",
            indicator,
            date_text,
            "│".bright_black(),
            format!("{:>15} tokens", tokens_str).bright_cyan(),
            "│".bright_black(),
            cost_str.bright_green(),
            "│".bright_black(),
            efficiency_str.bright_yellow()
        );
    }

    if daily.len() >= 2 {
        println!();
        println!("{}", Terminal::separator('─').bright_black());
        let avg_cost = total_cost_week / (daily.len().min(7) as f64);
        let avg_tokens = total_tokens_week / (daily.len().min(7) as u64);
        println!(
            "{}  Week Avg: {}  │  {} tokens  │  Trending: {}",
            "📊".bright_blue(),
            format_currency(avg_cost).bright_green(),
            format_number(avg_tokens).bright_cyan(),
            if daily[0].total_cost > avg_cost {
                "📈 Up"
            } else {
                "📉 Down"
            }
            .bright_yellow()
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

        let cost_str = format_currency(day.total_cost);
        let tokens_str = format_number(day.total_tokens);
        let input_str = format_number(day.input_tokens);
        let output_str = format_number(day.output_tokens);
        let cache_str = format_number(day.cache_creation_tokens + day.cache_read_tokens);

        // Calculate efficiency metrics
        let ratio = if day.input_tokens > 0 {
            day.output_tokens as f64 / day.input_tokens as f64
        } else {
            0.0
        };
        let tokens_per_dollar = if day.total_cost > 0.0 {
            day.total_tokens as f64 / day.total_cost
        } else {
            0.0
        };
        let cache_efficiency =
            if (day.cache_read_tokens + day.cache_creation_tokens + day.input_tokens) > 0 {
                day.cache_read_tokens as f64
                    / (day.cache_read_tokens + day.cache_creation_tokens + day.input_tokens) as f64
                    * 100.0
            } else {
                0.0
            };

        println!("{} {}", title_emoji, date_text);
        println!(
            "  💰 Cost: {} │ 🎯 Tokens: {} │ ⚡ Efficiency: {} tok/$",
            format!("{:>10}", cost_str).bright_green(),
            tokens_str.bright_cyan(),
            format!("{:.0}", tokens_per_dollar).bright_yellow()
        );
        println!(
            "  📥 In: {} │ 📤 Out: {} │ 🔄 Cache: {} ({:.1}%)",
            input_str.green(),
            output_str.blue(),
            cache_str.yellow(),
            cache_efficiency
        );
        println!(
            "  📊 O/I Ratio: {:.1}:1 │ 💡 Cache Hit Rate: {:.1}%",
            ratio, cache_efficiency
        );

        if i < daily.len() - 1 {
            println!();
        }
    }
}

#[allow(dead_code)]
fn display_daily_table_compact(report: &DailyReport, force_compact: bool) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    // Adjust columns based on terminal width or force compact
    let display_mode = if force_compact {
        DisplayMode::Compact
    } else {
        DisplayMode::detect()
    };
    let mut headers = vec![
        Cell::new("Date").fg(Color::Cyan),
        Cell::new("Cost").fg(Color::Cyan),
        Cell::new("Tokens").fg(Color::Cyan),
    ];

    if display_mode != DisplayMode::Compact {
        headers.push(Cell::new("Input").fg(Color::Cyan));
        headers.push(Cell::new("Output").fg(Color::Cyan));
        headers.push(Cell::new("O/I Ratio").fg(Color::Cyan));
    }

    if display_mode.should_show_efficiency() {
        headers.push(Cell::new("Efficiency").fg(Color::Cyan));
        headers.push(Cell::new("Cache Hit").fg(Color::Cyan));
    }

    table.set_header(headers);

    // Reverse the order to show newest dates at the bottom
    let reversed_daily: Vec<_> = report.daily.iter().rev().collect();
    let last_index = reversed_daily.len().saturating_sub(1);

    for (i, daily) in reversed_daily.iter().enumerate() {
        let date_color = if i == last_index {
            Color::Green
        } else {
            Color::White
        };
        let ratio = if daily.input_tokens > 0 {
            daily.output_tokens as f64 / daily.input_tokens as f64
        } else {
            0.0
        };
        let tokens_per_dollar = if daily.total_cost > 0.0 {
            daily.total_tokens as f64 / daily.total_cost
        } else {
            0.0
        };
        let cache_efficiency =
            if (daily.cache_read_tokens + daily.cache_creation_tokens + daily.input_tokens) > 0 {
                daily.cache_read_tokens as f64
                    / (daily.cache_read_tokens + daily.cache_creation_tokens + daily.input_tokens)
                        as f64
                    * 100.0
            } else {
                0.0
            };

        let mut row = vec![
            Cell::new(&daily.date).fg(date_color),
            Cell::new(format!("{:>10}", format_currency(daily.total_cost))).fg(Color::Green),
            Cell::new(format_number(daily.total_tokens)).fg(Color::Magenta),
        ];

        if display_mode != DisplayMode::Compact {
            row.push(Cell::new(format_number(daily.input_tokens)).fg(Color::Blue));
            row.push(Cell::new(format_number(daily.output_tokens)).fg(Color::Cyan));
            row.push(Cell::new(format!("{:.1}:1", ratio)).fg(Color::Yellow));
        }

        if display_mode.should_show_efficiency() {
            row.push(Cell::new(format!("{:.0} tok/$", tokens_per_dollar)).fg(Color::Green));
            row.push(Cell::new(format!("{:.1}%", cache_efficiency)).fg(Color::Magenta));
        }

        table.add_row(row);
    }

    println!("{}", table);
}

fn display_daily_table_complete(report: &DailyReport) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    // Always show all columns for the Complete Daily Breakdown
    let headers = vec![
        Cell::new("Date").fg(Color::Cyan),
        Cell::new("Cost").fg(Color::Cyan),
        Cell::new("Tokens").fg(Color::Cyan),
        Cell::new("Input").fg(Color::Cyan),
        Cell::new("Output").fg(Color::Cyan),
        Cell::new("O/I Ratio").fg(Color::Cyan),
        Cell::new("Efficiency").fg(Color::Cyan),
        Cell::new("Cache Hit").fg(Color::Cyan),
    ];

    table.set_header(headers);

    // Reverse the order to show newest dates at the bottom
    let reversed_daily: Vec<_> = report.daily.iter().rev().collect();
    let last_index = reversed_daily.len().saturating_sub(1);

    for (i, daily) in reversed_daily.iter().enumerate() {
        let date_color = if i == last_index {
            Color::Green
        } else {
            Color::White
        };
        let ratio = if daily.input_tokens > 0 {
            daily.output_tokens as f64 / daily.input_tokens as f64
        } else {
            0.0
        };
        let tokens_per_dollar = if daily.total_cost > 0.0 {
            daily.total_tokens as f64 / daily.total_cost
        } else {
            0.0
        };
        let cache_efficiency =
            if (daily.cache_read_tokens + daily.cache_creation_tokens + daily.input_tokens) > 0 {
                daily.cache_read_tokens as f64
                    / (daily.cache_read_tokens + daily.cache_creation_tokens + daily.input_tokens)
                        as f64
                    * 100.0
            } else {
                0.0
            };

        let row = vec![
            Cell::new(&daily.date).fg(date_color),
            Cell::new(format!("{:>10}", format_currency(daily.total_cost))).fg(Color::Green),
            Cell::new(format_number(daily.total_tokens)).fg(Color::Magenta),
            Cell::new(format_number(daily.input_tokens)).fg(Color::Blue),
            Cell::new(format_number(daily.output_tokens)).fg(Color::Cyan),
            Cell::new(format!("{:.1}:1", ratio)).fg(Color::Yellow),
            Cell::new(format!("{:.0} tok/$", tokens_per_dollar)).fg(Color::Green),
            Cell::new(format!("{:.1}%", cache_efficiency)).fg(Color::Magenta),
        ];

        table.add_row(row);
    }

    println!("{}", table);
}
