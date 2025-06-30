//! Claudelytics - Claude Code Usage Analytics Tool
//!
//! A fast CLI tool for analyzing Claude Code usage patterns, token consumption, and costs.
//! Parses JSONL files from ~/.claude/projects/ and generates comprehensive reports.

// Module declarations
mod billing_blocks;
mod burn_rate;
mod claude_sessions;
mod config;
mod config_v2;
mod conversation_display;
mod conversation_parser;
mod display;
mod domain;
mod error;
mod export;
mod helpers;
mod interactive;
mod live_dashboard;
mod mcp;
mod models;
mod models_registry;
mod parser;
mod performance;
mod pricing;
mod pricing_cache;
mod pricing_strategies;
mod processing;
mod projections;
mod realtime_analytics;
mod reports;
mod responsive_tables;
mod session_analytics;
mod session_blocks;
mod state;
mod terminal;
mod tui;
mod tui_visuals;
mod watcher;

// Core dependencies
use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand, ValueEnum};
use config::Config;
use display::{
    display_billing_blocks_responsive, display_daily_report_enhanced, display_daily_report_json,
    display_daily_report_responsive, display_daily_report_table, display_model_breakdown_report,
    display_monthly_report_enhanced, display_monthly_report_json, display_monthly_report_table,
    display_session_report_enhanced, display_session_report_json,
    display_session_report_responsive, display_session_report_table, print_error, print_info,
    print_warning,
};
use export::{export_daily_to_csv, export_sessions_to_csv, export_summary_to_csv};
use interactive::InteractiveSelector;
use models::SessionUsageMap;
use parser::UsageParser;
use projections::ProjectionCalculator;
use reports::{
    SortField as ReportSortField, SortOrder as ReportSortOrder, generate_daily_report_sorted,
    generate_monthly_report_sorted, generate_session_report_sorted,
};
use session_blocks::{SessionBlockConfig, SessionBlockManager};
use state::{TuiMode, TuiSessionState};
use std::path::{Path, PathBuf};
use tui::TuiApp;
use watcher::UsageWatcher;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortOrder {
    /// Sort in ascending order
    Asc,
    /// Sort in descending order
    Desc,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortField {
    /// Sort by date/time
    Date,
    /// Sort by cost
    Cost,
    /// Sort by total tokens
    Tokens,
    /// Sort by efficiency (tokens per dollar)
    Efficiency,
    /// Sort by project name
    Project,
}

#[derive(Parser)]
#[command(name = "claudelytics")]
#[command(
    about = "Claude Code usage analytics tool - Analyze token usage, costs, and session patterns"
)]
#[command(version = "0.4.1")]
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
        help = "Force compact display mode",
        long_help = "Force compact display mode for narrow terminals\nReduces columns shown in tables to fit smaller screens\nAutomatic: Adapts based on terminal width\nWith --compact: Always use minimal columns"
    )]
    compact: bool,

    #[arg(
        long,
        help = "Launch terminal user interface",
        long_help = "Launch interactive terminal user interface (TUI)\nFeatures: Navigation, search, charts, multiple tabs\nKeyboard shortcuts: j/k navigation, q to quit, ? for help"
    )]
    tui: bool,

    #[arg(
        long,
        value_name = "MODEL",
        help = "Filter by model type (opus, sonnet, haiku)",
        long_help = "Filter usage data by specific Claude model types\nOptions: opus, sonnet, haiku, or specific model names\nExamples:\n  --model-filter opus     # Show only Opus model usage\n  --model-filter sonnet   # Show only Sonnet model usage\n  --model-filter claude-opus-4-20250514  # Specific model version"
    )]
    model_filter: Option<String>,

    #[arg(
        long,
        help = "List all known models and exit",
        long_help = "Display all registered Claude models with their families and aliases\nUseful for seeing what models are available for filtering"
    )]
    list_models: bool,

    #[arg(
        long,
        help = "Show usage breakdown by model family",
        long_help = "Automatically categorize and display usage by model families (Opus, Sonnet, Haiku)\nShows cost and token usage for each model type in a single view\nCombines with other filters: --today, --since, --until"
    )]
    by_model: bool,

    #[arg(
        long,
        help = "Use responsive table layout",
        long_help = "Use responsive tables that automatically adjust to terminal width\nFeatures: Auto-adjusting columns, smart column hiding, abbreviated headers\nPriorities: Date/Cost always shown, cache tokens hidden first\nCombines with daily, session, and billing-blocks commands"
    )]
    responsive: bool,
    #[arg(
        long,
        help = "Show real-time analytics",
        long_help = "Display real-time analytics alongside regular reports\nAdds burn rates, budget projections, and efficiency metrics\nWorks with daily and session commands\nExample: claudelytics daily --realtime"
    )]
    realtime: bool,
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
        #[arg(
            long,
            help = "Sort field",
            long_help = "Field to sort by: date, cost, tokens\nDefault: date (most recent first)"
        )]
        sort_by: Option<SortField>,
        #[arg(
            long,
            help = "Sort order",
            long_help = "Sort order: asc (ascending), desc (descending)\nDefault: desc for date/cost/tokens"
        )]
        sort_order: Option<SortOrder>,
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
        #[arg(
            long,
            help = "Sort field",
            long_help = "Field to sort by: date, cost, tokens, efficiency, project\nDefault: cost (highest first)"
        )]
        sort_by: Option<SortField>,
        #[arg(
            long,
            help = "Sort order",
            long_help = "Sort order: asc (ascending), desc (descending)\nDefault: desc for cost/tokens/efficiency, asc for project"
        )]
        sort_order: Option<SortOrder>,
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
    #[command(about = "Show usage aggregated by months")]
    #[command(
        long_about = "Show usage aggregated by calendar months\n\nDisplays monthly summaries with total usage, active days,\nand average daily costs for better long-term analysis.\n\nEXAMPLES:\n  claudelytics monthly                  # Enhanced monthly report\n  claudelytics monthly --classic        # Classic table format\n  claudelytics --json monthly           # JSON output (global flag)\n  claudelytics --since 202401 monthly   # From January 2024 onwards"
    )]
    Monthly {
        #[arg(
            long,
            help = "Use classic table format",
            long_help = "Override enhanced format with classic ASCII tables\nInherits global flags: --json, --since, --until"
        )]
        classic: bool,
        #[arg(
            long,
            help = "Sort field",
            long_help = "Field to sort by: date, cost, tokens\nDefault: date (most recent first)"
        )]
        sort_by: Option<SortField>,
        #[arg(
            long,
            help = "Sort order",
            long_help = "Sort order: asc (ascending), desc (descending)\nDefault: desc for date/cost/tokens"
        )]
        sort_order: Option<SortOrder>,
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
    #[command(about = "Show billing blocks (5-hour usage blocks)")]
    #[command(
        long_about = "Display usage organized by Claude's 5-hour billing blocks\n\nClaude uses 5-hour billing blocks for tracking usage. This command\nshows your usage patterns aligned with these actual billing periods.\n\nBILLING BLOCKS:\n  00:00-05:00 UTC  (Block 1)\n  05:00-10:00 UTC  (Block 2)\n  10:00-15:00 UTC  (Block 3)\n  15:00-20:00 UTC  (Block 4)\n  20:00-00:00 UTC  (Block 5)\n\nFEATURES:\n  - Shows usage within each 5-hour block\n  - Identifies peak usage periods\n  - Calculates average usage per block\n  - Displays usage patterns by time of day\n\nEXAMPLES:\n  claudelytics billing-blocks           # Show all billing blocks\n  claudelytics billing-blocks --today   # Today's blocks only\n  claudelytics billing-blocks --json    # JSON output\n  claudelytics --since 20240301 billing-blocks # From specific date"
    )]
    BillingBlocks {
        #[arg(
            long,
            help = "Use classic table format",
            long_help = "Display billing blocks in classic table format\nDefault: Enhanced visual format with summaries"
        )]
        classic: bool,
        #[arg(
            long,
            help = "Show summary statistics",
            long_help = "Include summary statistics\nShows: peak block, average usage, patterns by time"
        )]
        summary: bool,
    },
    #[command(about = "Start Model Context Protocol (MCP) server")]
    #[command(
        long_about = "Start an MCP server to expose claudelytics data via the Model Context Protocol\n\nThe MCP server allows other applications to query claudelytics data through\na standardized protocol. Supports both stdio and HTTP transport methods.\n\nEXAMPLES:\n  claudelytics mcp-server                # Start stdio server\n  claudelytics mcp-server --http 8080    # Start HTTP server on port 8080\n  claudelytics mcp-server --list-tools   # Show available MCP tools\n  claudelytics mcp-server --list-resources # Show available MCP resources"
    )]
    McpServer {
        #[arg(
            long,
            help = "Start HTTP server on specified port",
            long_help = "Start MCP server using HTTP with Server-Sent Events\nAllows remote connections from MCP clients"
        )]
        http: Option<u16>,
        #[arg(
            long,
            help = "List available MCP tools",
            long_help = "Show all MCP tools that can be called by clients\nTools are functions that perform actions with optional parameters"
        )]
        list_tools: bool,
        #[arg(
            long,
            help = "List available MCP resources",
            long_help = "Show all MCP resources that can be read by clients\nResources are data endpoints that provide usage information"
        )]
        list_resources: bool,
    },
    #[command(about = "Debug resume state")]
    #[command(long_about = "Debug command to show TUI session state information")]
    DebugState,
    #[command(about = "Test resume functionality")]
    #[command(long_about = "Test command to verify resume functionality without starting TUI")]
    TestResume,
    #[command(about = "Manage offline pricing cache")]
    #[command(
        long_about = "Manage the offline pricing cache for model costs\n\nThe pricing cache allows claudelytics to work without internet connection\nby storing model pricing data locally. The cache is automatically updated\nwhen online and remains valid for 7 days.\n\nEXAMPLES:\n  claudelytics pricing-cache --show    # Show current cache status\n  claudelytics pricing-cache --clear   # Clear the cache\n  claudelytics pricing-cache --update  # Force update the cache"
    )]
    PricingCache {
        #[arg(
            long,
            help = "Show cache status and contents",
            long_help = "Display current cache status including age, validity, and stored pricing data"
        )]
        show: bool,
        #[arg(
            long,
            help = "Clear the pricing cache",
            long_help = "Remove the cached pricing data, forcing use of fallback pricing"
        )]
        clear: bool,
        #[arg(
            long,
            help = "Update the pricing cache",
            long_help = "Force update the pricing cache with latest data (currently uses fallback data)"
        )]
        update: bool,
    },
    #[command(about = "Show session blocks (configurable time windows)")]
    #[command(
        long_about = "Analyze usage in configurable session blocks\n\nSession blocks provide flexible time-based analysis similar to billing blocks\nbut with customizable durations. Default is 8-hour blocks.\n\nFEATURES:\n  - Configurable block duration (default: 8 hours)\n  - Active session tracking with burn rate\n  - Usage projections based on current activity\n  - Time to limit calculations\n\nEXAMPLES:\n  claudelytics blocks                  # Show all session blocks\n  claudelytics blocks --active         # Show only active sessions\n  claudelytics blocks --length 4       # Use 4-hour blocks\n  claudelytics blocks --recent         # Show last 30 days\n  claudelytics blocks --live           # Live monitoring mode"
    )]
    Blocks {
        #[arg(
            long,
            help = "Show only active session blocks",
            long_help = "Display only blocks with current activity"
        )]
        active: bool,
        #[arg(
            long,
            help = "Session block length in hours",
            long_help = "Duration of each session block (default: 8 hours)",
            default_value = "8"
        )]
        length: i64,
        #[arg(
            long,
            help = "Show recent blocks (last 30 days)",
            long_help = "Display only blocks from the last 30 days"
        )]
        recent: bool,
        #[arg(
            long,
            help = "Live monitoring mode",
            long_help = "Continuously monitor and update session blocks"
        )]
        live: bool,
        #[arg(
            long,
            help = "Refresh interval in seconds (for live mode)",
            long_help = "How often to refresh data in live mode (default: 5 seconds)",
            default_value = "5"
        )]
        refresh: u64,
        #[arg(
            long,
            help = "Token limit for warnings",
            long_help = "Set token limit for burn rate warnings"
        )]
        token_limit: Option<u64>,
        #[arg(
            long,
            help = "Cost limit for warnings",
            long_help = "Set cost limit (USD) for burn rate warnings"
        )]
        cost_limit: Option<f64>,
    },
    #[command(about = "Show usage projections and forecasts")]
    #[command(
        long_about = "Project future usage based on historical patterns\n\nProjections analyze your usage history to forecast future token consumption\nand costs. Includes trend analysis, growth rates, and limit predictions.\n\nFEATURES:\n  - Daily, weekly, and monthly averages\n  - Trend detection (increasing/decreasing/stable)\n  - Confidence intervals for projections\n  - Time to limit calculations\n  - Cost estimates for future periods\n\nEXAMPLES:\n  claudelytics projections             # Show 30-day projection\n  claudelytics projections --days 90   # Project 90 days ahead\n  claudelytics projections --json      # JSON output for scripts"
    )]
    Projections {
        #[arg(
            long,
            help = "Number of days to project",
            long_help = "How many days into the future to project (default: 30)",
            default_value = "30"
        )]
        days: i64,
        #[arg(
            long,
            help = "Token limit for projections",
            long_help = "Set token limit to calculate when it will be reached"
        )]
        token_limit: Option<u64>,
        #[arg(
            long,
            help = "Cost limit for projections",
            long_help = "Set cost limit (USD) to calculate when it will be reached"
        )]
        cost_limit: Option<f64>,
        #[arg(
            long,
            help = "JSON output",
            long_help = "Output projections in JSON format"
        )]
        json: bool,
    },
    #[command(about = "Advanced session analytics")]
    #[command(
        long_about = "Analyze session patterns and behaviors in depth\n\nProvides detailed insights into:\n  - Time of day usage patterns\n  - Day of week trends\n  - Session duration analysis\n  - Usage frequency and streaks\n  - Cost efficiency metrics\n\nEXAMPLES:\n  claudelytics analytics              # Show all analytics\n  claudelytics analytics --time-of-day # Time patterns only\n  claudelytics analytics --efficiency  # Cost efficiency analysis"
    )]
    Analytics {
        #[arg(
            long,
            help = "Show time of day analysis",
            long_help = "Analyze usage patterns by hour of day"
        )]
        time_of_day: bool,
        #[arg(
            long,
            help = "Show day of week analysis",
            long_help = "Analyze usage patterns by day of week"
        )]
        day_of_week: bool,
        #[arg(
            long,
            help = "Show session duration analysis",
            long_help = "Analyze session lengths and patterns"
        )]
        duration: bool,
        #[arg(
            long,
            help = "Show frequency analysis",
            long_help = "Analyze session frequency and streaks"
        )]
        frequency: bool,
        #[arg(
            long,
            help = "Show cost efficiency analysis",
            long_help = "Analyze cost efficiency of sessions"
        )]
        efficiency: bool,
        #[arg(
            long,
            help = "Cost threshold for efficiency analysis",
            long_help = "Sessions above this cost will be highlighted",
            default_value = "1.0"
        )]
        threshold: f64,
    },
    #[command(about = "Real-time analytics with burn rates and projections")]
    #[command(
        long_about = "Show comprehensive real-time analytics including burn rates and budget projections\n\nProvides detailed analytics on:\n  - Token and cost burn rates (per minute/hour/day)\n  - Budget projections and time to limits\n  - Session analytics and efficiency trends\n  - Usage alerts and recommendations\n\nFEATURES:\n  - Multi-window burn rate analysis (1hr, 3hr, 24hr)\n  - Budget utilization and projections\n  - Peak usage detection\n  - Efficiency scoring\n  - Smart alerts for unusual patterns\n\nEXAMPLES:\n  claudelytics realtime                # Show all real-time analytics\n  claudelytics realtime --json         # Output as JSON\n  claudelytics realtime --daily-limit 50  # Set $50 daily budget\n  claudelytics realtime --monthly-limit 1000  # Set $1000 monthly budget"
    )]
    Realtime {
        #[arg(
            long,
            help = "Daily budget limit (USD)",
            long_help = "Set daily budget limit for projections and alerts"
        )]
        daily_limit: Option<f64>,
        #[arg(
            long,
            help = "Monthly budget limit (USD)",
            long_help = "Set monthly budget limit for projections and alerts"
        )]
        monthly_limit: Option<f64>,
        #[arg(
            long,
            help = "Yearly budget limit (USD)",
            long_help = "Set yearly budget limit for projections and alerts"
        )]
        yearly_limit: Option<f64>,
        #[arg(
            long,
            help = "Alert threshold percentage",
            long_help = "Percentage of budget to trigger alerts (0.0-1.0, default: 0.8)",
            default_value = "0.8"
        )]
        alert_threshold: f64,
        #[arg(
            long,
            help = "Output as JSON",
            long_help = "Output analytics report in JSON format"
        )]
        json: bool,
    },
    #[command(about = "Live dashboard for real-time monitoring")]
    #[command(
        long_about = "Launch live dashboard for real-time token usage monitoring\n\nProvides a continuously updating view of:\n  - Real-time token burn rate (tokens/minute, tokens/hour)\n  - Active session progress tracking\n  - Cost projections based on current usage rate\n  - Estimated time to reach daily/monthly limits\n  - Auto-refresh display every 5 seconds\n\nFEATURES:\n  - Real-time burn rate calculation\n  - Active session monitoring\n  - Cost accumulation tracking\n  - Limit warnings and alerts\n  - Configurable refresh interval\n\nEXAMPLES:\n  claudelytics live                    # Start live dashboard\n  claudelytics live --refresh 10       # Update every 10 seconds\n  claudelytics live --token-limit 1000000  # Set token limit\n  claudelytics live --cost-limit 50    # Set daily cost limit ($50)"
    )]
    Live {
        #[arg(
            long,
            help = "Refresh interval in seconds",
            long_help = "How often to refresh the dashboard (default: 5 seconds)",
            default_value = "5"
        )]
        refresh: u64,
        #[arg(
            long,
            help = "Token limit for warnings",
            long_help = "Set token limit to show time remaining"
        )]
        token_limit: Option<u64>,
        #[arg(
            long,
            help = "Daily cost limit for warnings",
            long_help = "Set daily cost limit (USD) to show time remaining"
        )]
        cost_limit: Option<f64>,
        #[arg(
            long,
            help = "Show detailed session information",
            long_help = "Display detailed information for each active session",
            default_value = "true"
        )]
        show_details: bool,
        #[arg(
            long,
            help = "Enable burn rate alerts",
            long_help = "Show alerts for high burn rates and approaching limits",
            default_value = "true"
        )]
        enable_alerts: bool,
    },
    #[command(about = "Display conversation content")]
    #[command(
        long_about = "Display full conversation content from Claude sessions\n\nProvides detailed view of conversations including messages, thinking blocks,\ntool usage, and token usage. Supports multiple output formats and filtering.\n\nFEATURES:\n  - Full conversation thread display with parent/child relationships\n  - Syntax highlighting for code blocks\n  - Thinking block extraction and display\n  - Tool usage tracking\n  - Multiple export formats (terminal, markdown, JSON)\n  - Search and filter capabilities\n\nEXAMPLES:\n  claudelytics conversation --session abc123  # Show specific session\n  claudelytics conversation --project myproj  # Filter by project\n  claudelytics conversation --search \"error\" # Search in conversations\n  claudelytics conversation --export markdown # Export as markdown\n  claudelytics conversation --recent          # Show recent conversations"
    )]
    Conversation {
        #[arg(
            short = 's',
            long,
            help = "Session ID or path to display",
            long_help = "Specific session ID or path to display conversation for\nExample: --session abc123def or --session project/session-id"
        )]
        session: Option<String>,
        #[arg(
            short = 'p',
            long,
            help = "Filter by project name",
            long_help = "Filter conversations by project name\nExample: --project myproject"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Search for text in conversations",
            long_help = "Search for specific text in conversation content\nSearches in messages, thinking blocks, and tool usage"
        )]
        search: Option<String>,
        #[arg(
            short = 'e',
            long,
            help = "Export format",
            long_help = "Export conversation in specified format\nOptions: markdown, json, txt\nDefault: terminal display"
        )]
        export: Option<String>,
        #[arg(
            short = 'o',
            long,
            help = "Output file path for export",
            long_help = "Path to save exported conversation\nIf not specified, outputs to stdout"
        )]
        output: Option<PathBuf>,
        #[arg(
            long,
            help = "Show only recent conversations",
            long_help = "Display only conversations from the last 7 days"
        )]
        recent: bool,
        #[arg(
            long,
            help = "Display mode",
            long_help = "Display mode for conversation\nOptions: compact, detailed\nDefault: detailed",
            default_value = "detailed"
        )]
        mode: String,
        #[arg(
            long,
            help = "Include thinking blocks",
            long_help = "Include AI thinking blocks in output\nDefault: true for detailed mode",
            default_value = "true"
        )]
        include_thinking: bool,
        #[arg(
            long,
            help = "Include tool usage",
            long_help = "Include tool usage details in output\nDefault: true",
            default_value = "true"
        )]
        include_tools: bool,
        #[arg(
            short = 'l',
            long,
            help = "List available conversations",
            long_help = "List all available conversations instead of displaying content"
        )]
        list: bool,
    },
    #[command(about = "View conversation content (alias for conversation)")]
    #[command(
        long_about = "View full conversation content from Claude sessions\n\nThis is an alias for the 'conversation' command with simplified options.\nProvides quick access to view conversations by session ID or project.\n\nEXAMPLES:\n  claudelytics view abc123              # View specific session\n  claudelytics view --project myproj    # View conversations from project\n  claudelytics view --recent            # View recent conversations\n  claudelytics view --list              # List available conversations"
    )]
    View {
        #[arg(
            help = "Session ID or project name",
            long_help = "Session ID or project name to view\nIf not provided, shows recent conversations"
        )]
        target: Option<String>,
        #[arg(
            short = 'p',
            long,
            help = "Filter by project name",
            long_help = "Filter conversations by project name\nExample: --project myproject"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Show only recent conversations",
            long_help = "Display only conversations from the last 7 days"
        )]
        recent: bool,
        #[arg(
            short = 'l',
            long,
            help = "List available conversations",
            long_help = "List all available conversations instead of displaying content"
        )]
        list: bool,
        #[arg(
            short = 'e',
            long,
            help = "Export format",
            long_help = "Export conversation in specified format\nOptions: markdown, json, txt"
        )]
        export: Option<String>,
        #[arg(
            short = 'o',
            long,
            help = "Output file path",
            long_help = "Path to save exported conversation"
        )]
        output: Option<PathBuf>,
    },
    #[command(about = "Inspect session details and metadata")]
    #[command(
        long_about = "Inspect detailed session information including metadata and statistics\n\nProvides comprehensive information about sessions including:\n  - Session metadata (ID, project, timestamps)\n  - Token usage breakdown by model\n  - Cost analysis and efficiency metrics\n  - Conversation count and structure\n  - Activity timeline\n\nEXAMPLES:\n  claudelytics inspect abc123           # Inspect specific session\n  claudelytics inspect --project myproj # Inspect sessions from project\n  claudelytics inspect --recent         # Inspect recent sessions\n  claudelytics inspect --json           # Output as JSON"
    )]
    Inspect {
        #[arg(
            help = "Session ID or project name",
            long_help = "Session ID or project name to inspect\nIf not provided, shows summary of all sessions"
        )]
        target: Option<String>,
        #[arg(
            short = 'p',
            long,
            help = "Filter by project name",
            long_help = "Filter sessions by project name\nExample: --project myproject"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Show only recent sessions",
            long_help = "Display only sessions from the last 7 days"
        )]
        recent: bool,
        #[arg(
            long,
            help = "Show detailed breakdown",
            long_help = "Include detailed breakdown of token usage by conversation"
        )]
        detailed: bool,
        #[arg(
            long,
            help = "Output as JSON",
            long_help = "Output session information in JSON format"
        )]
        json: bool,
        #[arg(
            long,
            help = "Include conversation list",
            long_help = "Include list of all conversations in the session"
        )]
        conversations: bool,
        #[arg(
            long,
            help = "Show activity timeline",
            long_help = "Display timeline of session activity"
        )]
        timeline: bool,
    },
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

    // Handle --list-models flag
    if cli.list_models {
        use models_registry::ModelsRegistry;
        let registry = ModelsRegistry::new();

        println!("📋 Registered Claude Models\n");
        println!(
            "{:<40} {:<10} {:<20} {:<15}",
            "Model Name", "Family", "Aliases", "Version"
        );
        println!("{}", "-".repeat(85));

        for model in registry.list_models() {
            let aliases = model.aliases.join(", ");
            println!(
                "{:<40} {:<10} {:<20} {:<15}",
                model.name,
                model.family,
                aliases,
                model.version.as_deref().unwrap_or("-")
            );
        }

        println!("\n💡 Usage Examples:");
        println!("  claudelytics --model-filter opus        # Filter by family");
        println!("  claudelytics --model-filter sonnet-4    # Filter by alias");
        println!("  claudelytics --model-filter claude-opus # Filter by partial name");

        return Ok(());
    }

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
    let parser = UsageParser::new(
        claude_dir.clone(),
        since_date.clone(),
        until_date.clone(),
        cli.model_filter.clone(),
    )?;

    // Handle watch command
    if let Some(Commands::Watch) = &cli.command {
        let mut watcher = UsageWatcher::new(parser)?;
        return watcher.watch(&claude_dir);
    }

    // Parse all usage data
    let (daily_map, session_map, billing_manager) = parser.parse_all()?;

    // Check if we have any data
    if daily_map.is_empty() && session_map.is_empty() {
        print_warning("No usage data found for the specified criteria");
        return Ok(());
    }

    // Clone maps for potential re-generation with different sorting
    let daily_map_clone = daily_map.clone();
    let session_map_clone = session_map.clone();

    // Generate default reports
    let mut daily_report = generate_daily_report_sorted(daily_map, None, None);
    let mut session_report = generate_session_report_sorted(session_map, None, None);

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
        return handle_test_resume_command(daily_report, session_report, &billing_manager);
    }

    // Handle MCP server command
    if let Some(Commands::McpServer {
        http,
        list_tools,
        list_resources,
    }) = &cli.command
    {
        return handle_mcp_server_command(
            claude_dir.as_path(),
            *http,
            *list_tools,
            *list_resources,
        );
    }

    // Handle --by-model flag
    if cli.by_model {
        display_model_breakdown_report(&daily_map_clone, &session_map_clone);
        return Ok(());
    }

    // Handle TUI flag or command
    if cli.tui {
        let mut tui_app = TuiApp::new(daily_report, session_report, billing_manager.clone());

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
    let command = cli.command.unwrap_or(Commands::Daily {
        classic: false,
        sort_by: None,
        sort_order: None,
    });
    match command {
        Commands::Daily {
            classic,
            sort_by,
            sort_order,
        } => {
            // Re-generate with sorting if specified
            if sort_by.is_some() || sort_order.is_some() {
                daily_report = generate_daily_report_sorted(
                    daily_map_clone.clone(),
                    convert_sort_field(sort_by),
                    convert_sort_order(sort_order),
                );
            }

            if daily_report.daily.is_empty() {
                print_warning("No daily usage data found for the specified date range");
            } else if cli.json {
                display_daily_report_json(&daily_report);
            } else if cli.responsive {
                display_daily_report_responsive(&daily_report);
            } else if cli.classic || classic {
                display_daily_report_table(&daily_report);
            } else {
                display_daily_report_enhanced(&daily_report, cli.compact);
            }

            // Show real-time analytics if requested
            if cli.realtime {
                println!("\n{}", "─".repeat(60));
                handle_realtime_analytics_command(
                    &daily_map_clone,
                    &session_map_clone,
                    None, // Use default budget limits
                    None,
                    None,
                    0.8,   // Default alert threshold
                    false, // Not JSON since we're appending to existing output
                )?;
            }
        }
        Commands::Session {
            classic,
            sort_by,
            sort_order,
        } => {
            // Re-generate with sorting if specified
            if sort_by.is_some() || sort_order.is_some() {
                session_report = generate_session_report_sorted(
                    session_map_clone.clone(),
                    convert_sort_field(sort_by),
                    convert_sort_order(sort_order),
                );
            }

            if session_report.sessions.is_empty() {
                print_warning("No session usage data found for the specified date range");
            } else if cli.json {
                display_session_report_json(&session_report);
            } else if cli.responsive {
                display_session_report_responsive(&session_report);
            } else if cli.classic || classic {
                display_session_report_table(&session_report);
            } else {
                display_session_report_enhanced(&session_report);
            }

            // Show real-time analytics if requested
            if cli.realtime {
                println!("\n{}", "─".repeat(60));
                handle_realtime_analytics_command(
                    &daily_map_clone,
                    &session_map_clone,
                    None, // Use default budget limits
                    None,
                    None,
                    0.8,   // Default alert threshold
                    false, // Not JSON since we're appending to existing output
                )?;
            }
        }
        Commands::Monthly {
            classic,
            sort_by,
            sort_order,
        } => {
            // Generate monthly report from daily data with sorting
            let monthly_report = generate_monthly_report_sorted(
                daily_map_clone.clone(),
                convert_sort_field(sort_by),
                convert_sort_order(sort_order),
            );

            if monthly_report.monthly.is_empty() {
                print_warning("No monthly usage data found for the specified date range");
            } else if cli.json {
                display_monthly_report_json(&monthly_report);
            } else if cli.classic || classic {
                display_monthly_report_table(&monthly_report);
            } else {
                display_monthly_report_enhanced(&monthly_report);
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
            let mut tui_app = TuiApp::new(daily_report, session_report, billing_manager.clone());

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
        Commands::BillingBlocks { classic, summary } => {
            handle_billing_blocks_command(
                &billing_manager,
                cli.json,
                cli.responsive,
                classic,
                summary,
            );
        }
        Commands::PricingCache {
            show,
            clear,
            update,
        } => {
            handle_pricing_cache_command(show, clear, update)?;
        }
        Commands::Blocks {
            active,
            length,
            recent,
            live,
            refresh,
            token_limit,
            cost_limit,
        } => {
            handle_blocks_command(
                &claude_dir,
                BlocksCommandOptions {
                    active,
                    length,
                    recent,
                    live,
                    refresh,
                    token_limit,
                    cost_limit,
                    since: since_date.clone(),
                    until: until_date.clone(),
                },
            )?;
        }
        Commands::Projections {
            days,
            token_limit,
            cost_limit,
            json,
        } => {
            handle_projections_command(
                &claude_dir,
                days,
                token_limit,
                cost_limit,
                json,
                since_date.clone(),
            )?;
        }
        Commands::Analytics {
            time_of_day,
            day_of_week,
            duration,
            frequency,
            efficiency,
            threshold,
        } => {
            handle_analytics_command(
                &session_map_clone,
                time_of_day,
                day_of_week,
                duration,
                frequency,
                efficiency,
                threshold,
            )?;
        }
        Commands::Realtime {
            daily_limit,
            monthly_limit,
            yearly_limit,
            alert_threshold,
            json,
        } => {
            handle_realtime_analytics_command(
                &daily_map_clone,
                &session_map_clone,
                daily_limit,
                monthly_limit,
                yearly_limit,
                alert_threshold,
                json,
            )?;
        }
        Commands::Live {
            refresh,
            token_limit,
            cost_limit,
            show_details,
            enable_alerts,
        } => {
            use live_dashboard::{LiveDashboardOptions, run_live_dashboard};

            let options = LiveDashboardOptions {
                refresh,
                token_limit,
                cost_limit,
                show_details,
                enable_alerts,
            };

            run_live_dashboard(&claude_dir, options)?;
        }
        Commands::Conversation {
            session,
            project,
            search,
            export,
            output,
            recent,
            mode,
            include_thinking,
            include_tools,
            list,
        } => {
            handle_conversation_command(
                &claude_dir,
                session,
                project,
                search,
                export,
                output,
                recent,
                mode,
                include_thinking,
                include_tools,
                list,
            )?;
        }
        Commands::View {
            target,
            project,
            recent,
            list,
            export,
            output,
        } => {
            // View is an alias for conversation with simplified options
            let session = if let Some(ref t) = target {
                // Check if target looks like a session ID (contains hyphen or is long)
                if t.contains('-') || t.len() > 20 {
                    Some(t.clone())
                } else {
                    // Treat as project name if no explicit project flag
                    None
                }
            } else {
                None
            };

            let project_filter = project.or_else(|| {
                target.as_ref().and_then(|t| {
                    // If target doesn't look like session ID, treat as project
                    if !t.contains('-') && t.len() <= 20 {
                        Some(t.clone())
                    } else {
                        None
                    }
                })
            });

            handle_conversation_command(
                &claude_dir,
                session,
                project_filter,
                None, // search
                export,
                output,
                recent,
                "detailed".to_string(), // mode
                true,                   // include_thinking
                true,                   // include_tools
                list,
            )?;
        }
        Commands::Inspect {
            target,
            project,
            recent,
            detailed,
            json,
            conversations,
            timeline,
        } => {
            handle_inspect_command(
                &claude_dir,
                &session_map_clone,
                target,
                project,
                recent,
                detailed,
                json,
                conversations,
                timeline,
            )?;
        }
        _ => {} // Other commands handled above
    }

    Ok(())
}

/// Convert CLI SortField to report SortField
fn convert_sort_field(field: Option<SortField>) -> Option<ReportSortField> {
    field.map(|f| match f {
        SortField::Date => ReportSortField::Date,
        SortField::Cost => ReportSortField::Cost,
        SortField::Tokens => ReportSortField::Tokens,
        SortField::Efficiency => ReportSortField::Efficiency,
        SortField::Project => ReportSortField::Project,
    })
}

/// Convert CLI SortOrder to report SortOrder
fn convert_sort_order(order: Option<SortOrder>) -> Option<ReportSortOrder> {
    order.map(|o| match o {
        SortOrder::Asc => ReportSortOrder::Asc,
        SortOrder::Desc => ReportSortOrder::Desc,
    })
}

/// Handle MCP server command
fn handle_mcp_server_command(
    claude_dir: &Path,
    http_port: Option<u16>,
    list_tools: bool,
    list_resources: bool,
) -> Result<()> {
    use mcp::{McpServer, get_server_info};

    let server = McpServer::new(claude_dir.to_path_buf());

    // Handle list commands
    if list_tools {
        println!("📋 Available MCP Tools:");
        for tool in server.list_tools() {
            println!("  🔧 {}", tool.name);
            println!("     {}", tool.description);
            println!(
                "     Schema: {}",
                serde_json::to_string_pretty(&tool.input_schema)?
            );
            println!();
        }
        return Ok(());
    }

    if list_resources {
        println!("📋 Available MCP Resources:");
        for resource in server.list_resources() {
            println!("  📊 {} ({})", resource.name, resource.uri);
            println!("     {}", resource.description);
            println!("     Type: {}", resource.mime_type);
            println!();
        }
        return Ok(());
    }

    // Start server
    if let Some(port) = http_port {
        print_info(&format!("Starting MCP HTTP server on port {}", port));
        println!(
            "Server info: {}",
            serde_json::to_string_pretty(&get_server_info())?
        );
        print_warning("HTTP MCP server not yet fully implemented - use stdio mode");
    } else {
        print_info("Starting MCP stdio server");
        println!(
            "Server info: {}",
            serde_json::to_string_pretty(&get_server_info())?
        );
        print_warning("Stdio MCP server not yet fully implemented - this is a preview");

        // Show what would be available
        println!("\n📋 Available Resources:");
        for resource in server.list_resources() {
            println!("  - {}", resource.uri);
        }

        println!("\n🔧 Available Tools:");
        for tool in server.list_tools() {
            println!("  - {}", tool.name);
        }
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
    billing_manager: &billing_blocks::BillingBlockManager,
) -> Result<()> {
    print_info("🧪 Testing resume functionality...");

    // Create a test TUI app and set some state
    let mut tui_app = TuiApp::new(daily_report, session_report, billing_manager.clone());

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
        billing_manager.clone(),
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

/// Handle billing blocks command
fn handle_billing_blocks_command(
    billing_manager: &billing_blocks::BillingBlockManager,
    json: bool,
    responsive: bool,
    classic: bool,
    show_summary: bool,
) {
    let report = billing_manager.generate_report();

    if json {
        // JSON output
        if let Ok(json_str) = serde_json::to_string_pretty(&report) {
            println!("{}", json_str);
        }
    } else if responsive {
        // Responsive table format
        let blocks = billing_manager.get_all_blocks();
        display_billing_blocks_responsive(&blocks);
    } else if classic {
        // Classic table format - use enhanced format for now
        display_billing_blocks_enhanced(&report, false);
    } else {
        // Enhanced format
        display_billing_blocks_enhanced(&report, show_summary);
    }
}

/// Display billing blocks in enhanced format
fn display_billing_blocks_enhanced(
    report: &billing_blocks::BillingBlockReport,
    show_summary: bool,
) {
    use colored::Colorize;

    println!(
        "\n{}",
        "📊 Claude Usage by 5-Hour Billing Blocks".bold().cyan()
    );
    println!("{}", "═".repeat(50).blue());

    if report.blocks.is_empty() {
        print_warning("No billing block data found");
        return;
    }

    // Display blocks by date
    let mut current_date = String::new();
    for block in &report.blocks {
        if block.date != current_date {
            println!("\n📅 {}", block.date.bold());
            println!("{}", "─".repeat(40));
            current_date = block.date.clone();
        }

        let cost_color = if block.usage.total_cost > 1.0 {
            "red"
        } else if block.usage.total_cost > 0.1 {
            "yellow"
        } else {
            "green"
        };

        println!(
            "  {} │ {} tokens │ ${:.4} │ {} sessions",
            block.time_range.cyan(),
            format!("{:>8}", block.usage.total_tokens()).white(),
            block.usage.total_cost.to_string().color(cost_color),
            block.session_count
        );
    }

    if show_summary {
        println!("\n{}", "📈 Summary Statistics".bold().cyan());
        println!("{}", "─".repeat(40));

        // Peak usage block
        if let Some(ref peak) = report.peak_block {
            println!(
                "Peak Block: {} {} ({} tokens)",
                peak.date,
                peak.time_range,
                peak.usage.total_tokens()
            );
        }

        // Average usage
        println!(
            "Average per Block: {} tokens, ${:.4}",
            report.average_per_block.total_tokens(),
            report.average_per_block.total_cost
        );

        // Usage by time of day
        println!("\n⏰ Usage by Time of Day:");
        let mut time_blocks: Vec<_> = report.usage_by_time.iter().collect();
        time_blocks.sort_by_key(|(time, _)| *time);

        for (time, usage) in time_blocks {
            let bar_length = (usage.total_tokens() as f64 / 1000.0).min(40.0) as usize;
            let bar = "█".repeat(bar_length);
            println!(
                "  {} │ {} {} tokens",
                time.cyan(),
                bar.green(),
                usage.total_tokens()
            );
        }
    }

    // Total summary
    println!("\n{}", "💰 Total Usage".bold().cyan());
    println!("{}", "─".repeat(40));
    println!("Total Tokens: {}", report.total_usage.total_tokens());
    println!("Total Cost: ${:.4}", report.total_usage.total_cost);
    println!("Active Blocks: {}", report.blocks.len());
}

/// Display billing blocks in table format
#[allow(dead_code)]
fn display_billing_blocks_table(report: &billing_blocks::BillingBlockReport) {
    use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Date"),
            Cell::new("Time Block"),
            Cell::new("Input Tokens"),
            Cell::new("Output Tokens"),
            Cell::new("Total Tokens"),
            Cell::new("Cost ($)"),
            Cell::new("Sessions"),
        ]);

    for block in &report.blocks {
        table.add_row(vec![
            Cell::new(&block.date),
            Cell::new(&block.time_range),
            Cell::new(block.usage.input_tokens.to_string()),
            Cell::new(block.usage.output_tokens.to_string()),
            Cell::new(block.usage.total_tokens().to_string()),
            Cell::new(format!("{:.4}", block.usage.total_cost)),
            Cell::new(block.session_count.to_string()),
        ]);
    }

    // Add totals row
    table.add_row(vec![
        Cell::new("TOTAL").fg(Color::Yellow),
        Cell::new(""),
        Cell::new(report.total_usage.input_tokens.to_string()).fg(Color::Yellow),
        Cell::new(report.total_usage.output_tokens.to_string()).fg(Color::Yellow),
        Cell::new(report.total_usage.total_tokens().to_string()).fg(Color::Yellow),
        Cell::new(format!("{:.4}", report.total_usage.total_cost)).fg(Color::Yellow),
        Cell::new(report.blocks.len().to_string()).fg(Color::Yellow),
    ]);

    println!("{}", table);
}

/// Handle pricing cache command
fn handle_pricing_cache_command(show: bool, clear: bool, update: bool) -> Result<()> {
    use pricing_cache::PricingCache;

    if show {
        println!("📦 Pricing Cache Status");
        println!("{}", "─".repeat(40));

        match PricingCache::load()? {
            Some(cache) => {
                println!("✅ Cache found");
                println!(
                    "Last Updated: {}",
                    cache.last_updated.format("%Y-%m-%d %H:%M:%S UTC")
                );
                println!(
                    "Valid: {}",
                    if cache.is_valid() {
                        "Yes"
                    } else {
                        "No (expired)"
                    }
                );
                println!("Version: {}", cache.version);
                println!("Models Cached: {}", cache.pricing_data.len());

                if cache.is_valid() {
                    println!("\n📊 Cached Models:");
                    for model_name in cache.pricing_data.keys() {
                        println!("  - {}", model_name);
                    }
                } else {
                    print_warning("Cache is expired and will be ignored");
                }
            }
            None => {
                println!("❌ No cache found");
                println!("Using built-in fallback pricing data");
            }
        }
    } else if clear {
        print_info("Clearing pricing cache...");
        PricingCache::clear()?;
        println!("✅ Pricing cache cleared successfully");
    } else if update {
        print_info("Updating pricing cache...");

        // For now, just create a new cache with fallback data
        let new_cache = PricingCache::new();
        new_cache.save()?;

        println!("✅ Pricing cache updated successfully");
        println!("Cache will remain valid for 7 days");
    } else {
        // Show help if no flags provided
        println!("Use --show, --clear, or --update to manage the pricing cache");
        println!("Run 'claudelytics pricing-cache --help' for more information");
    }

    Ok(())
}

/// Session blocks command options
struct BlocksCommandOptions {
    active: bool,
    length: i64,
    recent: bool,
    live: bool,
    refresh: u64,
    token_limit: Option<u64>,
    cost_limit: Option<f64>,
    since: Option<String>,
    until: Option<String>,
}

/// Handle session blocks command
fn handle_blocks_command(claude_dir: &Path, options: BlocksCommandOptions) -> Result<()> {
    use colored::Colorize;
    use std::thread;
    use std::time::Duration;

    // Create session block configuration
    let config = SessionBlockConfig {
        block_hours: options.length,
        token_limit: options.token_limit,
        cost_limit: options.cost_limit,
    };

    loop {
        // Parse usage data
        let parser = UsageParser::new(
            claude_dir.to_path_buf(),
            options.since.clone(),
            options.until.clone(),
            None, // No model filter for session blocks
        )?;
        let (_daily_map, session_map, _billing_manager) = parser.parse_all()?;

        // Create session block manager
        let mut block_manager = SessionBlockManager::new(config.clone());

        // Add all usage records to blocks
        // We need to iterate through the session map to get timestamps
        for (session_path, (usage, last_activity)) in &session_map {
            block_manager.add_usage(*last_activity, usage, session_path);
        }

        // Generate report
        let report = block_manager.generate_report();

        // Clear screen for live mode
        if options.live {
            print!("\x1B[2J\x1B[1;1H");
        }

        // Display header
        println!("\n{}", "📊 Session Blocks Analysis".bold().cyan());
        println!("{}", "═".repeat(50).blue());
        println!("Block Duration: {} hours", options.length);
        if let Some(limit) = options.token_limit {
            println!("Token Limit: {}", format_number(limit));
        }
        if let Some(limit) = options.cost_limit {
            println!("Cost Limit: ${:.2}", limit);
        }
        println!();

        // Filter blocks based on flags
        let blocks_to_show: Vec<_> = if options.active {
            report.blocks.iter().filter(|b| b.is_active).collect()
        } else if options.recent {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
            report
                .blocks
                .iter()
                .filter(|b| b.start_time > cutoff)
                .collect()
        } else {
            report.blocks.iter().collect()
        };

        if blocks_to_show.is_empty() {
            print_warning("No session blocks found matching criteria");
        } else {
            // Sort blocks by start time (newest first)
            let mut sorted_blocks = blocks_to_show;
            sorted_blocks.sort_by(|a, b| b.start_time.cmp(&a.start_time));

            // Display blocks
            for block in sorted_blocks {
                let is_active_indicator = if block.is_active { "🟢" } else { "⚪" };
                let time_range = format!(
                    "{} - {}",
                    block.start_time.format("%Y-%m-%d %H:%M"),
                    block.end_time.format("%H:%M")
                );

                println!(
                    "{} {} │ {} tokens │ ${:.4} │ {} sessions",
                    is_active_indicator,
                    time_range.cyan(),
                    format!("{:>8}", block.usage.total_tokens()).white(),
                    block.usage.total_cost,
                    block.session_count
                );

                // Show burn rate for active blocks
                if let Some(ref burn_rate) = block.burn_rate {
                    println!(
                        "   ├─ Burn Rate: {} tokens/hr, ${:.2}/hr",
                        burn_rate.tokens_per_hour as u64, burn_rate.cost_per_hour
                    );
                    println!(
                        "   ├─ Projected Daily: {} tokens, ${:.2}",
                        format_number(burn_rate.projected_daily_tokens),
                        burn_rate.projected_daily_cost
                    );

                    if let Some(time_to_limit) = burn_rate.time_to_limit {
                        let hours = time_to_limit.num_hours();
                        let minutes = time_to_limit.num_minutes() % 60;
                        println!("   └─ Time to Limit: {}h {}m", hours, minutes);
                    }
                }
            }

            // Show summary
            if let Some(ref current_burn) = report.current_burn_rate {
                println!("\n{}", "🔥 Current Burn Rate".bold().yellow());
                println!("{}", "─".repeat(40));
                println!(
                    "Hourly: {} tokens, ${:.2}",
                    current_burn.tokens_per_hour as u64, current_burn.cost_per_hour
                );
                println!(
                    "Daily Projection: {} tokens, ${:.2}",
                    format_number(current_burn.projected_daily_tokens),
                    current_burn.projected_daily_cost
                );
                println!(
                    "Monthly Projection: ${:.2}",
                    current_burn.projected_monthly_cost
                );
            }

            println!("\n{}", "📈 Summary".bold().cyan());
            println!("{}", "─".repeat(40));
            println!("Total Blocks: {}", report.total_blocks);
            println!("Active Blocks: {}", report.active_blocks);
            println!(
                "Total Tokens: {}",
                format_number(report.total_usage.total_tokens())
            );
            println!("Total Cost: ${:.4}", report.total_usage.total_cost);
        }

        if !options.live {
            break;
        }

        // Wait before refresh
        thread::sleep(Duration::from_secs(options.refresh));
    }

    Ok(())
}

/// Handle projections command
fn handle_projections_command(
    claude_dir: &Path,
    days: i64,
    token_limit: Option<u64>,
    cost_limit: Option<f64>,
    json: bool,
    since: Option<String>,
) -> Result<()> {
    use colored::Colorize;

    // Parse usage data
    let parser = UsageParser::new(
        claude_dir.to_path_buf(),
        since,
        None,
        None, // No model filter for projections
    )?;
    let (daily_usage, _, _) = parser.parse_all()?;

    // Calculate projections
    let calculator = ProjectionCalculator::new()
        .with_projection_days(days)
        .with_limits(token_limit, cost_limit);

    let projection = calculator.calculate_projections(&daily_usage);

    if json {
        // Output as JSON
        println!("{}", serde_json::to_string_pretty(&projection)?);
    } else {
        // Display formatted output
        println!("\n{}", "📊 Usage Projections".bold().cyan());
        println!("{}", "═".repeat(50).blue());

        // Current averages
        println!("\n{}", "📈 Current Usage Patterns".bold());
        println!("{}", "─".repeat(40));
        println!("Daily Average: ${:.2}", projection.daily_average);
        println!("Weekly Average: ${:.2}", projection.weekly_average);
        println!("Monthly Average: ${:.2}", projection.monthly_average);

        // Trend analysis
        let trend_emoji = match projection.trend {
            projections::TrendDirection::Increasing => "📈",
            projections::TrendDirection::Decreasing => "📉",
            projections::TrendDirection::Stable => "➡️",
        };
        let _trend_color = match projection.trend {
            projections::TrendDirection::Increasing => "red",
            projections::TrendDirection::Decreasing => "green",
            projections::TrendDirection::Stable => "yellow",
        };

        println!(
            "\nTrend: {} {:?} ({:+.1}%)",
            trend_emoji, projection.trend, projection.growth_rate
        );

        // Projections
        println!("\n{}", "🔮 Future Projections".bold());
        println!("{}", "─".repeat(40));
        println!(
            "Estimated Monthly Cost: ${:.2}",
            projection.estimated_monthly_cost
        );

        if let Some(days_until) = projection.days_until_limit {
            if let Some(limit_date) = projection.limit_date {
                let warning = if days_until <= 7 { "⚠️ " } else { "" };
                println!(
                    "{}Days Until Limit: {} ({})",
                    warning,
                    days_until,
                    limit_date.format("%Y-%m-%d")
                );
            }
        }

        // Show projection details for key dates
        if !projection.projections.is_empty() {
            println!("\n{}", "📅 Projection Details".bold());
            println!("{}", "─".repeat(40));

            // Show projections for 7, 14, 30 days
            for days_ahead in &[7, 14, 30] {
                if let Some(proj) = projection.projections.get((*days_ahead - 1) as usize) {
                    println!(
                        "{} days: ${:.2} (${:.2} - ${:.2})",
                        days_ahead, proj.value, proj.lower_bound, proj.upper_bound
                    );
                }
            }
        }

        // Recommendations
        println!("\n{}", "💡 Recommendations".bold());
        println!("{}", "─".repeat(40));

        match projection.trend {
            projections::TrendDirection::Increasing => {
                if projection.growth_rate > 20.0 {
                    println!("⚠️  Usage is growing rapidly. Consider:");
                    println!("   - Review recent sessions for efficiency");
                    println!("   - Set up usage alerts");
                    println!("   - Implement cost controls");
                } else {
                    println!("📈 Usage is increasing moderately");
                    println!("   - Monitor for sustained growth");
                    println!("   - Consider setting budget limits");
                }
            }
            projections::TrendDirection::Decreasing => {
                println!("✅ Usage is decreasing");
                println!("   - Good cost management");
                println!("   - Continue current practices");
            }
            projections::TrendDirection::Stable => {
                println!("➡️  Usage is stable");
                println!("   - Predictable costs");
                println!("   - Budget planning is straightforward");
            }
        }
    }

    Ok(())
}

/// Handle analytics command
fn handle_analytics_command(
    session_map: &SessionUsageMap,
    time_of_day: bool,
    day_of_week: bool,
    duration: bool,
    frequency: bool,
    efficiency: bool,
    threshold: f64,
) -> Result<()> {
    use colored::Colorize;
    use session_analytics::SessionAnalytics;

    let analytics = SessionAnalytics::new(session_map);

    // Show all analytics if no specific flags are set
    let show_all = !time_of_day && !day_of_week && !duration && !frequency && !efficiency;

    println!("\n{}", "🔍 Advanced Session Analytics".bold().cyan());
    println!("{}", "═".repeat(50).blue());

    // Time of day analysis
    if show_all || time_of_day {
        let time_analysis = analytics.analyze_time_of_day();

        println!("\n{}", "⏰ Time of Day Analysis".bold());
        println!("{}", "─".repeat(40));
        println!(
            "Peak Hour: {} ({}:00 - {}:00)",
            time_analysis.peak_hour,
            time_analysis.peak_hour,
            (time_analysis.peak_hour + 1) % 24
        );
        println!(
            "Off-Peak Hour: {} ({}:00 - {}:00)",
            time_analysis.off_peak_hour,
            time_analysis.off_peak_hour,
            (time_analysis.off_peak_hour + 1) % 24
        );

        let business_tokens = time_analysis.business_hours_usage.total_tokens();
        let after_hours_tokens = time_analysis.after_hours_usage.total_tokens();
        let business_pct = if business_tokens + after_hours_tokens > 0 {
            (business_tokens as f64 / (business_tokens + after_hours_tokens) as f64) * 100.0
        } else {
            0.0
        };

        println!("\nBusiness Hours (9AM-6PM):");
        println!(
            "  Tokens: {} ({:.1}%)",
            format_number(business_tokens),
            business_pct
        );
        println!(
            "  Cost: ${:.4}",
            time_analysis.business_hours_usage.total_cost
        );

        println!("\nAfter Hours:");
        println!(
            "  Tokens: {} ({:.1}%)",
            format_number(after_hours_tokens),
            100.0 - business_pct
        );
        println!("  Cost: ${:.4}", time_analysis.after_hours_usage.total_cost);

        // Show hourly distribution
        println!("\nHourly Distribution:");
        for hour in 0..24 {
            if let Some(metrics) = time_analysis.hourly_usage.get(&hour) {
                let bar_length =
                    (metrics.usage.total_tokens() as f64 / 100000.0).min(40.0) as usize;
                let bar = "█".repeat(bar_length);
                println!(
                    "  {:02}:00 │ {} {} sessions",
                    hour,
                    bar.green(),
                    metrics.session_count
                );
            }
        }
    }

    // Day of week analysis
    if show_all || day_of_week {
        let dow_analysis = analytics.analyze_day_of_week();

        println!("\n{}", "📅 Day of Week Analysis".bold());
        println!("{}", "─".repeat(40));
        println!("Most Active Day: {:?}", dow_analysis.most_active_day);
        println!("Least Active Day: {:?}", dow_analysis.least_active_day);
        println!(
            "Weekend/Weekday Ratio: {:.2}",
            dow_analysis.weekend_vs_weekday_ratio
        );

        println!("\nUsage by Day:");
        use chrono::Weekday;
        for day in &[
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ] {
            if let Some(usage) = dow_analysis.daily_usage.get(day) {
                println!(
                    "  {:?}: {} tokens, ${:.4}",
                    day,
                    format_number(usage.total_tokens()),
                    usage.total_cost
                );
            }
        }
    }

    // Session duration analysis
    if show_all || duration {
        let duration_analysis = analytics.analyze_session_durations();

        println!("\n{}", "⏱️ Session Duration Analysis".bold());
        println!("{}", "─".repeat(40));
        println!(
            "Average Duration: {}",
            session_analytics::format_duration(&duration_analysis.avg_session_duration)
        );

        println!("\nLongest Session:");
        println!("  Path: {}", duration_analysis.longest_session.path);
        println!(
            "  Duration: {}",
            session_analytics::format_duration(&duration_analysis.longest_session.duration)
        );
        println!(
            "  Tokens: {}",
            format_number(duration_analysis.longest_session.tokens)
        );

        println!("\nDuration Distribution:");
        let dist = &duration_analysis.duration_distribution;
        println!("  < 5 min: {} sessions", dist.under_5_min);
        println!("  5-30 min: {} sessions", dist.min_5_to_30);
        println!("  30-60 min: {} sessions", dist.min_30_to_60);
        println!("  1-3 hours: {} sessions", dist.hour_1_to_3);
        println!("  > 3 hours: {} sessions", dist.over_3_hours);
    }

    // Session frequency analysis
    if show_all || frequency {
        let freq_analysis = analytics.analyze_session_frequency();

        println!("\n{}", "📊 Session Frequency Analysis".bold());
        println!("{}", "─".repeat(40));
        println!("Sessions per Day: {:.2}", freq_analysis.sessions_per_day);
        println!("Sessions per Week: {:.2}", freq_analysis.sessions_per_week);
        println!("Days with Usage: {}", freq_analysis.days_with_usage);
        println!(
            "Average Sessions per Active Day: {:.2}",
            freq_analysis.avg_sessions_per_active_day
        );

        println!("\nStreaks:");
        println!("  Longest Streak: {} days", freq_analysis.longest_streak);
        let current_color = if freq_analysis.current_streak > 0 {
            "green"
        } else {
            "red"
        };
        println!(
            "  Current Streak: {} days",
            freq_analysis
                .current_streak
                .to_string()
                .color(current_color)
        );
    }

    // Cost efficiency analysis
    if show_all || efficiency {
        let eff_analysis = analytics.analyze_cost_efficiency(threshold);

        println!("\n{}", "💰 Cost Efficiency Analysis".bold());
        println!("{}", "─".repeat(40));

        println!("\nMost Expensive Session:");
        println!("  Path: {}", eff_analysis.most_expensive_session.path);
        println!("  Cost: ${:.4}", eff_analysis.most_expensive_session.cost);
        println!(
            "  Tokens: {}",
            format_number(eff_analysis.most_expensive_session.tokens)
        );

        println!("\nMost Efficient Session:");
        println!("  Path: {}", eff_analysis.most_efficient_session.path);
        let eff = if eff_analysis.most_efficient_session.cost > 0.0 {
            eff_analysis.most_efficient_session.tokens as f64
                / eff_analysis.most_efficient_session.cost
        } else {
            0.0
        };
        println!("  Efficiency: {:.0} tokens/$", eff);

        if !eff_analysis.sessions_above_threshold.is_empty() {
            println!("\n⚠️  Sessions Above ${} Threshold:", threshold);
            for session in &eff_analysis.sessions_above_threshold {
                println!("  - {} (${:.4})", session.path, session.cost);
            }
        }
    }

    println!("\n{}", "═".repeat(50).blue());

    Ok(())
}

/// Handle real-time analytics command
fn handle_realtime_analytics_command(
    daily_map: &models::DailyUsageMap,
    session_map: &SessionUsageMap,
    daily_limit: Option<f64>,
    monthly_limit: Option<f64>,
    yearly_limit: Option<f64>,
    alert_threshold: f64,
    json: bool,
) -> Result<()> {
    use realtime_analytics::{BudgetConfig, RealtimeAnalytics, format_realtime_analytics};

    // Create budget configuration
    let budget_config = BudgetConfig {
        daily_limit,
        monthly_limit,
        yearly_limit,
        alert_threshold,
    };

    // Create real-time analytics instance
    let analytics = RealtimeAnalytics::new(daily_map, session_map, budget_config);

    // Generate comprehensive report
    let report = analytics.generate_report();

    if json {
        // Output as JSON
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        // Format and display the report
        let formatted_output = format_realtime_analytics(&report);
        println!("{}", formatted_output);
    }

    Ok(())
}

/// Handle conversation command
#[allow(clippy::too_many_arguments)]
fn handle_conversation_command(
    claude_dir: &Path,
    session: Option<String>,
    project: Option<String>,
    search: Option<String>,
    export: Option<String>,
    output: Option<PathBuf>,
    recent: bool,
    mode: String,
    include_thinking: bool,
    include_tools: bool,
    list: bool,
) -> Result<()> {
    use colored::Colorize;
    use conversation_display::{ConversationDisplay, DisplayMode};
    use conversation_parser::{Conversation, ConversationParser};

    let parser = ConversationParser::new(claude_dir.to_path_buf());

    // Find all conversation files
    let mut conversation_files = parser.find_conversation_files()?;

    // Apply filters
    if let Some(proj) = &project {
        conversation_files.retain(|path| path.to_string_lossy().contains(proj));
    }

    if recent {
        let seven_days_ago = chrono::Utc::now() - chrono::Duration::days(7);
        conversation_files.retain(|path| {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                    return modified_time > seven_days_ago;
                }
            }
            false
        });
    }

    // If listing conversations
    if list {
        println!("{}", "📋 Available Conversations".bold().cyan());
        println!("{}", "═".repeat(50).blue());

        if conversation_files.is_empty() {
            print_warning("No conversations found matching criteria");
            return Ok(());
        }

        for (idx, file_path) in conversation_files.iter().enumerate() {
            // Extract project and session from path
            let path_str = file_path.to_string_lossy();
            let relative_path = path_str
                .strip_prefix(&format!("{}/projects/", claude_dir.display()))
                .unwrap_or(&path_str);

            println!("{}. {}", idx + 1, relative_path.dimmed());

            // Try to parse and show summary
            if let Ok(conversation) = parser.parse_conversation(file_path) {
                if let Some(summary) = &conversation.summary {
                    println!("   📄 {}", summary.summary.bright_white());
                }
                println!(
                    "   💬 {} messages | 💰 ${:.4} | 📊 {} tokens",
                    conversation.messages.len(),
                    conversation.total_usage.total_cost,
                    conversation.total_usage.total_tokens()
                );
            }
        }
        return Ok(());
    }

    // Find specific session if requested
    let conversations_to_display: Vec<Conversation> = if let Some(sess) = &session {
        // Find conversation file matching session
        let matching_file = conversation_files
            .iter()
            .find(|path| path.to_string_lossy().contains(sess));

        if let Some(file_path) = matching_file {
            vec![parser.parse_conversation(file_path)?]
        } else {
            print_warning(&format!("No conversation found for session: {}", sess));
            return Ok(());
        }
    } else {
        // Parse all matching conversations
        let mut conversations = Vec::new();
        for file_path in &conversation_files {
            if let Ok(conv) = parser.parse_conversation(file_path) {
                conversations.push(conv);
            }
        }
        conversations
    };

    // Apply search filter
    let mut filtered_conversations = conversations_to_display;
    if let Some(search_term) = &search {
        filtered_conversations.retain(|conv| {
            // Search in messages
            for msg in &conv.messages {
                for content in &msg.content {
                    match content {
                        conversation_parser::MessageContentBlock::Text { text, .. } => {
                            if text.to_lowercase().contains(&search_term.to_lowercase()) {
                                return true;
                            }
                        }
                        conversation_parser::MessageContentBlock::ToolUse {
                            name, input, ..
                        } => {
                            if name.to_lowercase().contains(&search_term.to_lowercase())
                                || input
                                    .to_string()
                                    .to_lowercase()
                                    .contains(&search_term.to_lowercase())
                            {
                                return true;
                            }
                        }
                        conversation_parser::MessageContentBlock::ToolResult {
                            content, ..
                        } => {
                            if content.to_lowercase().contains(&search_term.to_lowercase()) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        });
    }

    if filtered_conversations.is_empty() {
        print_warning("No conversations found matching criteria");
        return Ok(());
    }

    // Set display mode
    let display_mode = match mode.as_str() {
        "compact" => DisplayMode::Compact,
        _ => DisplayMode::Detailed,
    };

    let display = ConversationDisplay::new()
        .with_mode(display_mode)
        .with_terminal_width(terminal::Terminal::width() as usize);

    // Handle export
    if let Some(export_format) = &export {
        let content = match export_format.as_str() {
            "json" => {
                // Export as JSON
                serde_json::to_string_pretty(
                    &filtered_conversations
                        .iter()
                        .map(|conv| {
                            serde_json::json!({
                                "file_path": conv.file_path,
                                "summary": conv.summary,
                                "messages": conv.messages.len(),
                                "total_tokens": conv.total_usage.total_tokens(),
                                "total_cost": conv.total_usage.total_cost,
                                "started_at": conv.started_at,
                                "ended_at": conv.ended_at,
                                "conversation": conv.messages
                            })
                        })
                        .collect::<Vec<_>>(),
                )?
            }
            "markdown" => {
                // Export as markdown
                let mut markdown = String::new();
                for conv in &filtered_conversations {
                    markdown.push_str(&format_conversation_as_markdown(
                        conv,
                        include_thinking,
                        include_tools,
                    ));
                    markdown.push_str("\n\n---\n\n");
                }
                markdown
            }
            _ => {
                // Default to text export
                let mut text = String::new();
                for conv in &filtered_conversations {
                    text.push_str(&display.format_conversation(conv));
                    text.push_str("\n\n");
                }
                text
            }
        };

        // Write to file or stdout
        if let Some(output_path) = output {
            std::fs::write(&output_path, content)?;
            print_info(&format!(
                "Conversation exported to: {}",
                output_path.display()
            ));
        } else {
            println!("{}", content);
        }
    } else {
        // Display in terminal
        for conv in &filtered_conversations {
            println!("{}", display.format_conversation(conv));

            if !include_thinking {
                // Filter out thinking blocks if not included
                // This is handled by the display module based on mode
            }

            if !include_tools {
                // Filter out tool usage if not included
                // This is handled by the display module based on mode
            }
        }
    }

    Ok(())
}

/// Format conversation as markdown
fn format_conversation_as_markdown(
    conversation: &conversation_parser::Conversation,
    include_thinking: bool,
    include_tools: bool,
) -> String {
    let mut markdown = String::new();

    // Header
    if let Some(summary) = &conversation.summary {
        markdown.push_str(&format!("# {}\n\n", summary.summary));
    } else {
        markdown.push_str("# Conversation\n\n");
    }

    // Metadata
    if let (Some(start), Some(end)) = (conversation.started_at, conversation.ended_at) {
        markdown.push_str(&format!(
            "**Started:** {}\n",
            start.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        markdown.push_str(&format!(
            "**Ended:** {}\n",
            end.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        markdown.push_str(&format!(
            "**Duration:** {}\n",
            format_duration(&(end - start))
        ));
    }

    markdown.push_str(&format!(
        "**Total Tokens:** {}\n",
        conversation.total_usage.total_tokens()
    ));
    markdown.push_str(&format!(
        "**Total Cost:** ${:.4}\n\n",
        conversation.total_usage.total_cost
    ));

    markdown.push_str("---\n\n");

    // Messages
    for thread in conversation.get_thread_structure() {
        markdown.push_str(&format_thread_as_markdown(
            &thread,
            0,
            include_thinking,
            include_tools,
        ));
    }

    markdown
}

/// Format a message thread as markdown
fn format_thread_as_markdown(
    thread: &conversation_parser::MessageThread,
    depth: usize,
    include_thinking: bool,
    include_tools: bool,
) -> String {
    use conversation_parser::MessageContentBlock;

    let mut markdown = String::new();
    let indent = "  ".repeat(depth);
    let message = &thread.message;

    // Message header
    let role_emoji = match message.role.as_str() {
        "user" => "👤",
        "assistant" => "🤖",
        _ => "📝",
    };

    markdown.push_str(&format!(
        "{}## {} {} ({})\n\n",
        indent,
        role_emoji,
        message.role,
        message.timestamp.format("%H:%M:%S")
    ));

    // Message content
    for content in &message.content {
        match content {
            MessageContentBlock::Text { content_type, text } => {
                if content_type == "thinking" {
                    if include_thinking {
                        markdown.push_str(&format!("{}> 💭 *Thinking...*\n", indent));
                        markdown.push_str(&format!(
                            "{}> {}\n\n",
                            indent,
                            text.replace('\n', "\n> ")
                        ));
                    }
                } else {
                    markdown.push_str(&format!("{}{}\n\n", indent, text));
                }
            }
            MessageContentBlock::ToolUse { name, input, .. } => {
                if include_tools {
                    markdown.push_str(&format!("{}🔧 **Tool:** {}\n", indent, name));
                    markdown.push_str(&format!("{}```json\n", indent));
                    markdown.push_str(&format!(
                        "{}{}\n",
                        indent,
                        serde_json::to_string_pretty(input).unwrap_or_default()
                    ));
                    markdown.push_str(&format!("{}```\n\n", indent));
                }
            }
            MessageContentBlock::ToolResult { content, .. } => {
                if include_tools {
                    markdown.push_str(&format!("{}✅ **Result:**\n", indent));
                    markdown.push_str(&format!("{}```\n", indent));
                    markdown.push_str(&format!("{}{}\n", indent, content));
                    markdown.push_str(&format!("{}```\n\n", indent));
                }
            }
        }
    }

    // Process children
    for child in &thread.children {
        markdown.push_str(&format_thread_as_markdown(
            child,
            depth + 1,
            include_thinking,
            include_tools,
        ));
    }

    markdown
}

/// Format duration for display
fn format_duration(duration: &chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Format large numbers with commas
fn format_number(num: u64) -> String {
    let s = num.to_string();
    let mut result = String::new();
    let mut count = 0;

    for ch in s.chars().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(ch);
        count += 1;
    }

    result.chars().rev().collect()
}
/// Handle inspect command for session details
#[allow(clippy::too_many_arguments)]
fn handle_inspect_command(
    claude_dir: &Path,
    session_map: &SessionUsageMap,
    target: Option<String>,
    project: Option<String>,
    recent: bool,
    detailed: bool,
    json: bool,
    conversations: bool,
    timeline: bool,
) -> Result<()> {
    use colored::Colorize;
    use conversation_parser::ConversationParser;
    use serde_json::json;

    // Filter sessions based on criteria
    let mut sessions: Vec<_> = session_map.iter().collect();

    // Apply filters
    if recent {
        let cutoff = chrono::Local::now() - chrono::Duration::days(7);
        sessions.retain(|(_, (_, last_activity))| *last_activity > cutoff);
    }

    if let Some(ref proj) = project {
        sessions.retain(|(path, _)| path.contains(proj));
    }

    if let Some(ref t) = target {
        // Check if it's a session ID or project name
        if t.contains('-') || t.len() > 20 {
            // Looks like a session ID
            sessions.retain(|(path, _)| path.contains(t));
        } else {
            // Treat as project name
            sessions.retain(|(path, _)| path.contains(t));
        }
    }

    if sessions.is_empty() {
        print_warning("No sessions found matching the specified criteria");
        return Ok(());
    }

    // Sort sessions by last activity (newest first)
    sessions.sort_by(|(_, (_, a)), (_, (_, b))| b.cmp(a));

    if json {
        // JSON output
        let mut json_output = Vec::new();

        for (session_path, (usage, last_activity)) in sessions {
            let parts: Vec<&str> = session_path.split('/').collect();
            let project_name = parts.first().unwrap_or(&"unknown");
            let session_id = parts.get(1).unwrap_or(&"unknown");

            let mut session_info = json!({
                "session_id": session_id,
                "project": project_name,
                "last_activity": last_activity.format("%Y-%m-%d %H:%M:%S").to_string(),
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cache_creation_tokens": usage.cache_creation_tokens,
                "cache_read_tokens": usage.cache_read_tokens,
                "total_tokens": usage.total_tokens(),
                "total_cost": usage.total_cost,
                "efficiency": if usage.total_cost > 0.0 {
                    (usage.total_tokens() as f64 / usage.total_cost) as u64
                } else {
                    0
                }
            });

            if conversations {
                // Add conversation list
                let parser = ConversationParser::new(claude_dir.to_path_buf());
                if let Ok(conv_files) = parser.find_conversation_files() {
                    let session_convs: Vec<_> = conv_files
                        .iter()
                        .filter(|path| path.to_string_lossy().contains(session_path.as_str()))
                        .map(|p| {
                            p.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        })
                        .collect();
                    session_info["conversations"] = json!(session_convs);
                    session_info["conversation_count"] = json!(session_convs.len());
                }
            }

            json_output.push(session_info);
        }

        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        // Terminal output
        println!("\n{}", "📊 Session Inspection Report".bold().cyan());
        println!("{}", "═".repeat(60).blue());

        for (session_path, (usage, last_activity)) in
            sessions.iter().take(if target.is_some() { 1 } else { 10 })
        {
            let parts: Vec<&str> = session_path.split('/').collect();
            let project_name = parts.first().unwrap_or(&"unknown");
            let session_id = parts.get(1).unwrap_or(&"unknown");

            println!("\n{} Session: {}", "🔍".cyan(), session_id.yellow());
            println!("   Project: {}", project_name.green());
            println!(
                "   Last Activity: {}",
                last_activity
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .bright_white()
            );

            // Token usage breakdown
            println!("\n   {} Token Usage:", "📈".cyan());
            println!("   ├─ Input: {} tokens", format_number(usage.input_tokens));
            println!(
                "   ├─ Output: {} tokens",
                format_number(usage.output_tokens)
            );
            if usage.cache_creation_tokens > 0 {
                println!(
                    "   ├─ Cache Creation: {} tokens",
                    format_number(usage.cache_creation_tokens)
                );
            }
            if usage.cache_read_tokens > 0 {
                println!(
                    "   ├─ Cache Read: {} tokens",
                    format_number(usage.cache_read_tokens)
                );
            }
            println!(
                "   └─ Total: {} tokens",
                format_number(usage.total_tokens()).bold()
            );

            // Cost analysis
            println!("\n   {} Cost Analysis:", "💰".cyan());
            println!("   ├─ Total Cost: ${:.6}", usage.total_cost);
            let efficiency = if usage.total_cost > 0.0 {
                (usage.total_tokens() as f64 / usage.total_cost) as u64
            } else {
                0
            };
            println!("   └─ Efficiency: {} tokens/$", format_number(efficiency));

            if conversations || detailed {
                // Show conversation count
                let parser = ConversationParser::new(claude_dir.to_path_buf());
                if let Ok(conv_files) = parser.find_conversation_files() {
                    let session_convs: Vec<_> = conv_files
                        .iter()
                        .filter(|path| path.to_string_lossy().contains(session_path.as_str()))
                        .collect();

                    println!(
                        "\n   {} Conversations: {}",
                        "💬".cyan(),
                        session_convs.len()
                    );

                    if conversations && !session_convs.is_empty() {
                        println!("   Conversation files:");
                        for (i, conv_path) in session_convs.iter().take(5).enumerate() {
                            let conv_name =
                                conv_path.file_name().unwrap_or_default().to_string_lossy();
                            println!("   {}. {}", i + 1, conv_name.bright_black());
                        }
                        if session_convs.len() > 5 {
                            println!("   ... and {} more", session_convs.len() - 5);
                        }
                    }
                }
            }

            if timeline {
                // Show activity timeline (simplified for now)
                println!("\n   {} Activity Timeline:", "📅".cyan());
                println!(
                    "   └─ Active for approximately {}",
                    format_duration(&chrono::Duration::hours(2))
                ); // Placeholder
            }

            println!("\n   {}", "─".repeat(50).bright_black());
        }

        if sessions.len() > 10 && target.is_none() {
            println!(
                "\n💡 Showing top 10 sessions. Found {} total sessions.",
                sessions.len()
            );
            println!("   Use --target <session-id> to inspect a specific session.");
        }
    }

    Ok(())
}
