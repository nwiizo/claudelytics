//! Claudelytics - Claude Code Usage Analytics Tool
//!
//! A fast CLI tool for analyzing Claude Code usage patterns, token consumption, and costs.
//! Parses JSONL files from ~/.claude/projects/ and generates comprehensive reports.

// Module declarations
mod config;
mod config_v2;
mod display;
mod domain;
mod error;
mod export;
mod interactive;
mod models;
mod parser;
mod performance;
mod pricing;
mod pricing_strategies;
mod processing;
mod reports;
mod state;
mod tui;
mod watcher;

// Core dependencies
use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use config::Config;
use display::{
    display_daily_report_enhanced, display_daily_report_json, display_daily_report_table,
    display_session_report_enhanced, display_session_report_json, display_session_report_table,
    print_error, print_info, print_warning,
};
use export::{export_daily_to_csv, export_sessions_to_csv, export_summary_to_csv};
use interactive::InteractiveSelector;
use parser::UsageParser;
use reports::{generate_daily_report, generate_session_report};
use state::{TuiMode, TuiSessionState};
use std::path::PathBuf;
use tui::TuiApp;
use watcher::UsageWatcher;

#[derive(Parser)]
#[command(name = "claudelytics")]
#[command(
    about = "Claude Code usage analytics tool - Analyze token usage, costs, and session patterns"
)]
#[command(version = "0.2.0")]
#[command(
    long_about = "Claudelytics analyzes Claude Code usage patterns and costs by parsing JSONL files from ~/.claude/projects/.

EXAMPLES:
  claudelytics                    # Show today's usage in enhanced format
  claudelytics --today --json     # Today's usage as JSON
  claudelytics daily --since 20240101  # Daily report from Jan 1, 2024
  claudelytics session --classic  # Classic table format for sessions
  claudelytics cost --today       # Quick cost check for today
  claudelytics interactive        # Browse sessions interactively
  claudelytics export --daily -o report  # Export daily data to CSV
  claudelytics config --show      # View current configuration
  claudelytics tui                # Launch terminal interface
  claudelytics watch              # Monitor usage in real-time

GLOBAL FLAGS:
  Global flags like --json, --today, --since work with any command:
  claudelytics --json daily       # Daily report as JSON
  claudelytics --today session    # Today's sessions only"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(
        short,
        long,
        value_name = "DATE",
        help = "Filter from date (YYYYMMDD)",
        long_help = "Filter usage data from this date onwards. Format: YYYYMMDD\nExample: --since 20240101 (January 1, 2024)\nCombine with --until for date ranges: --since 20240101 --until 20240131"
    )]
    since: Option<String>,

    #[arg(
        short,
        long,
        value_name = "DATE",
        help = "Filter until date (YYYYMMDD)",
        long_help = "Filter usage data up to this date. Format: YYYYMMDD\nExample: --until 20241231 (December 31, 2024)\nUse alone for 'up to date' or combine with --since for ranges"
    )]
    until: Option<String>,

    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Path to Claude directory",
        long_help = "Custom path to Claude directory (default: ~/.claude)\nUseful if Claude Code data is in a non-standard location\nExample: --path /custom/claude or --path ~/Dropbox/.claude"
    )]
    path: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Output in JSON format",
        long_help = "Output data in JSON format instead of formatted tables\nUseful for scripting, APIs, or piping to other tools\nExample: claudelytics --json daily | jq '.totals.total_cost'"
    )]
    json: bool,

    #[arg(
        short,
        long,
        help = "Show today's usage only",
        long_help = "Filter to show only today's usage data\nEquivalent to setting --since and --until to today's date\nCombines with other commands: claudelytics --today session"
    )]
    today: bool,

    #[arg(
        long,
        help = "Use classic table format",
        long_help = "Use classic table format instead of enhanced cards\nPrimary interface: Traditional ASCII tables\nDefault: Enhanced format with visual cards and summaries"
    )]
    classic: bool,

    #[arg(
        long,
        help = "Launch terminal user interface",
        long_help = "Launch interactive terminal user interface (TUI)\nFeatures: Navigation, search, charts, multiple tabs\nKeyboard shortcuts: j/k navigation, q to quit, ? for help"
    )]
    tui: bool,
    // #[arg(long, help = "Launch analytics studio TUI", long_help = "Launch comprehensive analytics studio with AI insights\nFeatures: Pattern analysis, predictive modeling, ML insights, risk management\nKeyboard shortcuts: F10-F12 for analytics tabs, advanced data exploration")]
    // analytics_tui: bool, // Temporarily disabled - work in progress
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Show daily usage report (default)")]
    #[command(
        long_about = "Show daily usage aggregated by date\n\nDisplays token usage, costs, and activity patterns grouped by day.\nDefault enhanced format shows visual cards; use --classic for tables.\n\nEXAMPLES:\n  claudelytics daily                    # Enhanced daily report\n  claudelytics daily --classic          # Classic table format\n  claudelytics --json daily             # JSON output (global flag)\n  claudelytics --since 20240101 daily   # From specific date (global flag)\n  claudelytics --today daily            # Today only (global flag)"
    )]
    Daily {
        #[arg(
            long,
            help = "Use classic table format",
            long_help = "Override enhanced format with classic ASCII tables\nInherits global flags: --json, --today, --since, --until"
        )]
        classic: bool,
    },
    #[command(about = "Show session-based usage report")]
    #[command(
        long_about = "Show usage aggregated by Claude Code sessions\n\nDisplays individual session data with project paths, activity times,\nand per-session token usage and costs.\n\nEXAMPLES:\n  claudelytics session                  # Enhanced session report\n  claudelytics session --classic        # Classic table format\n  claudelytics --json session           # JSON output (global flag)\n  claudelytics --today session          # Today's sessions only (global flag)"
    )]
    Session {
        #[arg(
            long,
            help = "Use classic table format",
            long_help = "Override enhanced format with classic ASCII tables\nInherits global flags: --json, --today, --since, --until"
        )]
        classic: bool,
    },
    #[command(about = "Interactive session selector (peco-style)")]
    #[command(
        long_about = "Launch interactive session browser with fuzzy search\n\nProvides a searchable, filterable interface to browse and select\nsessions. Type to filter, use arrow keys to navigate, Enter to select.\n\nFEATURES:\n  - Fuzzy search across project paths and session IDs\n  - Real-time filtering as you type\n  - Shows session metadata (tokens, cost, last activity)\n  - Keyboard navigation (arrows, Enter, Esc)\n\nEXAMPLE:\n  claudelytics interactive              # Launch interactive browser"
    )]
    Interactive,
    #[command(about = "Watch for real-time usage updates")]
    #[command(
        long_about = "Monitor Claude Code usage in real-time\n\nWatches the Claude directory for new usage data and displays\nupdates as they occur. Useful for monitoring active sessions.\n\nFEATURES:\n  - Real-time file monitoring\n  - Automatic data refresh\n  - Debounced updates (avoids spam)\n  - Graceful interruption with Ctrl+C\n\nEXAMPLE:\n  claudelytics watch                    # Start monitoring"
    )]
    Watch,
    #[command(about = "Launch terminal user interface")]
    #[command(
        long_about = "Launch interactive terminal user interface\n\nFull-featured TUI with multiple tabs, navigation, and visual charts.\nProvides comprehensive analysis in a terminal-based interface.\n\nFEATURES:\n  - Multiple tabs: Overview, Daily, Sessions, Charts, Help\n  - Keyboard navigation (j/k, arrows, Enter, Tab)\n  - Visual elements: gauges, charts, formatted tables\n  - Search and filtering capabilities\n  - Real-time data display\n\nKEYBOARD SHORTCUTS:\n  q/Esc: Quit  Tab: Next tab  j/k: Navigate  Enter: Select\n\nEXAMPLE:\n  claudelytics tui                      # Launch TUI"
    )]
    Tui,
    // #[command(about = "Launch analytics studio TUI")]
    // #[command(long_about = "Launch comprehensive analytics studio with AI insights\n\nData science-grade analytics interface with 17 specialized tabs,\npattern analysis, predictive modeling, and machine learning insights.\n\nFEATURES:\n  - 17 specialized analytics tabs\n  - Usage pattern detection and clustering\n  - Productivity analytics with deep work analysis\n  - Predictive cost forecasting and trend analysis\n  - Risk management with budget tracking\n  - Workflow integration (Git, projects, milestones)\n  - AI-powered insights and recommendations\n  - Interactive data exploration with correlation analysis\n  - Advanced search with smart suggestions\n  - Custom dashboards and personalization\n\nKEYBOARD SHORTCUTS:\n  F10-F12: Analytics tabs  Ctrl+F: Advanced search\n  Ctrl+D: Custom dashboard  All advanced TUI shortcuts apply\n\nEXAMPLE:\n  claudelytics analytics-tui            # Launch Analytics Studio")]
    // AnalyticsTui, // Temporarily disabled - work in progress
    #[command(about = "Export data to CSV")]
    #[command(
        long_about = "Export usage data to CSV files for external analysis\n\nCreates CSV files containing daily reports, session data, or summaries.\nDefault behavior exports all types if no specific flags are provided.\n\nFILE NAMING:\n  Daily report: {base}.daily.csv\n  Sessions: {base}.sessions.csv\n  Summary: {base}.summary.csv\n\nEXAMPLES:\n  claudelytics export                   # Export all to default location\n  claudelytics export --daily -o report # Export daily data only\n  claudelytics export --sessions --summary # Export sessions + summary\n  claudelytics --since 20240101 export # Export data from specific date"
    )]
    Export {
        #[arg(
            long,
            help = "Export daily report",
            long_help = "Export daily aggregated usage data to CSV\nIncludes: date, total tokens, costs, session counts"
        )]
        daily: bool,
        #[arg(
            long,
            help = "Export session report",
            long_help = "Export individual session data to CSV\nIncludes: project path, session ID, timestamps, tokens, costs"
        )]
        sessions: bool,
        #[arg(
            long,
            help = "Export summary",
            long_help = "Export overall summary statistics to CSV\nIncludes: totals, averages, date ranges, top sessions"
        )]
        summary: bool,
        #[arg(
            short,
            long,
            help = "Output file path",
            long_help = "Base path for output files (without extension)\nDefault: ./claudelytics_export (creates .daily.csv, .sessions.csv, etc.)\nExample: -o ~/reports/usage creates ~/reports/usage.daily.csv"
        )]
        output: Option<PathBuf>,
    },
    #[command(about = "Manage configuration")]
    #[command(
        long_about = "Manage Claudelytics configuration settings\n\nConfiguration is stored in YAML format and persists between runs.\nUse --show to view current settings or modify specific options.\n\nCONFIG LOCATION:\n  ~/.config/claudelytics/config.yaml (or platform equivalent)\n\nAVAILABLE SETTINGS:\n  - Claude directory path\n  - Default output format (enhanced/classic/json)\n  - Default command\n  - Watch interval for real-time monitoring\n  - Export directory\n  - Date format preferences\n\nEXAMPLES:\n  claudelytics config --show            # View current configuration\n  claudelytics config --set-path ~/claude # Set custom Claude directory\n  claudelytics config --reset           # Reset to defaults"
    )]
    Config {
        #[arg(
            long,
            help = "Show current configuration",
            long_help = "Display all current configuration settings\nShows: Claude path, default formats, intervals, directories"
        )]
        show: bool,
        #[arg(
            long,
            help = "Reset to default configuration",
            long_help = "Reset all settings to default values\nWarning: This will overwrite your current configuration file"
        )]
        reset: bool,
        #[arg(
            long,
            help = "Set Claude directory path",
            long_help = "Set custom path to Claude directory\nUseful for non-standard installations or shared configurations\nExample: --set-path ~/Dropbox/.claude"
        )]
        set_path: Option<PathBuf>,
    },
    #[command(about = "Show cost summary")]
    #[command(
        long_about = "Display cost analysis and summaries\n\nQuick access to cost information without full reports.\nUseful for monitoring expenses and budget tracking.\n\nCOST CALCULATION:\n  Based on Claude API pricing for input/output tokens\n  Includes cache creation and cache read tokens\n  Costs shown in USD\n\nEXAMPLES:\n  claudelytics cost                     # Total cost summary\n  claudelytics cost --today             # Today's cost only\n  claudelytics cost --date 20240315     # Specific date cost\n\nSHELL INTEGRATION:\n  alias ctoday='claudelytics cost --today'\n  alias ctotal='claudelytics cost'"
    )]
    Cost {
        #[arg(
            long,
            help = "Show only today's cost",
            long_help = "Display cost information for today only\nShows: date, total cost, token count\nUseful for daily budget monitoring"
        )]
        today: bool,
        #[arg(
            long,
            help = "Show cost for specific date (YYYYMMDD)",
            long_help = "Display cost for a specific date\nFormat: YYYYMMDD (e.g., 20240315 for March 15, 2024)\nShows: date, cost, tokens for that day only"
        )]
        date: Option<String>,
    },
    #[command(about = "Debug resume state")]
    #[command(long_about = "Debug command to show TUI session state information")]
    DebugState,
    #[command(about = "Test resume functionality")]
    #[command(long_about = "Test command to verify resume functionality without starting TUI")]
    TestResume,
}

/// Application entry point
fn main() {
    if let Err(e) = run() {
        print_error(&format!("{}", e));
        std::process::exit(1);
    }
}

/// Main application logic
fn run() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let mut config = Config::load().unwrap_or_default();

    // Get Claude directory path
    let claude_dir = if let Some(path) = cli.path {
        path
    } else {
        config.get_claude_path().unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".claude")
        })
    };

    // Handle --today flag by setting since and until to today
    let (since_date, until_date) = if cli.today {
        let today = Local::now().date_naive().format("%Y%m%d").to_string();
        (Some(today.clone()), Some(today))
    } else {
        (cli.since, cli.until)
    };

    // Handle configuration commands first
    if let Some(Commands::Config {
        show,
        reset,
        set_path,
    }) = &cli.command
    {
        return handle_config_command(&mut config, *show, *reset, set_path.clone());
    }

    // Validate Claude directory exists
    if !claude_dir.exists() {
        anyhow::bail!(
            "Claude directory not found at {}\nHint: Make sure Claude Code is installed and has been used at least once.",
            claude_dir.display()
        );
    }

    // Create parser
    let parser = UsageParser::new(claude_dir.clone(), since_date, until_date)?;

    // Handle watch command
    if let Some(Commands::Watch) = &cli.command {
        let mut watcher = UsageWatcher::new(parser)?;
        return watcher.watch(&claude_dir);
    }

    // Parse all usage data
    let (daily_map, session_map) = parser.parse_all()?;

    // Check if we have any data
    if daily_map.is_empty() && session_map.is_empty() {
        print_warning("No usage data found for the specified criteria");
        return Ok(());
    }

    // Generate reports
    let daily_report = generate_daily_report(daily_map);
    let session_report = generate_session_report(session_map);

    // Handle export command
    if let Some(Commands::Export {
        daily,
        sessions,
        summary,
        output,
    }) = &cli.command
    {
        return handle_export_command(
            &daily_report,
            &session_report,
            *daily,
            *sessions,
            *summary,
            output,
            &config,
        );
    }

    // Handle cost command
    if let Some(Commands::Cost { today, date }) = &cli.command {
        return handle_cost_command(&daily_report, *today, date.as_deref());
    }

    // Handle debug state command
    if let Some(Commands::DebugState) = &cli.command {
        return handle_debug_state_command();
    }

    // Handle test resume command
    if let Some(Commands::TestResume) = &cli.command {
        return handle_test_resume_command(daily_report, session_report);
    }

    // Handle TUI flag or command
    if cli.tui {
        let mut tui_app = TuiApp::new(daily_report, session_report);

        // Try to restore previous session state
        if let Ok(state) = TuiSessionState::load() {
            if state.should_resume() {
                restore_tui_state(&mut tui_app, &state);
                tui_app.set_restored_state();
            }
        }

        let result = tui_app.run();

        // Save final state on exit
        save_tui_state(&tui_app, TuiMode::Basic).ok();
        return result;
    }

    // Handle Analytics TUI flag (temporarily disabled)
    // if cli.analytics_tui {
    //     let mut analytics_tui_app = AnalyticsTuiApp::new(daily_report, session_report);
    //     return analytics_tui_app.run();
    // }

    // Generate and display report based on command
    let command = cli.command.unwrap_or(Commands::Daily { classic: false });
    match command {
        Commands::Daily { classic } => {
            if daily_report.daily.is_empty() {
                print_warning("No daily usage data found for the specified date range");
            } else if cli.json {
                display_daily_report_json(&daily_report);
            } else if cli.classic || classic {
                display_daily_report_table(&daily_report);
            } else {
                display_daily_report_enhanced(&daily_report);
            }
        }
        Commands::Session { classic } => {
            if session_report.sessions.is_empty() {
                print_warning("No session usage data found for the specified date range");
            } else if cli.json {
                display_session_report_json(&session_report);
            } else if cli.classic || classic {
                display_session_report_table(&session_report);
            } else {
                display_session_report_enhanced(&session_report);
            }
        }
        Commands::Interactive => {
            if session_report.sessions.is_empty() {
                print_warning("No session data found for interactive selection");
            } else {
                let mut selector = InteractiveSelector::new(session_report);
                if let Some(selected_session) = selector.run()? {
                    println!("\n📊 Selected Session Details:");
                    println!(
                        "Path: {}/{}",
                        selected_session.project_path, selected_session.session_id
                    );
                    println!("Last Activity: {}", selected_session.last_activity);
                    println!("Input Tokens: {}", selected_session.input_tokens);
                    println!("Output Tokens: {}", selected_session.output_tokens);
                    println!("Total Cost: ${:.6}", selected_session.total_cost);
                }
            }
        }
        Commands::Tui => {
            let mut tui_app = TuiApp::new(daily_report, session_report);

            // Try to restore previous session state
            if let Ok(state) = TuiSessionState::load() {
                if state.should_resume() {
                    restore_tui_state(&mut tui_app, &state);
                    tui_app.set_restored_state();
                }
            }

            let result = tui_app.run();

            save_tui_state(&tui_app, TuiMode::Basic).ok();
            result?;
        }
        // Commands::AnalyticsTui => {
        //     let mut analytics_tui_app = AnalyticsTuiApp::new(daily_report, session_report);
        //     analytics_tui_app.run()?;
        // } // Temporarily disabled - work in progress
        _ => {} // Other commands handled above
    }

    Ok(())
}

/// Handle configuration management commands
fn handle_config_command(
    config: &mut Config,
    show: bool,
    reset: bool,
    set_path: Option<PathBuf>,
) -> Result<()> {
    if reset {
        *config = Config::default();
        config.save()?;
        print_info("Configuration reset to defaults");
        return Ok(());
    }

    if let Some(path) = set_path {
        config.set_claude_path(path.clone());
        config.save()?;
        print_info(&format!("Claude path set to: {}", path.display()));
        return Ok(());
    }

    if show {
        println!("Current Configuration:");
        println!("Claude Path: {:?}", config.claude_path);
        println!("Default Output Format: {:?}", config.default_output_format);
        println!("Default Command: {:?}", config.default_command);
        println!("Watch Interval: {}s", config.watch_interval_seconds);
        println!("Export Directory: {:?}", config.export_directory);
        println!("Date Format: {}", config.date_format);
        println!("Config File: {:?}", Config::config_path()?);
    }

    Ok(())
}

/// Handle data export commands
fn handle_export_command(
    daily_report: &crate::models::DailyReport,
    session_report: &crate::models::SessionReport,
    export_daily: bool,
    export_sessions: bool,
    export_summary: bool,
    output_path: &Option<PathBuf>,
    config: &Config,
) -> Result<()> {
    let base_path = output_path
        .clone()
        .unwrap_or_else(|| config.get_export_directory().join("claudelytics_export"));

    if export_daily {
        let path = base_path.with_extension("daily.csv");
        export_daily_to_csv(daily_report, &path)?;
        print_info(&format!("Daily report exported to: {}", path.display()));
    }

    if export_sessions {
        let path = base_path.with_extension("sessions.csv");
        export_sessions_to_csv(session_report, &path)?;
        print_info(&format!("Sessions report exported to: {}", path.display()));
    }

    if export_summary {
        let path = base_path.with_extension("summary.csv");
        export_summary_to_csv(daily_report, session_report, &path)?;
        print_info(&format!("Summary exported to: {}", path.display()));
    }

    if !export_daily && !export_sessions && !export_summary {
        // Export all by default
        let daily_path = base_path.with_extension("daily.csv");
        let sessions_path = base_path.with_extension("sessions.csv");
        let summary_path = base_path.with_extension("summary.csv");

        export_daily_to_csv(daily_report, &daily_path)?;
        export_sessions_to_csv(session_report, &sessions_path)?;
        export_summary_to_csv(daily_report, session_report, &summary_path)?;

        print_info(&format!("All reports exported to: {}", base_path.display()));
    }

    Ok(())
}

/// Handle cost summary commands
fn handle_cost_command(
    daily_report: &crate::models::DailyReport,
    today_only: bool,
    specific_date: Option<&str>,
) -> Result<()> {
    if today_only {
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        if let Some(daily_usage) = daily_report.daily.iter().find(|d| d.date == today) {
            println!("💰 Today's Usage Cost");
            println!("Date: {}", daily_usage.date);
            println!("Cost: ${:.4}", daily_usage.total_cost);
            println!("Tokens: {}", daily_usage.total_tokens);
        } else {
            print_warning("No usage data found for today");
        }
    } else if let Some(date_str) = specific_date {
        // Parse YYYYMMDD format to YYYY-MM-DD
        if date_str.len() == 8 {
            let formatted_date = format!(
                "{}-{}-{}",
                &date_str[0..4],
                &date_str[4..6],
                &date_str[6..8]
            );
            if let Some(daily_usage) = daily_report.daily.iter().find(|d| d.date == formatted_date)
            {
                println!("💰 Usage Cost for {}", formatted_date);
                println!("Date: {}", daily_usage.date);
                println!("Cost: ${:.4}", daily_usage.total_cost);
                println!("Tokens: {}", daily_usage.total_tokens);
            } else {
                print_warning(&format!("No usage data found for {}", formatted_date));
            }
        } else {
            anyhow::bail!("Date must be in YYYYMMDD format");
        }
    } else {
        // Show total cost summary
        println!("💰 Total Cost Summary");
        println!("Total Cost: ${:.4}", daily_report.totals.total_cost);
        println!("Total Tokens: {}", daily_report.totals.total_tokens);
        println!("Days with usage: {}", daily_report.daily.len());

        if let Some(latest) = daily_report.daily.first() {
            println!("Latest usage: {} (${:.4})", latest.date, latest.total_cost);
        }
    }

    Ok(())
}

/// Save TUI session state for resume functionality
fn save_tui_state(tui_app: &TuiApp, mode: TuiMode) -> Result<()> {
    let mut state = TuiSessionState::load().unwrap_or_default();
    state.mode = mode;
    state.update_timestamp();

    // Extract actual state from TUI app
    state.last_tab = Some(tui_app.get_current_tab_index());
    state.last_search_query = if tui_app.get_search_query().is_empty() {
        None
    } else {
        Some(tui_app.get_search_query())
    };
    state.bookmarked_sessions = tui_app.get_bookmarked_sessions();
    state.comparison_sessions = tui_app.get_comparison_sessions();
    state.last_session_path = tui_app.get_selected_session_path();

    state.save()
}

/// Restore TUI session state from saved data
fn restore_tui_state(tui_app: &mut TuiApp, state: &TuiSessionState) {
    // Silently restore state without logging

    // Restore last active tab
    if let Some(tab_index) = state.last_tab {
        tui_app.set_current_tab(tab_index);
    }

    // Restore search query
    if let Some(ref search_query) = state.last_search_query {
        tui_app.set_search_query(search_query.clone());
    }

    // Restore bookmarked sessions
    if !state.bookmarked_sessions.is_empty() {
        tui_app.set_bookmarked_sessions(state.bookmarked_sessions.clone());
    }

    // Restore comparison sessions
    if !state.comparison_sessions.is_empty() {
        tui_app.set_comparison_sessions(state.comparison_sessions.clone());
    }

    // Restore last selected session
    if let Some(ref session_path) = state.last_session_path {
        tui_app.restore_session_selection(Some(session_path.clone()));
    }
}

/// Handle debug state command to show current TUI session state
fn handle_debug_state_command() -> Result<()> {
    let state = TuiSessionState::load().unwrap_or_default();

    println!("🔧 TUI Session State Debug Information");
    println!("=====================================");
    println!("Mode: {:?}", state.mode);
    println!("Last Tab: {:?}", state.last_tab);
    println!("Last Session Path: {:?}", state.last_session_path);
    println!("Last Search Query: {:?}", state.last_search_query);
    println!(
        "Bookmarked Sessions: {} items",
        state.bookmarked_sessions.len()
    );
    for (i, bookmark) in state.bookmarked_sessions.iter().enumerate() {
        println!("  {}. {}", i + 1, bookmark);
    }
    println!(
        "Comparison Sessions: {} items",
        state.comparison_sessions.len()
    );
    for (i, comparison) in state.comparison_sessions.iter().enumerate() {
        println!("  {}. {}", i + 1, comparison);
    }
    println!("Timestamp: {}", state.timestamp);
    println!("Should Resume: {}", state.should_resume());

    let state_path = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let state_file = format!("{}/.claude/claudelytics/tui_session.json", state_path);
    println!("State File: {}", state_file);

    if std::path::Path::new(&state_file).exists() {
        println!("\n📄 Raw State File Content:");
        if let Ok(content) = std::fs::read_to_string(&state_file) {
            println!("{}", content);
        }
    } else {
        println!("❌ State file does not exist");
    }

    Ok(())
}

/// Handle test resume command to verify resume functionality
fn handle_test_resume_command(
    daily_report: crate::models::DailyReport,
    session_report: crate::models::SessionReport,
) -> Result<()> {
    print_info("🧪 Testing resume functionality...");

    // Create a test TUI app and set some state
    let mut tui_app = TuiApp::new(daily_report, session_report);

    // Set some test state
    tui_app.set_current_tab(2); // Sessions tab
    tui_app.set_search_query("test_query".to_string());
    tui_app.set_bookmarked_sessions(vec![
        "test/bookmark1".to_string(),
        "test/bookmark2".to_string(),
    ]);
    tui_app.set_comparison_sessions(vec!["test/comparison1".to_string()]);

    print_info("  ✓ Set test state in TUI app");

    // Save the state
    if let Err(e) = save_tui_state(&tui_app, TuiMode::Basic) {
        print_error(&format!("Failed to save state: {}", e));
        return Err(e);
    }
    print_info("  ✓ Saved test state");

    // Load the state back
    let loaded_state = TuiSessionState::load()?;
    print_info("  ✓ Loaded state back");

    // Create a new TUI app and restore state
    let mut new_tui_app = TuiApp::new(
        tui_app.get_daily_report().clone(),
        tui_app.get_session_report().clone(),
    );
    restore_tui_state(&mut new_tui_app, &loaded_state);

    // Verify state was restored correctly
    let restored_tab = new_tui_app.get_current_tab_index();
    let restored_query = new_tui_app.get_search_query();
    let restored_bookmarks = new_tui_app.get_bookmarked_sessions();
    let restored_comparisons = new_tui_app.get_comparison_sessions();

    println!("🔍 Verification Results:");
    println!(
        "  Tab: {} (expected: 2) {}",
        restored_tab,
        if restored_tab == 2 { "✓" } else { "❌" }
    );
    println!(
        "  Search: '{}' (expected: 'test_query') {}",
        restored_query,
        if restored_query == "test_query" {
            "✓"
        } else {
            "❌"
        }
    );
    println!(
        "  Bookmarks: {} (expected: 2) {}",
        restored_bookmarks.len(),
        if restored_bookmarks.len() == 2 {
            "✓"
        } else {
            "❌"
        }
    );
    println!(
        "  Comparisons: {} (expected: 1) {}",
        restored_comparisons.len(),
        if restored_comparisons.len() == 1 {
            "✓"
        } else {
            "❌"
        }
    );

    if restored_tab == 2
        && restored_query == "test_query"
        && restored_bookmarks.len() == 2
        && restored_comparisons.len() == 1
    {
        print_info("🎉 Resume functionality test PASSED!");
    } else {
        print_error("❌ Resume functionality test FAILED!");
    }

    Ok(())
}
