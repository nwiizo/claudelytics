use super::helpers::{format_currency, format_number};
use colored::*;

pub(crate) fn display_enhanced_summary_card(
    totals: &crate::models::TokenUsageTotals,
    days_count: usize,
) {
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
    let cache_efficiency =
        if (totals.cache_read_tokens + totals.cache_creation_tokens + totals.input_tokens) > 0 {
            totals.cache_read_tokens as f64
                / (totals.cache_read_tokens + totals.cache_creation_tokens + totals.input_tokens)
                    as f64
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

pub(crate) fn display_summary_card(totals: &crate::models::TokenUsageTotals, days_count: usize) {
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
    let cache_efficiency =
        if (totals.cache_read_tokens + totals.cache_creation_tokens + totals.input_tokens) > 0 {
            totals.cache_read_tokens as f64
                / (totals.cache_read_tokens + totals.cache_creation_tokens + totals.input_tokens)
                    as f64
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
