use crate::burn_rate::BurnRateCalculator;
use crate::models::{DailyReport, MonthlyReport, SessionReport};
use crate::terminal::{DisplayMode, Terminal};
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
// use std::io::{self, Write};

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

pub fn display_session_report_enhanced(report: &SessionReport) {
    // Header with timestamp and separator
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{}", Terminal::separator('═').bright_black());
    println!(
        "{}  {}",
        "📊 Claude Code Session Analytics".bright_blue().bold(),
        format!("Generated {}", timestamp).dimmed()
    );
    println!("{}", Terminal::separator('═').bright_black());
    println!();

    // Enhanced session summary
    display_enhanced_session_summary(&report.totals, report.sessions.len());
    println!();

    // Top sessions with better formatting
    display_enhanced_top_sessions(&report.sessions);
    println!();

    // Detailed table for many sessions with visual separation
    if report.sessions.len() > 5 {
        println!("{}", Terminal::separator('─').bright_black());
        println!("{}", "📋 Complete Session List".bright_green().bold());
        println!("{}", Terminal::separator('─').bright_black());
        display_session_table_compact(report);
    }

    // Footer
    println!();
    println!("{}", Terminal::separator('═').bright_black());
}

fn display_enhanced_summary_card(totals: &crate::models::TokenUsageTotals, days_count: usize) {
    // Calculate additional insights
    let avg_daily_cost = if days_count > 0 {
        totals.total_cost / days_count as f64
    } else {
        0.0
    };
    let avg_daily_tokens = if days_count > 0 {
        totals.total_tokens / days_count as u64
    } else {
        0
    };

    let cost_str = format_currency(totals.total_cost);
    let tokens_str = format_number(totals.total_tokens);
    let input_str = format_number(totals.input_tokens);
    let output_str = format_number(totals.output_tokens);
    let cache_str = format_number(totals.cache_creation_tokens + totals.cache_read_tokens);

    // Calculate efficiency metrics following ccusage methodology
    let tokens_per_dollar = if totals.total_cost > 0.0 {
        totals.total_tokens as f64 / totals.total_cost
    } else {
        0.0
    };
    let output_input_ratio = if totals.input_tokens > 0 {
        totals.output_tokens as f64 / totals.input_tokens as f64
    } else {
        0.0
    };
    let cache_efficiency = if (totals.input_tokens + totals.cache_creation_tokens) > 0 {
        totals.cache_read_tokens as f64
            / (totals.input_tokens + totals.cache_creation_tokens) as f64
            * 100.0
    } else {
        0.0
    };

    println!("{}", "💰 COST & USAGE SUMMARY".bright_yellow().bold());
    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!(
        "│ 💰 Total Cost: {:>10}  │  📅 Period: {:>2} days  │  🎯 Total Tokens: {:>15} │",
        cost_str.bright_green().bold(),
        days_count.to_string().bright_blue().bold(),
        tokens_str.bright_magenta().bold()
    );
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!(
        "│ 📥 Input: {:>12}  │  📤 Output: {:>12}  │  🔄 Cache: {:>15} │",
        input_str.green(),
        output_str.blue(),
        cache_str.yellow()
    );
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!(
        "│ ⚡ Efficiency: {:>8} tok/$  │  📊 O/I Ratio: {:>5}  │  🎯 Cache Hit: {:>7} │",
        format!("{:.0}", tokens_per_dollar).bright_cyan().bold(),
        format!("{:.1}:1", output_input_ratio)
            .bright_yellow()
            .bold(),
        format!("{:.1}%", cache_efficiency).bright_magenta().bold()
    );
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!(
        "│ 📈 Daily Avg: {:>10} ({:>15} tokens)  │  💡 Est. Monthly: {:>10} │",
        format_currency(avg_daily_cost).bright_green(),
        format_number(avg_daily_tokens).bright_magenta(),
        format_currency(avg_daily_cost * 30.0).bright_red()
    );
    println!("└─────────────────────────────────────────────────────────────────────────────┘");
}

fn display_summary_card(totals: &crate::models::TokenUsageTotals, days_count: usize) {
    let cost_str = format_currency(totals.total_cost);
    let tokens_str = format_number(totals.total_tokens);
    let input_str = format_number(totals.input_tokens);
    let output_str = format_number(totals.output_tokens);
    let cache_str = format_number(totals.cache_creation_tokens + totals.cache_read_tokens);

    // Calculate efficiency metrics following ccusage methodology
    let tokens_per_dollar = if totals.total_cost > 0.0 {
        totals.total_tokens as f64 / totals.total_cost
    } else {
        0.0
    };
    let output_input_ratio = if totals.input_tokens > 0 {
        totals.output_tokens as f64 / totals.input_tokens as f64
    } else {
        0.0
    };
    let cache_efficiency = if (totals.input_tokens + totals.cache_creation_tokens) > 0 {
        totals.cache_read_tokens as f64
            / (totals.input_tokens + totals.cache_creation_tokens) as f64
            * 100.0
    } else {
        0.0
    };

    // Fixed width for consistent alignment
    let box_width = 95;

    println!(
        "{}",
        format!("┌{}┐", "─".repeat(box_width - 2)).bright_black()
    );

    // Line 1
    let line1_plain = format!(
        "  💰 Total Cost: {}  │  📅 Days: {}  │  🎯 Total Tokens: {}  ",
        cost_str, days_count, tokens_str
    );
    print!("{}", "│".bright_black());
    print!("  💰 Total Cost: {}", cost_str.bright_green().bold());
    print!(
        "  │  📅 Days: {}",
        days_count.to_string().bright_blue().bold()
    );
    print!(
        "  │  🎯 Total Tokens: {}  ",
        tokens_str.bright_magenta().bold()
    );
    let padding1 = if box_width > line1_plain.len() {
        box_width - line1_plain.len()
    } else {
        1
    };
    println!("{}{}", " ".repeat(padding1), "│".bright_black());

    // Line 2
    let line2_plain = format!(
        "  📥 Input: {}  │  📤 Output: {}  │  🔄 Cache: {}  ",
        input_str, output_str, cache_str
    );
    print!("{}", "│".bright_black());
    print!("  📥 Input: {}", input_str.green());
    print!("  │  📤 Output: {}", output_str.blue());
    print!("  │  🔄 Cache: {}  ", cache_str.yellow());
    let padding2 = if box_width > line2_plain.len() {
        box_width - line2_plain.len()
    } else {
        1
    };
    println!("{}{}", " ".repeat(padding2), "│".bright_black());

    // Line 3
    let line3_plain = format!(
        "  ⚡ Efficiency: {:.0} tok/$  │  📊 Ratio: {:.1}:1  │  🎯 Cache Hit: {:.1}%  ",
        tokens_per_dollar, output_input_ratio, cache_efficiency
    );
    print!("{}", "│".bright_black());
    print!(
        "  ⚡ Efficiency: {}",
        format!("{:.0} tok/$", tokens_per_dollar)
            .bright_cyan()
            .bold()
    );
    print!(
        "  │  📊 Ratio: {}",
        format!("{:.1}:1", output_input_ratio)
            .bright_yellow()
            .bold()
    );
    print!(
        "  │  🎯 Cache Hit: {}  ",
        format!("{:.1}%", cache_efficiency).bright_magenta().bold()
    );
    let padding3 = if box_width > line3_plain.len() {
        box_width - line3_plain.len()
    } else {
        1
    };
    println!("{}{}", " ".repeat(padding3), "│".bright_black());

    println!(
        "{}",
        format!("└{}┘", "─".repeat(box_width - 2)).bright_black()
    );
}

fn display_enhanced_session_summary(
    totals: &crate::models::TokenUsageTotals,
    session_count: usize,
) {
    let cost_str = format_currency(totals.total_cost);
    let tokens_str = format_number(totals.total_tokens);

    // Calculate insights
    let avg_cost_per_session = if session_count > 0 {
        totals.total_cost / session_count as f64
    } else {
        0.0
    };
    let avg_tokens_per_session = if session_count > 0 {
        totals.total_tokens / session_count as u64
    } else {
        0
    };
    let tokens_per_dollar = if totals.total_cost > 0.0 {
        totals.total_tokens as f64 / totals.total_cost
    } else {
        0.0
    };

    println!("{}", "🔍 SESSION ANALYSIS SUMMARY".bright_yellow().bold());

    // Fixed width for consistent alignment
    let box_width = 95;

    println!("┌{}┐", "─".repeat(box_width - 2));

    // Line 1
    let line1_plain = format!(
        " 💰 Total Cost: {}  │  📊 Sessions: {}  │  🎯 Total Tokens: {} ",
        cost_str, session_count, tokens_str
    );
    print!("│");
    print!(" 💰 Total Cost: {}", cost_str.bright_green().bold());
    print!(
        "  │  📊 Sessions: {}",
        session_count.to_string().bright_blue().bold()
    );
    print!(
        "  │  🎯 Total Tokens: {} ",
        tokens_str.bright_magenta().bold()
    );
    let padding1 = if box_width > line1_plain.len() + 2 {
        box_width - line1_plain.len() - 2
    } else {
        1
    };
    println!("{}│", " ".repeat(padding1));

    println!("├{}┤", "─".repeat(box_width - 2));

    // Line 2
    let line2_plain = format!(
        " 📈 Avg/Session: {} ({} tokens)  │  ⚡ Efficiency: {:.0} tok/$ ",
        format_currency(avg_cost_per_session),
        format_number(avg_tokens_per_session),
        tokens_per_dollar
    );
    print!("│");
    print!(
        " 📈 Avg/Session: {} ({} tokens)",
        format_currency(avg_cost_per_session).bright_green(),
        format_number(avg_tokens_per_session).bright_cyan()
    );
    print!(
        "  │  ⚡ Efficiency: {} tok/$ ",
        format!("{:.0}", tokens_per_dollar).bright_yellow()
    );
    let padding2 = if box_width > line2_plain.len() + 2 {
        box_width - line2_plain.len() - 2
    } else {
        1
    };
    println!("{}│", " ".repeat(padding2));

    println!("└{}┘", "─".repeat(box_width - 2));
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
            format_number(metrics_7d.projected_monthly_cost as u64 * 1000000 / 150)
                .bright_magenta()
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

fn display_enhanced_top_sessions(sessions: &[crate::models::SessionUsage]) {
    println!("{}", "🏆 TOP SESSIONS BY COST".bright_cyan().bold());
    println!();

    for (i, session) in sessions.iter().take(5).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "🔸",
        };

        let session_path = format!("{}/{}", session.project_path, session.session_id);
        let truncated_path = truncate_path(&session_path, 32);
        let tokens_str = format_number(session.total_tokens);
        let cost_str = format_currency(session.total_cost);

        // Calculate efficiency and additional metrics
        let tokens_per_dollar = if session.total_cost > 0.0 {
            session.total_tokens as f64 / session.total_cost
        } else {
            0.0
        };

        let cache_percentage = if (session.input_tokens + session.cache_creation_tokens) > 0 {
            session.cache_read_tokens as f64
                / (session.input_tokens + session.cache_creation_tokens) as f64
                * 100.0
        } else {
            0.0
        };

        println!(
            "{} {:<34} {} {} {} {} {} {} {} {}",
            medal,
            truncated_path.bright_white(),
            "│".bright_black(),
            format!("{:>12} tokens", tokens_str).bright_cyan(),
            "│".bright_black(),
            format!("{:>10}", cost_str).bright_green(),
            "│".bright_black(),
            format!("{:>7.0} tok/$", tokens_per_dollar).bright_yellow(),
            "│".bright_black(),
            format!("{:>5.1}% cache", cache_percentage).bright_magenta()
        );
    }

    if sessions.len() > 5 {
        println!();
        println!(
            "{}",
            format!("   ... and {} more sessions", sessions.len() - 5).dimmed()
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
        let cache_efficiency = if (day.input_tokens + day.cache_creation_tokens) > 0 {
            day.cache_read_tokens as f64 / (day.input_tokens + day.cache_creation_tokens) as f64
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
        let cache_efficiency = if (daily.input_tokens + daily.cache_creation_tokens) > 0 {
            daily.cache_read_tokens as f64
                / (daily.input_tokens + daily.cache_creation_tokens) as f64
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
        let cache_efficiency = if (daily.input_tokens + daily.cache_creation_tokens) > 0 {
            daily.cache_read_tokens as f64
                / (daily.input_tokens + daily.cache_creation_tokens) as f64
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
            Cell::new(format!("{:>10}", format_currency(session.total_cost))).fg(Color::Green),
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

// Create a simple structure for family aggregation
#[derive(Debug, Clone, Default)]
struct FamilyUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    total_cost: f64,
}

impl FamilyUsage {
    fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }

    fn add_usage(&mut self, usage: &crate::models::TokenUsage) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation_tokens += usage.cache_creation_tokens;
        self.cache_read_tokens += usage.cache_read_tokens;
        self.total_cost += usage.total_cost;
    }
}

pub fn display_model_breakdown_report(
    daily_map: &std::collections::HashMap<chrono::NaiveDate, crate::models::TokenUsage>,
    _session_map: &std::collections::HashMap<
        String,
        (crate::models::TokenUsage, chrono::DateTime<chrono::Utc>),
    >,
) {
    use std::collections::HashMap;

    // Check for display format preference
    let display_format = std::env::var("CLAUDELYTICS_DISPLAY_FORMAT")
        .unwrap_or_else(|_| "default".to_string())
        .to_lowercase();

    // Check if user wants table format (based on FIX_SUMMARY.md documentation)
    if std::env::var("CLAUDELYTICS_TABLE_FORMAT").is_ok() || display_format == "table" {
        // Calculate total cost and tokens for the table display
        let mut total_cost = 0.0;
        let mut total_tokens = 0u64;
        let mut family_usage: HashMap<String, FamilyUsage> = HashMap::new();

        if let Ok(model_breakdown) = parse_usage_by_model() {
            for (family, usage_data) in model_breakdown {
                let fu = family_usage.entry(family.clone()).or_default();
                fu.add_usage(&usage_data);
                total_cost += usage_data.total_cost;
                total_tokens += usage_data.input_tokens
                    + usage_data.output_tokens
                    + usage_data.cache_creation_tokens
                    + usage_data.cache_read_tokens;
            }
        }

        return display_model_breakdown_as_table(&family_usage, total_cost, total_tokens);
    }

    let _registry = crate::models_registry::ModelsRegistry::new();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    // Header
    println!("{}", Terminal::separator('═').bright_black());
    println!(
        "{}  {}",
        "📊 Claude Usage by Model Family".bright_blue().bold(),
        format!("Generated {}", timestamp).dimmed()
    );
    println!("{}", Terminal::separator('═').bright_black());
    println!();

    // Group usage by model family
    let mut family_usage: HashMap<String, FamilyUsage> = HashMap::new();

    // Parse raw JSONL files to extract model information
    if let Ok(model_breakdown) = parse_usage_by_model() {
        for (family, usage_data) in model_breakdown {
            let family_usage_entry = FamilyUsage {
                input_tokens: usage_data.input_tokens,
                output_tokens: usage_data.output_tokens,
                cache_creation_tokens: usage_data.cache_creation_tokens,
                cache_read_tokens: usage_data.cache_read_tokens,
                total_cost: usage_data.total_cost,
            };

            family_usage.insert(family, family_usage_entry);
        }
    } else {
        // Fallback to aggregated data if parsing fails
        print_warning(
            "Unable to parse model data from JSONL files, showing aggregated data as 'Unknown'",
        );
        let mut unknown_usage = FamilyUsage::default();

        // Process daily data
        for usage in daily_map.values() {
            unknown_usage.add_usage(usage);
        }

        if unknown_usage.total_tokens() > 0 {
            family_usage.insert("Unknown".to_string(), unknown_usage);
        }
    }

    if family_usage.is_empty() {
        print_warning("No model usage data found");
        return;
    }

    // Sort families by cost (highest first)
    let mut sorted_families: Vec<_> = family_usage.iter().collect();
    sorted_families.sort_by(|a, b| {
        b.1.total_cost
            .partial_cmp(&a.1.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate totals
    let total_cost: f64 = family_usage.values().map(|u| u.total_cost).sum();
    let total_tokens: u64 = family_usage.values().map(|u| u.total_tokens()).sum();

    // Display overall summary using ASCII table
    println!("{}", "💰 OVERALL USAGE SUMMARY".bright_yellow().bold());
    println!("{}", "=".repeat(80));

    println!(
        "Total Cost: {}  |  Total Tokens: {}  |  Model Families: {}",
        format_currency(total_cost).bright_green().bold(),
        format_number(total_tokens).bright_magenta().bold(),
        family_usage.len().to_string().bright_blue().bold()
    );

    println!("{}", "=".repeat(80));
    println!();

    // Display breakdown by family
    println!("{}", "📋 USAGE BY MODEL FAMILY".bright_green().bold());
    println!();

    for (family, usage) in sorted_families {
        let cost_str = format_currency(usage.total_cost);
        let tokens_str = format_number(usage.total_tokens());
        let input_str = format_number(usage.input_tokens);
        let output_str = format_number(usage.output_tokens);
        let cache_str = format_number(usage.cache_creation_tokens + usage.cache_read_tokens);

        // Calculate metrics
        let cost_percentage = if total_cost > 0.0 {
            (usage.total_cost / total_cost) * 100.0
        } else {
            0.0
        };

        let token_percentage = if total_tokens > 0 {
            (usage.total_tokens() as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };

        let efficiency = if usage.total_cost > 0.0 {
            usage.total_tokens() as f64 / usage.total_cost
        } else {
            0.0
        };

        let output_input_ratio = if usage.input_tokens > 0 {
            usage.output_tokens as f64 / usage.input_tokens as f64
        } else {
            0.0
        };

        // Family icon and display name
        let (family_icon, family_display) = match family.to_lowercase().as_str() {
            "opus" => ("🔥", "Opus"),
            "sonnet" => ("🎵", "Sonnet"),
            "haiku" => ("🌸", "Haiku"),
            _ => ("❓", family.as_str()),
        };

        // Use comfy_table for proper alignment
        let mut model_table = Table::new();
        model_table.load_preset(comfy_table::presets::ASCII_FULL);

        println!(
            "{} {} Model Family",
            family_icon,
            family_display.bright_cyan().bold()
        );
        println!("{}", "-".repeat(70));

        // Display metrics in a clean, aligned format
        println!(
            "  Cost:         {:>12} ({:>5.1}%)",
            cost_str.bright_green(),
            cost_percentage
        );
        println!(
            "  Tokens:       {:>12} ({:>5.1}%)",
            tokens_str.bright_magenta(),
            token_percentage
        );
        println!("  Input:        {:>12}", input_str.green());
        println!("  Output:       {:>12}", output_str.blue());
        println!("  Cache:        {:>12}", cache_str.yellow());
        println!(
            "  Efficiency:   {:>12} tok/$",
            format!("{:.0}", efficiency).bright_cyan()
        );
        println!(
            "  O/I Ratio:    {:>12}",
            format!("{:.1}:1", output_input_ratio).bright_yellow()
        );
        println!();
    }

    // Footer
    println!("{}", Terminal::separator('═').bright_black());
}

/// Display model breakdown as a proper aligned table
fn display_model_breakdown_as_table(
    family_usage: &std::collections::HashMap<String, FamilyUsage>,
    total_cost: f64,
    _total_tokens: u64,
) {
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::ASCII_FULL);

    table.set_header(vec![
        Cell::new("Model").fg(Color::Cyan),
        Cell::new("Cost").fg(Color::Green),
        Cell::new("Cost %").fg(Color::Green),
        Cell::new("Tokens").fg(Color::Magenta),
        Cell::new("Input").fg(Color::Blue),
        Cell::new("Output").fg(Color::Yellow),
        Cell::new("Efficiency").fg(Color::Cyan),
    ]);

    // Sort families by cost
    let mut sorted_families: Vec<_> = family_usage.iter().collect();
    sorted_families.sort_by(|a, b| {
        b.1.total_cost
            .partial_cmp(&a.1.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (family, usage) in sorted_families {
        let cost_pct = if total_cost > 0.0 {
            (usage.total_cost / total_cost) * 100.0
        } else {
            0.0
        };
        let efficiency = if usage.total_cost > 0.0 {
            usage.total_tokens() as f64 / usage.total_cost
        } else {
            0.0
        };

        table.add_row(vec![
            Cell::new(family),
            Cell::new(format_currency(usage.total_cost)),
            Cell::new(format!("{:.1}%", cost_pct)),
            Cell::new(format_number(usage.total_tokens())),
            Cell::new(format_number(usage.input_tokens)),
            Cell::new(format_number(usage.output_tokens)),
            Cell::new(format!("{:.0} tok/$", efficiency)),
        ]);
    }

    println!("{}", table);
}

/// Parse raw JSONL files to extract usage by model family
fn parse_usage_by_model()
-> Result<std::collections::HashMap<String, crate::models::TokenUsage>, Box<dyn std::error::Error>>
{
    use crate::models::{TokenUsage, UsageRecord};
    use crate::models_registry::ModelsRegistry;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;
    use walkdir::WalkDir;

    let registry = ModelsRegistry::new();
    let mut family_usage: HashMap<String, TokenUsage> = HashMap::new();

    // Get Claude directory path (use default ~/.claude)
    let claude_dir = std::env::var("HOME")
        .map(|home| PathBuf::from(home).join(".claude"))
        .map_err(|_| "Unable to determine home directory")?;

    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return Err("Claude projects directory not found".into());
    }

    // Find all JSONL files
    let jsonl_files: Vec<PathBuf> = WalkDir::new(projects_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "jsonl")
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    // Parse each file
    for file_path in jsonl_files {
        if let Ok(file) = File::open(&file_path) {
            let reader = BufReader::new(file);

            for line in reader.lines().map_while(Result::ok) {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(record) = serde_json::from_str::<UsageRecord>(&line) {
                    if let Some(model_name) = record.get_model_name() {
                        if record
                            .message
                            .as_ref()
                            .and_then(|m| m.usage.as_ref())
                            .is_some()
                        {
                            let family = registry
                                .get_model_family(model_name)
                                .unwrap_or_else(|| "Unknown".to_string());

                            let family = capitalize_family_name(&family);

                            let usage = TokenUsage::from(&record);
                            family_usage.entry(family).or_default().add(&usage);
                        }
                    }
                }
            }
        }
    }

    Ok(family_usage)
}

/// Capitalize family name for display
fn capitalize_family_name(family: &str) -> String {
    match family.to_lowercase().as_str() {
        "opus" => "Opus".to_string(),
        "sonnet" => "Sonnet".to_string(),
        "haiku" => "Haiku".to_string(),
        _ => family.to_string(),
    }
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
        assert_eq!(format_currency(0.0), "$0.0000");
        assert_eq!(format_currency(99.9999), "$99.9999");
        assert_eq!(format_currency(100.0), "$100.00");
        assert_eq!(format_currency(123.45), "$123.45");
        assert_eq!(format_currency(0.1), "$0.1000");
        assert_eq!(format_currency(999.99), "$999.99");
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
