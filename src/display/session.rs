use super::helpers::{format_currency, format_number, truncate_path, truncate_text};
use crate::models::SessionReport;
use crate::responsive_tables::{ResponsiveTable, display_responsive_summary};
use crate::terminal::Terminal;
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

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

/// Display session report with responsive table layout
pub fn display_session_report_responsive(report: &SessionReport) {
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

    // Enhanced session summary using responsive display
    let context = format!("{} sessions", report.sessions.len());
    display_responsive_summary(&report.totals, &context);
    println!();

    // Top sessions with better formatting
    if report.sessions.len() > 5 {
        display_enhanced_top_sessions(&report.sessions);
        println!();
    }

    // Responsive table for session details
    if !report.sessions.is_empty() {
        println!("{}", Terminal::separator('─').bright_black());
        println!(
            "{}",
            "📋 Session Details (Responsive)".bright_green().bold()
        );
        println!("{}", Terminal::separator('─').bright_black());

        let responsive_table = ResponsiveTable::new();
        responsive_table.display_session_report(report);
    }

    // Footer
    println!();
    println!("{}", Terminal::separator('═').bright_black());
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

        let cache_percentage = if (session.cache_read_tokens
            + session.cache_creation_tokens
            + session.input_tokens)
            > 0
        {
            session.cache_read_tokens as f64
                / (session.cache_read_tokens + session.cache_creation_tokens + session.input_tokens)
                    as f64
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
