use super::helpers::{format_currency, format_number};
use crate::responsive_tables::ResponsiveTable;
use crate::terminal::Terminal;
use colored::*;

/// Display billing blocks with responsive table layout
pub fn display_billing_blocks_responsive(
    blocks: &[(chrono::NaiveDate, &crate::billing_blocks::BillingBlock)],
) {
    // Header
    println!("{}", Terminal::separator('═').bright_black());
    println!(
        "{}",
        "💰 5-Hour Billing Blocks (Responsive)".bright_blue().bold()
    );
    println!("{}", Terminal::separator('═').bright_black());
    println!();

    if blocks.is_empty() {
        println!("No billing block data available.");
        return;
    }

    // Calculate totals
    let mut total_cost = 0.0;
    let mut total_tokens = 0u64;
    for (_, block) in blocks {
        total_cost += block.usage.total_cost;
        total_tokens += block.usage.total_tokens();
    }

    // Summary
    println!(
        "Total across {} blocks: {} ({} tokens)",
        blocks.len(),
        format_currency(total_cost).bright_green().bold(),
        format_number(total_tokens).bright_magenta().bold()
    );
    println!();

    // Responsive table
    let responsive_table = ResponsiveTable::new();
    responsive_table.display_billing_blocks(blocks);

    // Footer
    println!();
    println!("{}", Terminal::separator('═').bright_black());
}
