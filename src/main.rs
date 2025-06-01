mod display;
mod models;
mod parser;
mod reports;

use anyhow::Result;
use clap::{Parser, Subcommand};
use display::{
    display_daily_report_json, display_daily_report_table, display_session_report_json,
    display_session_report_table, print_error, print_warning,
};
use parser::UsageParser;
use reports::{generate_daily_report, generate_session_report};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "claudelytics")]
#[command(about = "Claude Code usage analytics tool")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, value_name = "DATE", help = "Filter from date (YYYYMMDD)")]
    since: Option<String>,

    #[arg(short, long, value_name = "DATE", help = "Filter until date (YYYYMMDD)")]
    until: Option<String>,

    #[arg(short, long, value_name = "PATH", help = "Path to Claude directory")]
    path: Option<PathBuf>,

    #[arg(short, long, help = "Output in JSON format")]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Show daily usage report (default)")]
    Daily,
    #[command(about = "Show session-based usage report")]
    Session,
}

fn main() {
    if let Err(e) = run() {
        print_error(&format!("{}", e));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Get Claude directory path
    let claude_dir = match cli.path {
        Some(path) => path,
        None => {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
            PathBuf::from(home).join(".claude")
        }
    };

    // Validate Claude directory exists
    if !claude_dir.exists() {
        anyhow::bail!(
            "Claude directory not found at {}\nHint: Make sure Claude Code is installed and has been used at least once.",
            claude_dir.display()
        );
    }

    // Create parser
    let parser = UsageParser::new(claude_dir, cli.since, cli.until)?;

    // Parse all usage data
    let (daily_map, session_map) = parser.parse_all()?;

    // Check if we have any data
    if daily_map.is_empty() && session_map.is_empty() {
        print_warning("No usage data found for the specified criteria");
        return Ok(());
    }

    // Generate and display report based on command
    let command = cli.command.unwrap_or(Commands::Daily);
    match command {
        Commands::Daily => {
            let report = generate_daily_report(daily_map);
            if report.daily.is_empty() {
                print_warning("No daily usage data found for the specified date range");
            } else if cli.json {
                display_daily_report_json(&report);
            } else {
                display_daily_report_table(&report);
            }
        }
        Commands::Session => {
            let report = generate_session_report(session_map);
            if report.sessions.is_empty() {
                print_warning("No session usage data found for the specified date range");
            } else if cli.json {
                display_session_report_json(&report);
            } else {
                display_session_report_table(&report);
            }
        }
    }

    Ok(())
}