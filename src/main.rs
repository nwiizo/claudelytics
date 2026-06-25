//! Claudelytics - Claude Code and Codex Usage Analytics Tool
//!
//! A fast CLI tool for analyzing AI coding usage patterns, token consumption, and costs.
//! Parses Claude Code and Codex JSONL files and generates comprehensive reports.

// Module declarations
mod billing_blocks;
mod burn_rate;
mod cache_analysis;
mod claude_sessions;
mod codex_usage;
mod config;
mod config_v2;
mod conversation_display;
mod conversation_parser;
mod display;
mod domain;
mod error;
mod export;
mod helpers;
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

// Core dependencies
use anyhow::Result;
use chrono::{Local, Timelike};
use clap::{Parser, Subcommand, ValueEnum};
use config::Config;
use display::{
    display_billing_blocks_responsive, display_daily_report_enhanced,
    display_daily_report_responsive, display_daily_report_table, display_monthly_report_enhanced,
    display_monthly_report_table, display_report_json, display_session_report_enhanced,
    display_session_report_responsive, display_session_report_table, print_error, print_info,
    print_warning,
};
use export::{export_daily_to_csv, export_sessions_to_csv, export_summary_to_csv};
use models::{ModelReport, SessionUsageMap, TokenUsageTotals};
use parser::UsageParser;
use projections::ProjectionCalculator;
use reports::{
    SortField as ReportSortField, SortOrder as ReportSortOrder, generate_daily_report_sorted,
    generate_model_report_from_sources, generate_monthly_report_sorted,
    generate_session_report_sorted,
};
use session_blocks::{SessionBlockConfig, SessionBlockManager};
use state::{TuiMode, TuiSessionState};
use std::path::{Path, PathBuf};
use tui::TuiApp;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CliCostMode {
    /// Use costUSD from JSONL if available, otherwise calculate from tokens
    Auto,
    /// Always recalculate cost from token counts and model pricing
    Calculate,
    /// Only use costUSD field from JSONL data
    Display,
}

impl From<CliCostMode> for parser::CostMode {
    fn from(mode: CliCostMode) -> Self {
        match mode {
            CliCostMode::Auto => parser::CostMode::Auto,
            CliCostMode::Calculate => parser::CostMode::Calculate,
            CliCostMode::Display => parser::CostMode::Display,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortOrder {
    /// Sort in ascending order
    Asc,
    /// Sort in descending order
    Desc,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CacheSortField {
    /// Sort by write cost (default)
    WriteCost,
    /// Sort by cache hit rate
    HitRate,
    /// Sort by 5m TTL miss tokens
    #[value(name = "miss-5m")]
    Miss5m,
    /// Sort by 60m TTL miss tokens
    #[value(name = "miss-60m")]
    Miss60m,
    /// Sort by cold start tokens
    ColdStart,
    /// Sort by normal churn tokens
    NormalChurn,
    /// Sort by break-even turn (ascending)
    BreakevenTurn,
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
#[command(about = "Claude Code and Codex analytics - Audit token usage, costs, and sessions")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    long_about = "Claudelytics audits Claude Code and Codex usage and costs from their local JSONL logs.

EXAMPLES:
  claudelytics                    # Today's usage (compact table)
  claudelytics --today --json     # Today's usage as JSON
  claudelytics daily --since 20260101  # Daily report from date
  claudelytics weekly             # Weekly usage summary
  claudelytics session            # Usage by session
  claudelytics monthly            # Monthly aggregation
  claudelytics billing-blocks     # 5-hour billing blocks
  claudelytics tui                # Interactive terminal UI
  claudelytics -v daily           # Verbose with burn rates

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
        long_help = "Filter usage data from this date onwards. Formats: YYYYMMDD or YYYY-MM-DD\nExample: --since 20240101 or --since 2024-01-01\nCombine with --until for date ranges: --since 2024-01-01 --until 2024-01-31"
    )]
    since: Option<String>,

    #[arg(
        short,
        long,
        value_name = "DATE",
        help = "Filter until date (YYYYMMDD)",
        long_help = "Filter usage data up to this date. Formats: YYYYMMDD or YYYY-MM-DD\nExample: --until 20241231 or --until 2024-12-31\nUse alone for 'up to date' or combine with --since for ranges"
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

    #[arg(long, help = "Show last 7 days")]
    last_7d: bool,

    #[arg(long, help = "Show last 30 days")]
    last_30d: bool,

    #[arg(
        long,
        help = "Use classic table format",
        long_help = "Use classic table format instead of enhanced cards\nPrimary interface: Traditional ASCII tables\nDefault: Enhanced format with visual cards and summaries",
        hide = true
    )]
    classic: bool,

    #[arg(
        long,
        help = "Force compact display mode",
        long_help = "Force compact display mode for narrow terminals\nReduces columns shown in tables to fit smaller screens\nAutomatic: Adapts based on terminal width\nWith --compact: Always use minimal columns",
        hide = true
    )]
    compact: bool,

    #[arg(
        long,
        help = "Launch terminal user interface",
        long_help = "Launch interactive terminal user interface (TUI)\nFeatures: Navigation, search, charts, multiple tabs\nKeyboard shortcuts: j/k navigation, q to quit, ? for help",
        hide = true
    )]
    tui: bool,

    #[arg(
        long,
        value_name = "MODEL",
        help = "Filter by model or family (fable, opus, sonnet, haiku, openai)",
        long_help = "Filter usage data by model family, alias, or full model name\nExamples:\n  --model-filter fable    # Show only Fable usage\n  --model-filter openai   # Show only Codex/OpenAI usage\n  --model-filter gpt-5.6  # GPT-5.6 Sol (provider default)\n  --model-filter claude-fable-5  # Specific model"
    )]
    model_filter: Option<String>,

    #[arg(
        long,
        value_name = "PROJECT",
        help = "Filter Claude Code usage by project path substring",
        long_help = "Filter usage data to a specific Claude Code project or path substring.\nMatches the encoded project directory and full session path.\nExample: claudelytics --project myproject daily"
    )]
    project: Option<String>,

    #[arg(
        long,
        value_name = "TZ",
        default_value = "local",
        help = "Timezone for date grouping (IANA name, e.g. UTC or Asia/Tokyo)",
        long_help = "Timezone used when grouping timestamps into daily, weekly, and monthly reports.\nUse IANA names such as UTC, Asia/Tokyo, or America/Los_Angeles.\nDefault: local, using TZ when set and UTC as a portable fallback."
    )]
    timezone: String,

    #[arg(
        long,
        help = "List all known models and exit",
        long_help = "Display all registered Claude and OpenAI models with families and aliases\nUseful for seeing what models are available for filtering"
    )]
    list_models: bool,

    #[arg(
        long,
        help = "Show usage breakdown by model family",
        long_help = "Display usage by source and model family (Fable, Opus, Sonnet, Haiku, OpenAI)\nShows cost and token usage for each model type in a single view\nCombines with other filters: --today, --since, --until",
        hide = true
    )]
    by_model: bool,

    #[arg(
        long,
        help = "Show usage breakdown by model family",
        long_help = "Alias for --by-model, matching ccusage's --breakdown option."
    )]
    breakdown: bool,

    #[arg(
        long,
        help = "Use responsive table layout",
        long_help = "Use responsive tables that automatically adjust to terminal width\nFeatures: Auto-adjusting columns, smart column hiding, abbreviated headers\nPriorities: Date/Cost always shown, cache tokens hidden first\nCombines with daily, session, and billing-blocks commands",
        hide = true
    )]
    responsive: bool,
    #[arg(
        long,
        help = "Show real-time analytics",
        long_help = "Display real-time analytics alongside regular reports\nAdds burn rates, budget projections, and efficiency metrics\nWorks with daily and session commands\nExample: claudelytics daily --realtime",
        hide = true
    )]
    realtime: bool,

    #[arg(
        long,
        value_enum,
        default_value = "auto",
        help = "Cost calculation mode",
        long_help = "Control how costs are calculated:\n  auto: Use costUSD from JSONL if present, otherwise calculate\n  calculate: Always recalculate from token counts and pricing\n  display: Only show costUSD field from JSONL data"
    )]
    cost_mode: CliCostMode,

    #[arg(
        short,
        long,
        help = "Show detailed output with burn rate and trends",
        long_help = "Show detailed enhanced output with burn rate analysis,\nactivity trends, and efficiency metrics.\nDefault output is a compact table."
    )]
    verbose: bool,
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
    #[command(about = "Launch terminal user interface")]
    #[command(
        long_about = "Launch interactive terminal user interface\n\nFull-featured TUI for auditing Claude Code and Codex usage in the loaded scope.\n\nVIEWS:\n  - Overview, Daily, Weekly, Models, Sessions, Cache, Billing, Help\n  - Source-aware token, cost, model, and cache accounting\n  - Responsive tables for narrow and wide terminals\n\nKEYBOARD SHORTCUTS:\n  1-7/h: Select view  Tab/Shift+Tab: Cycle views\n  j/k or arrows: Navigate  /: Search  s: Sort  e: Export\n  q/Esc: Quit\n\nSet the loaded time range before launch with --today, --since, or --until.\n\nEXAMPLE:\n  claudelytics tui                      # Launch TUI"
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
    #[command(about = "Show usage aggregated by weeks")]
    #[command(
        long_about = "Show usage aggregated by weeks\n\nDisplays weekly summaries with total usage, active days,\nand average daily costs. Configurable week start day.\n\nEXAMPLES:\n  claudelytics weekly                   # Weekly report (weeks start Monday)\n  claudelytics weekly --start-of-week sunday  # Weeks start Sunday\n  claudelytics --json weekly            # JSON output"
    )]
    Weekly {
        #[arg(long, help = "Use classic table format")]
        classic: bool,
        #[arg(long, help = "Sort field")]
        sort_by: Option<SortField>,
        #[arg(long, help = "Sort order")]
        sort_order: Option<SortOrder>,
        #[arg(
            long,
            default_value = "monday",
            help = "Start of week (monday or sunday)"
        )]
        start_of_week: String,
    },
    #[command(about = "Manage configuration")]
    #[command(
        long_about = "Manage Claudelytics configuration settings\n\nConfiguration is stored in YAML format and persists between runs.\nUse --show to view current settings or modify specific options.\n\nCONFIG LOCATION:\n  ~/.config/claudelytics/config.yaml (or platform equivalent)\n\nAVAILABLE SETTINGS:\n  - Claude directory path\n  - Default output format (enhanced/classic/json)\n  - Default command\n  - Export directory\n  - Date format preferences\n\nEXAMPLES:\n  claudelytics config --show            # View current configuration\n  claudelytics config --set-path ~/claude # Set custom Claude directory\n  claudelytics config --reset           # Reset to defaults"
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
    #[command(about = "Show cost summary", hide = true)]
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
    #[command(about = "Show compact status line for shell hooks")]
    #[command(
        long_about = "Display a compact one-line usage summary suitable for Claude Code status hooks.\n\nEXAMPLES:\n  claudelytics statusline\n  claudelytics --json statusline"
    )]
    Statusline,
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
    #[command(about = "Analyze cache behavior and TTL efficiency")]
    #[command(
        long_about = "Analyze per-turn cache behavior across sessions\n\nBreaks down cache writes by TTL bucket (5m vs 60m expiry), identifies\ncold start costs, and computes the cache balance point per session.\n\nEXAMPLES:\n  claudelytics cache                    # Cache analysis summary\n  claudelytics cache --top 20           # Show top 20 sessions\n  claudelytics --today cache            # Today only (global flag)\n  claudelytics --json cache             # JSON output (global flag)\n  claudelytics --project myproj cache   # Filter by project (global flag)"
    )]
    Cache {
        #[arg(
            long,
            default_value = "10",
            help = "Number of sessions to show in detail"
        )]
        top: usize,

        #[arg(long, default_value = "10", help = "Number of projects to show")]
        top_projects: usize,

        #[arg(long, help = "Filter by project path substring")]
        project: Option<String>,

        #[arg(long, default_value = "write-cost", help = "Sort sessions by field")]
        sort: CacheSortField,

        #[arg(long, help = "Sort ascending")]
        asc: bool,

        #[arg(long, help = "Sort descending")]
        desc: bool,

        #[arg(
            long,
            default_value = "0.92",
            value_name = "RATIO",
            help = "Cache hit rate threshold for warmup detection (0.0-1.0)"
        )]
        threshold: f64,

        #[arg(
            long,
            value_name = "PCT",
            help = "Min cache hit rate % to show (e.g. 90)"
        )]
        min_hit: Option<f64>,

        #[arg(long, value_name = "PCT", help = "Min churn rate % to show (e.g. 50)")]
        min_churn: Option<f64>,
    },
    #[command(about = "Start Model Context Protocol (MCP) server", hide = true)]
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
    #[command(about = "Debug resume state", hide = true)]
    #[command(long_about = "Debug command to show TUI session state information")]
    DebugState,
    #[command(about = "Test resume functionality", hide = true)]
    #[command(long_about = "Test command to verify resume functionality without starting TUI")]
    TestResume,
    #[command(about = "Manage offline pricing cache", hide = true)]
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
    #[command(about = "Show session blocks (configurable time windows)", hide = true)]
    #[command(
        long_about = "Analyze usage in configurable session blocks\n\nSession blocks provide flexible time-based analysis similar to billing blocks\nbut with customizable durations. Default is 8-hour blocks.\n\nFor real-time monitoring, use `claudelytics live`.\n\nFEATURES:\n  - Configurable block duration (default: 8 hours)\n  - Active session tracking with burn rate\n  - Usage projections based on current activity\n  - Time to limit calculations\n\nEXAMPLES:\n  claudelytics blocks                  # Show all session blocks\n  claudelytics blocks --active         # Show only active sessions\n  claudelytics blocks --length 4       # Use 4-hour blocks\n  claudelytics blocks --recent         # Show last 30 days"
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
    #[command(about = "Show usage projections and forecasts", hide = true)]
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
    #[command(about = "Advanced session analytics", hide = true)]
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
    #[command(
        about = "Real-time analytics with burn rates and projections",
        hide = true
    )]
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
    #[command(about = "Live dashboard for real-time monitoring", hide = true)]
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
    #[command(
        about = "View conversation content (alias for conversation)",
        hide = true
    )]
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
    #[command(about = "Show today's budget pacing and burn rate")]
    #[command(
        long_about = "Show daily budget pacing metrics for today\n\nComputes today's spend, burn rate per hour, and hours remaining\nbefore the budget is exhausted.\n\nEXAMPLES:\n  claudelytics budget-status --budget 5.0          # Human-readable pacing\n  claudelytics --json budget-status --budget 5.0   # JSON output"
    )]
    BudgetStatus {
        #[arg(
            long,
            help = "Daily budget in USD",
            long_help = "Daily budget in USD to pace against\nExample: --budget 5.0"
        )]
        budget: f64,
    },
    #[command(about = "Inspect session details and metadata", hide = true)]
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
    #[command(about = "Show cache efficiency metrics for a single session")]
    #[command(
        long_about = "Show cache efficiency metrics for a single session\n\nOutputs cold%, hit%, windowed churn rate (last 5 turns), and turn count.\n\nEXAMPLES:\n  claudelytics cache-stats --session-id <uuid>        # Human-readable output\n  claudelytics --json cache-stats --session-id <uuid> # JSON output"
    )]
    CacheStats {
        #[arg(long, help = "Session ID (JSONL filename stem)")]
        session_id: String,
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

    // Get Claude directory paths (supports both legacy ~/.claude and XDG ~/.config/claude)
    let (claude_dir, claude_dirs) = if let Some(path) = cli.path {
        (path.clone(), vec![path])
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let legacy = PathBuf::from(&home).join(".claude");
        let xdg = PathBuf::from(&home).join(".config").join("claude");

        let mut dirs = Vec::new();
        if legacy.exists() {
            dirs.push(legacy.clone());
        }
        if xdg.exists() {
            dirs.push(xdg.clone());
        }

        // Primary dir: prefer config, then legacy, then XDG
        let primary = config.get_claude_path().unwrap_or_else(|_| {
            if legacy.exists() {
                legacy.clone()
            } else if xdg.exists() {
                xdg
            } else {
                legacy // default for error message
            }
        });

        if dirs.is_empty() {
            dirs.push(primary.clone());
        }

        (primary, dirs)
    };

    // Handle date shortcut flags: today > last_7d > last_30d > explicit
    let report_timezone = parser::parse_timezone(Some(&cli.timezone))?;
    let report_today = chrono::Utc::now()
        .with_timezone(&report_timezone)
        .date_naive();
    let (since_date, until_date) = if cli.today {
        let today = report_today.format("%Y%m%d").to_string();
        (Some(today.clone()), Some(today))
    } else if cli.last_7d {
        let since = (report_today - chrono::Duration::days(6))
            .format("%Y%m%d")
            .to_string();
        let until = report_today.format("%Y%m%d").to_string();
        (Some(since), Some(until))
    } else if cli.last_30d {
        let since = (report_today - chrono::Duration::days(29))
            .format("%Y%m%d")
            .to_string();
        let until = report_today.format("%Y%m%d").to_string();
        (Some(since), Some(until))
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

    // Validate at least one Claude directory exists
    let any_dir_exists = claude_dirs.iter().any(|d| d.exists());
    if !any_dir_exists && !codex_usage::has_codex_usage() {
        anyhow::bail!(
            "No Claude or Codex usage data found.\nHint: Make sure Claude Code or Codex has been used at least once.\nPrimary Claude dir: {}\nClaude searched: {}\nCodex searched: CODEX_HOME or ~/.codex",
            claude_dir.display(),
            claude_dirs
                .iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Create parser with all discovered directories
    let parser = UsageParser::new_multi_with_options(
        claude_dirs.clone(),
        since_date.clone(),
        until_date.clone(),
        cli.model_filter.clone(),
        cli.cost_mode.into(),
        Some(cli.timezone.clone()),
        cli.project.clone(),
    )?;

    // Cache stats reads a single session file directly and should not depend on
    // aggregated usage matching the current global filters.
    if let Some(Commands::CacheStats { session_id }) = &cli.command {
        return handle_cache_stats_command(&claude_dirs, session_id, cli.json);
    }
    let is_tui_command = cli.tui || matches!(&cli.command, Some(Commands::Tui));
    let show_default_agent_breakdown = cli.command.is_none() && !cli.tui && !cli.json;
    let needs_model_report = cli.by_model || cli.breakdown || show_default_agent_breakdown;

    let (mut daily_map, mut session_map, billing_manager, mut model_source_map) =
        if needs_model_report {
            let (daily_map, session_map, billing_manager, claude_models) =
                parser.parse_all_with_models()?;
            let model_source_map: std::collections::HashMap<(String, String), models::TokenUsage> =
                claude_models
                    .into_iter()
                    .map(|(model, usage)| (("Claude Code".to_string(), model), usage))
                    .collect();

            (daily_map, session_map, billing_manager, model_source_map)
        } else {
            let (daily_map, session_map, billing_manager) = parser.parse_all()?;
            (
                daily_map,
                session_map,
                billing_manager,
                std::collections::HashMap::new(),
            )
        };

    match codex_usage::parse_codex_usage(
        since_date.as_deref(),
        until_date.as_deref(),
        Some(&cli.timezone),
        cli.model_filter.as_deref(),
        !matches!(cli.cost_mode, CliCostMode::Display),
    ) {
        Ok(codex_data) => {
            for warning in &codex_data.warnings {
                print_warning(&format!("Partial Codex audit: {warning}"));
            }
            for (date, usage) in codex_data.daily {
                daily_map.entry(date).or_default().add(&usage);
            }
            for (session_id, (usage, last_activity)) in codex_data.sessions {
                let session = session_map
                    .entry(session_id)
                    .or_insert_with(|| (models::TokenUsage::default(), last_activity));
                session.0.add(&usage);
                if last_activity > session.1 {
                    session.1 = last_activity;
                }
            }
            if needs_model_report {
                for (key, usage) in codex_data.models {
                    model_source_map.entry(key).or_default().add(&usage);
                }
            }
        }
        Err(error) if codex_usage::has_codex_usage() => {
            print_warning(&format!("Could not audit Codex usage: {error}"));
        }
        Err(_) => {}
    }

    // Check if we have any data
    if daily_map.is_empty()
        && session_map.is_empty()
        && !is_tui_command
        && (!needs_model_report || model_source_map.is_empty())
    {
        if let Some(Commands::BudgetStatus { budget }) = &cli.command {
            let empty_daily_report = crate::models::DailyReport {
                daily: Vec::new(),
                totals: crate::models::TokenUsageTotals {
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 0,
                    total_tokens: 0,
                    total_cost: 0.0,
                },
            };
            return handle_budget_status_command(&empty_daily_report, *budget, cli.json);
        }

        print_warning("No usage data found for the specified criteria");
        return Ok(());
    }

    // Clone maps for potential re-generation with different sorting
    let daily_map_clone = daily_map.clone();
    let session_map_clone = session_map.clone();

    // Generate default reports
    let mut daily_report = generate_daily_report_sorted(daily_map, None, None);
    let mut session_report = generate_session_report_sorted(session_map, None, None);
    let model_report = if needs_model_report {
        generate_model_report_from_sources(std::mem::take(&mut model_source_map))
    } else {
        empty_model_report()
    };

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

    if let Some(Commands::Statusline) = &cli.command {
        return handle_statusline_command(&daily_report, &billing_manager, cli.json);
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
    if cli.by_model || cli.breakdown {
        return handle_model_report_command(&model_report, cli.json);
    }

    // Handle TUI flag or command
    if cli.tui {
        let mut tui_app = TuiApp::new(
            daily_report,
            session_report,
            model_report.clone(),
            billing_manager.clone(),
        );
        tui_app.set_model_report_receiver(spawn_model_report_loader(
            claude_dirs.clone(),
            since_date.clone(),
            until_date.clone(),
            cli.model_filter.clone(),
            cli.cost_mode,
            cli.timezone.clone(),
            cli.project.clone(),
        ));

        // Try to restore previous session state
        if let Ok(state) = TuiSessionState::load()
            && state.should_resume()
        {
            restore_tui_state(&mut tui_app, &state);
            tui_app.set_restored_state();
        }

        let result = tui_app.run();

        // Save final state on exit
        save_tui_state(&tui_app, TuiMode::Basic).ok();
        return result;
    }

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
                display_report_json(&daily_report);
            } else if cli.responsive {
                display_daily_report_responsive(&daily_report);
            } else if cli.classic || classic {
                display_daily_report_table(&daily_report);
            } else if cli.verbose {
                display_daily_report_enhanced(&daily_report, cli.compact);
            } else {
                display::display_daily_report_compact(&daily_report);
            }

            if show_default_agent_breakdown {
                println!();
                display_default_agent_model_summary(&model_report);
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
                display_report_json(&session_report);
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
                display_report_json(&monthly_report);
            } else if cli.classic || classic {
                display_monthly_report_table(&monthly_report);
            } else {
                display_monthly_report_enhanced(&monthly_report);
            }
        }
        Commands::Weekly {
            classic,
            sort_by,
            sort_order,
            start_of_week,
        } => {
            let weekday = match start_of_week.to_lowercase().as_str() {
                "monday" | "mon" => chrono::Weekday::Mon,
                "sunday" | "sun" => chrono::Weekday::Sun,
                other => {
                    print_error(&format!(
                        "Invalid start-of-week '{}'. Use 'monday' or 'sunday'.",
                        other
                    ));
                    return Ok(());
                }
            };

            let weekly_report = reports::generate_weekly_report_sorted(
                daily_map_clone.clone(),
                convert_sort_field(sort_by),
                convert_sort_order(sort_order),
                weekday,
            );

            if weekly_report.weekly.is_empty() {
                print_warning("No weekly usage data found for the specified date range");
            } else if cli.json {
                display::display_report_json(&weekly_report);
            } else if cli.classic || classic {
                display::display_weekly_report_table(&weekly_report);
            } else {
                display::display_weekly_report_enhanced(&weekly_report);
            }
        }
        Commands::Tui => {
            let mut tui_app = TuiApp::new(
                daily_report,
                session_report,
                model_report.clone(),
                billing_manager.clone(),
            );
            tui_app.set_model_report_receiver(spawn_model_report_loader(
                claude_dirs.clone(),
                since_date.clone(),
                until_date.clone(),
                cli.model_filter.clone(),
                cli.cost_mode,
                cli.timezone.clone(),
                cli.project.clone(),
            ));

            // Try to restore previous session state
            if let Ok(state) = TuiSessionState::load()
                && state.should_resume()
            {
                restore_tui_state(&mut tui_app, &state);
                tui_app.set_restored_state();
            }

            let result = tui_app.run();

            save_tui_state(&tui_app, TuiMode::Basic).ok();
            result?;
        }
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
            token_limit,
            cost_limit,
        } => {
            handle_blocks_command(
                &claude_dir,
                BlocksCommandOptions {
                    active,
                    length,
                    recent,
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
        Commands::Cache {
            top,
            top_projects,
            project,
            sort,
            asc,
            desc,
            threshold,
            min_hit,
            min_churn,
        } => {
            let sort_asc = if asc {
                Some(true)
            } else if desc {
                Some(false)
            } else {
                None
            };
            handle_cache_command(
                &claude_dir,
                since_date.as_deref(),
                until_date.as_deref(),
                project.as_deref(),
                cli.json,
                top,
                top_projects,
                convert_cache_sort_field(sort),
                threshold,
                sort_asc,
                min_hit,
                min_churn,
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
        Commands::BudgetStatus { budget } => {
            handle_budget_status_command(&daily_report, budget, cli.json)?;
        }
        Commands::Statusline => {
            handle_statusline_command(&daily_report, &billing_manager, cli.json)?;
        }
        _ => {} // Other commands handled above
    }

    Ok(())
}

fn spawn_model_report_loader(
    claude_dirs: Vec<PathBuf>,
    since_date: Option<String>,
    until_date: Option<String>,
    model_filter: Option<String>,
    cost_mode: CliCostMode,
    timezone: String,
    project: Option<String>,
) -> std::sync::mpsc::Receiver<std::result::Result<ModelReport, String>> {
    let (sender, receiver) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = load_model_report(
            claude_dirs,
            since_date,
            until_date,
            model_filter,
            cost_mode,
            timezone,
            project,
        )
        .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });

    receiver
}

fn load_model_report(
    claude_dirs: Vec<PathBuf>,
    since_date: Option<String>,
    until_date: Option<String>,
    model_filter: Option<String>,
    cost_mode: CliCostMode,
    timezone: String,
    project: Option<String>,
) -> Result<ModelReport> {
    let parser = UsageParser::new_multi_with_options(
        claude_dirs,
        since_date.clone(),
        until_date.clone(),
        model_filter.clone(),
        cost_mode.into(),
        Some(timezone.clone()),
        project,
    )?;
    let (_, _, _, claude_models) = parser.parse_all_with_models()?;
    let mut model_source_map: std::collections::HashMap<(String, String), models::TokenUsage> =
        claude_models
            .into_iter()
            .map(|(model, usage)| (("Claude Code".to_string(), model), usage))
            .collect();

    match codex_usage::parse_codex_model_usage(
        since_date.as_deref(),
        until_date.as_deref(),
        Some(&timezone),
        model_filter.as_deref(),
        !matches!(cost_mode, CliCostMode::Display),
    ) {
        Ok(codex_models) => {
            for (key, usage) in codex_models {
                model_source_map.entry(key).or_default().add(&usage);
            }
        }
        Err(error) if codex_usage::has_codex_usage() => {
            print_warning(&format!("Could not audit Codex model usage: {error}"));
        }
        Err(_) => {}
    }

    Ok(generate_model_report_from_sources(model_source_map))
}

fn handle_model_report_command(model_report: &ModelReport, json: bool) -> Result<()> {
    if json {
        display_report_json(model_report);
        return Ok(());
    }

    if model_report.models.is_empty() {
        print_warning("No model usage data found for the specified criteria");
        return Ok(());
    }

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::ASCII_FULL);
    table.set_header(vec![
        "Agent", "Family", "Model", "Cost", "Share", "$/1M", "Tokens", "Cache W", "Cache R", "Hit%",
    ]);

    for model in &model_report.models {
        let cost_share = if model_report.totals.total_cost > 0.0 {
            (model.total_cost / model_report.totals.total_cost) * 100.0
        } else {
            0.0
        };
        let hit_pct = model_cache_hit_rate_pct(
            &model.agent,
            model.input_tokens,
            model.cache_read_tokens,
            model.cache_creation_tokens,
        );
        let cost_per_million = cost_per_million_tokens(model.total_cost, model.total_tokens);

        table.add_row(vec![
            model.agent.clone(),
            model.family.clone(),
            model.model.clone(),
            format_cli_cost(model.total_cost),
            format!("{cost_share:.1}%"),
            format_cli_cost(cost_per_million),
            format_cli_number(model.total_tokens),
            format_cache_write(&model.agent, model.cache_creation_tokens),
            format_cli_number(model.cache_read_tokens),
            format!("{hit_pct:.1}%"),
        ]);
    }
    table.add_row(vec![
        "TOTAL".to_string(),
        "—".to_string(),
        "All models".to_string(),
        format_cli_cost(model_report.totals.total_cost),
        "100.0%".to_string(),
        format_cli_cost(cost_per_million_tokens(
            model_report.totals.total_cost,
            model_report.totals.total_tokens,
        )),
        format_cli_number(model_report.totals.total_tokens),
        format_cli_number(model_report.totals.cache_creation_tokens),
        format_cli_number(model_report.totals.cache_read_tokens),
        "—".to_string(),
    ]);

    println!("Usage by Agent and Model");
    println!("{table}");

    Ok(())
}

#[derive(Default)]
struct AgentUsageSummary {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    total_tokens: u64,
    total_cost: f64,
    models: usize,
}

impl AgentUsageSummary {
    fn add_model(&mut self, model: &models::ModelUsage) {
        self.input_tokens = self.input_tokens.saturating_add(model.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(model.output_tokens);
        self.cache_creation_tokens = self
            .cache_creation_tokens
            .saturating_add(model.cache_creation_tokens);
        self.cache_read_tokens = self
            .cache_read_tokens
            .saturating_add(model.cache_read_tokens);
        self.total_tokens = self.total_tokens.saturating_add(model.total_tokens);
        self.total_cost += model.total_cost;
        self.models += 1;
    }
}

fn display_default_agent_model_summary(model_report: &ModelReport) {
    print!("{}", render_default_agent_model_summary(model_report));
}

fn render_default_agent_model_summary(model_report: &ModelReport) -> String {
    let summaries = summarize_model_report(model_report);

    let mut agent_table = comfy_table::Table::new();
    agent_table.load_preset(comfy_table::presets::ASCII_FULL);
    agent_table.set_header(vec![
        "Agent",
        "Cost",
        "Tokens",
        "Models",
        "Input",
        "Output",
        "Cache Write",
        "Cache Read",
    ]);
    for (agent, summary) in &summaries {
        agent_table.add_row(vec![
            agent.clone(),
            format_cli_cost(summary.total_cost),
            format_cli_number(summary.total_tokens),
            summary.models.to_string(),
            format_cli_number(summary.input_tokens),
            format_cli_number(summary.output_tokens),
            format_cache_write(agent, summary.cache_creation_tokens),
            format_cli_number(summary.cache_read_tokens),
        ]);
    }
    agent_table.add_row(vec![
        "TOTAL".to_string(),
        format_cli_cost(model_report.totals.total_cost),
        format_cli_number(model_report.totals.total_tokens),
        model_report.models.len().to_string(),
        format_cli_number(model_report.totals.input_tokens),
        format_cli_number(model_report.totals.output_tokens),
        format_cli_number(model_report.totals.cache_creation_tokens),
        format_cli_number(model_report.totals.cache_read_tokens),
    ]);

    if model_report.models.is_empty() {
        return format!(
            "No model usage found in the loaded scope.\n\nUsage by agent\n{agent_table}\n"
        );
    }

    let mut models: Vec<_> = model_report.models.iter().collect();
    models.sort_by(|left, right| {
        left.agent.cmp(&right.agent).then_with(|| {
            right
                .total_cost
                .partial_cmp(&left.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    let mut model_table = comfy_table::Table::new();
    model_table.load_preset(comfy_table::presets::ASCII_FULL);
    model_table.set_header(vec![
        "Agent",
        "Model",
        "Cost",
        "Tokens",
        "Input",
        "Output",
        "Cache Write",
        "Cache Read",
    ]);
    for model in models {
        model_table.add_row(vec![
            model.agent.clone(),
            model.model.clone(),
            format_cli_cost(model.total_cost),
            format_cli_number(model.total_tokens),
            format_cli_number(model.input_tokens),
            format_cli_number(model.output_tokens),
            format_cache_write(&model.agent, model.cache_creation_tokens),
            format_cli_number(model.cache_read_tokens),
        ]);
    }
    model_table.add_row(vec![
        "TOTAL".to_string(),
        "All models".to_string(),
        format_cli_cost(model_report.totals.total_cost),
        format_cli_number(model_report.totals.total_tokens),
        format_cli_number(model_report.totals.input_tokens),
        format_cli_number(model_report.totals.output_tokens),
        format_cli_number(model_report.totals.cache_creation_tokens),
        format_cli_number(model_report.totals.cache_read_tokens),
    ]);

    format!("Usage by model\n{model_table}\n\nUsage by agent\n{agent_table}\n")
}

fn summarize_model_report(
    model_report: &ModelReport,
) -> std::collections::BTreeMap<String, AgentUsageSummary> {
    let mut summaries = std::collections::BTreeMap::from([
        ("Claude Code".to_string(), AgentUsageSummary::default()),
        ("Codex".to_string(), AgentUsageSummary::default()),
    ]);
    for model in &model_report.models {
        summaries
            .entry(model.agent.clone())
            .or_default()
            .add_model(model);
    }
    summaries
}

fn format_cache_write(agent: &str, tokens: u64) -> String {
    if (agent.eq_ignore_ascii_case("codex") || agent.eq_ignore_ascii_case("openai")) && tokens == 0
    {
        "—".to_string()
    } else {
        format_cli_number(tokens)
    }
}

fn empty_model_report() -> ModelReport {
    ModelReport {
        models: Vec::new(),
        totals: TokenUsageTotals {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 0,
            total_cost: 0.0,
        },
    }
}

fn format_cli_cost(amount: f64) -> String {
    if amount >= 100.0 {
        format!("${amount:.2}")
    } else {
        format!("${amount:.4}")
    }
}

fn format_cli_number(num: u64) -> String {
    let text = num.to_string();
    let mut out = String::with_capacity(text.len() + text.len() / 3);
    for (idx, ch) in text.chars().enumerate() {
        if idx > 0 && (text.len() - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn model_cache_hit_rate_pct(
    agent: &str,
    input_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
) -> f64 {
    if agent.eq_ignore_ascii_case("codex") || agent.eq_ignore_ascii_case("openai") {
        let prompt_side_tokens = input_tokens
            .saturating_add(cache_creation_tokens)
            .saturating_add(cache_read_tokens);
        if prompt_side_tokens == 0 {
            0.0
        } else {
            cache_read_tokens as f64 / prompt_side_tokens as f64 * 100.0
        }
    } else {
        let cache_io = cache_read_tokens.saturating_add(cache_creation_tokens);
        if cache_io == 0 {
            0.0
        } else {
            cache_read_tokens as f64 / cache_io as f64 * 100.0
        }
    }
}

fn cost_per_million_tokens(total_cost: f64, total_tokens: u64) -> f64 {
    if total_tokens == 0 {
        0.0
    } else {
        total_cost / total_tokens as f64 * 1_000_000.0
    }
}

#[derive(Debug, serde::Serialize)]
struct BudgetStatusOutput {
    today_cost: f64,
    burn_rate_per_hour: f64,
    hours_remaining: Option<f64>,
    over_budget: bool,
}

#[derive(Debug, serde::Serialize)]
struct StatuslineOutput {
    today_cost: f64,
    total_cost: f64,
    total_tokens: u64,
    active_block_cost: Option<f64>,
    active_block_tokens: Option<u64>,
    active_block_remaining_minutes: Option<i64>,
}

fn handle_statusline_command(
    daily_report: &crate::models::DailyReport,
    billing_manager: &crate::billing_blocks::BillingBlockManager,
    json: bool,
) -> Result<()> {
    let now = chrono::Utc::now();
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let today_cost = daily_report
        .daily
        .iter()
        .find(|d| d.date == today)
        .map(|d| d.total_cost)
        .unwrap_or(0.0);

    let active_block = billing_manager.get_current_block();
    let active_block_cost = active_block.map(|block| block.usage.total_cost);
    let active_block_tokens = active_block.map(|block| block.usage.total_tokens());
    let active_block_remaining_minutes =
        active_block.map(|block| (block.end_time - now).num_minutes().max(0));

    let output = StatuslineOutput {
        today_cost,
        total_cost: daily_report.totals.total_cost,
        total_tokens: daily_report.totals.total_tokens,
        active_block_cost,
        active_block_tokens,
        active_block_remaining_minutes,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let block = match (
        output.active_block_cost,
        output.active_block_tokens,
        output.active_block_remaining_minutes,
    ) {
        (Some(cost), Some(tokens), Some(minutes)) => format!(
            "${:.2} block / {} tokens / {}h {}m left",
            cost,
            format_number(tokens),
            minutes / 60,
            minutes % 60
        ),
        _ => "No active block".to_string(),
    };

    println!(
        "Claude usage | ${:.2} today / ${:.2} total / {} total tokens / {}",
        output.today_cost,
        output.total_cost,
        format_number(output.total_tokens),
        block
    );
    Ok(())
}

fn handle_budget_status_command(
    daily_report: &crate::models::DailyReport,
    budget: f64,
    json: bool,
) -> Result<()> {
    let now = Local::now();
    let today = now.date_naive().format("%Y-%m-%d").to_string();
    let today_cost = daily_report
        .daily
        .iter()
        .find(|d| d.date == today)
        .map(|d| d.total_cost)
        .unwrap_or(0.0);

    let hours_elapsed = now.time().num_seconds_from_midnight() as f64 / 3600.0;
    let rate = today_cost / hours_elapsed.max(0.1);
    let hours_remaining = if rate > 0.0 && !output_over_budget(today_cost, budget) {
        Some((budget - today_cost) / rate)
    } else {
        None
    };

    let output = BudgetStatusOutput {
        today_cost,
        burn_rate_per_hour: rate,
        hours_remaining,
        over_budget: output_over_budget(today_cost, budget),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Budget Status");
        println!("Today's cost:    ${:.4}", output.today_cost);
        println!("Burn rate:       ${:.4}/hr", output.burn_rate_per_hour);
        if output.over_budget {
            println!(
                "Status:          OVER BUDGET (${:.4} over)",
                today_cost - budget
            );
        } else if let Some(hours_remaining) = output.hours_remaining {
            println!("Hours remaining: {:.1}h at current rate", hours_remaining);
        } else {
            println!("Hours remaining: n/a (no spend yet today)");
        }
    }

    Ok(())
}

fn output_over_budget(today_cost: f64, budget: f64) -> bool {
    today_cost > budget
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
    let mut tui_app = TuiApp::new(
        daily_report,
        session_report,
        crate::models::ModelReport {
            models: Vec::new(),
            totals: crate::models::TokenUsageTotals {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 0,
                total_cost: 0.0,
            },
        },
        billing_manager.clone(),
    );

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
        tui_app.model_report.clone(),
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
    token_limit: Option<u64>,
    cost_limit: Option<f64>,
    since: Option<String>,
    until: Option<String>,
}

/// Handle session blocks command
fn handle_blocks_command(claude_dir: &Path, options: BlocksCommandOptions) -> Result<()> {
    use colored::Colorize;

    // Create session block configuration
    let config = SessionBlockConfig {
        block_hours: options.length,
        token_limit: options.token_limit,
        cost_limit: options.cost_limit,
    };

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
    for (session_path, (usage, last_activity)) in &session_map {
        block_manager.add_usage(*last_activity, usage, session_path);
    }

    // Generate report
    let report = block_manager.generate_report();

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
        sorted_blocks.sort_by_key(|block| std::cmp::Reverse(block.start_time));

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

        if let Some(days_until) = projection.days_until_limit
            && let Some(limit_date) = projection.limit_date
        {
            let warning = if days_until <= 7 { "⚠️ " } else { "" };
            println!(
                "{}Days Until Limit: {} ({})",
                warning,
                days_until,
                limit_date.format("%Y-%m-%d")
            );
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

fn convert_cache_sort_field(field: CacheSortField) -> cache_analysis::CacheSortField {
    match field {
        CacheSortField::WriteCost => cache_analysis::CacheSortField::WriteCost,
        CacheSortField::HitRate => cache_analysis::CacheSortField::HitRate,
        CacheSortField::Miss5m => cache_analysis::CacheSortField::Miss5m,
        CacheSortField::Miss60m => cache_analysis::CacheSortField::Miss60m,
        CacheSortField::ColdStart => cache_analysis::CacheSortField::ColdStart,
        CacheSortField::NormalChurn => cache_analysis::CacheSortField::NormalChurn,
        CacheSortField::BreakevenTurn => cache_analysis::CacheSortField::BreakevenTurn,
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_cache_command(
    claude_dir: &Path,
    since: Option<&str>,
    until: Option<&str>,
    project: Option<&str>,
    json: bool,
    top: usize,
    top_projects: usize,
    sort_field: cache_analysis::CacheSortField,
    warmup_threshold: f64,
    sort_asc: Option<bool>,
    min_hit: Option<f64>,
    min_churn: Option<f64>,
) -> Result<()> {
    use chrono::NaiveDate;
    let since_date = since.and_then(|s| NaiveDate::parse_from_str(s, "%Y%m%d").ok());
    let until_date = until.and_then(|s| NaiveDate::parse_from_str(s, "%Y%m%d").ok());
    let threshold = warmup_threshold.clamp(0.5, 0.99);
    let analysis =
        cache_analysis::analyze_cache(claude_dir, since_date, until_date, project, threshold)?;
    cache_analysis::display_cache_analysis(
        &analysis,
        json,
        top,
        top_projects,
        sort_field,
        threshold,
        sort_asc,
        min_hit,
        min_churn,
    );
    Ok(())
}

fn handle_cache_stats_command(claude_dirs: &[PathBuf], session_id: &str, json: bool) -> Result<()> {
    let filename = format!("{}.jsonl", session_id);
    // Sessions live at <claude_dir>/projects/<project-dir>/<session-id>.jsonl.
    // Check each project subdir directly instead of walking all files.
    let file_path = claude_dirs
        .iter()
        .map(|d| d.join("projects"))
        .filter(|p| p.is_dir())
        .flat_map(|projects_dir| {
            std::fs::read_dir(projects_dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| e.path().join(&filename))
        })
        .find(|p| p.is_file());

    let file_path = match file_path {
        Some(p) => p,
        None => anyhow::bail!("Session not found: {}", session_id),
    };

    let output = cache_analysis::compute_session_cache_stats(&file_path, 5);

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Session: {}", session_id);
        println!("Turn count: {}", output.turn_count);
        println!("Cold %:    {:.1}%", output.cold_pct * 100.0);
        println!("Hit %:     {:.1}%", output.hit_pct * 100.0);
        if let Some(churn) = output.churn_tokens_per_turn {
            println!("Churn:     {} tokens/turn (last 5 turns)", churn);
        } else {
            println!("Churn:     n/a (fewer than 5 turns)");
        }
        println!("Total tokens:       {}", output.total_tokens);
        println!("Cache read tokens:  {}", output.cache_read_tokens);
    }

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
            if let Ok(metadata) = std::fs::metadata(path)
                && let Ok(modified) = metadata.modified()
            {
                let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                return modified_time > seven_days_ago;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelUsage;

    fn sample_model_report() -> ModelReport {
        ModelReport {
            models: vec![
                ModelUsage {
                    agent: "Claude Code".to_string(),
                    model: "claude-fable-5".to_string(),
                    family: "fable".to_string(),
                    input_tokens: 100,
                    output_tokens: 20,
                    cache_creation_tokens: 30,
                    cache_read_tokens: 40,
                    total_tokens: 190,
                    total_cost: 0.5,
                },
                ModelUsage {
                    agent: "Codex".to_string(),
                    model: "gpt-5.6-sol".to_string(),
                    family: "openai".to_string(),
                    input_tokens: 200,
                    output_tokens: 50,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 80,
                    total_tokens: 330,
                    total_cost: 0.25,
                },
            ],
            totals: TokenUsageTotals {
                input_tokens: 300,
                output_tokens: 70,
                cache_creation_tokens: 30,
                cache_read_tokens: 120,
                total_tokens: 520,
                total_cost: 0.75,
            },
        }
    }

    #[test]
    fn default_summary_keeps_agents_and_cache_directions_separate() {
        let report = sample_model_report();
        let summaries = summarize_model_report(&report);
        let claude = summaries.get("Claude Code").unwrap();
        let codex = summaries.get("Codex").unwrap();
        assert_eq!(claude.cache_creation_tokens, 30);
        assert_eq!(claude.cache_read_tokens, 40);
        assert_eq!(codex.cache_creation_tokens, 0);
        assert_eq!(codex.cache_read_tokens, 80);
        assert_eq!(claude.models, 1);
        assert_eq!(codex.models, 1);
        assert_eq!(format_cache_write("Claude Code", 0), "0");
        assert_eq!(format_cache_write("Codex", 0), "—");
    }

    #[test]
    fn default_summary_ends_with_agent_totals() {
        let rendered = render_default_agent_model_summary(&sample_model_report());
        let model_position = rendered.find("Usage by model").unwrap();
        let agent_position = rendered.find("Usage by agent").unwrap();

        assert!(model_position < agent_position);

        let final_section = &rendered[agent_position..];
        let claude_position = final_section.find("| Claude Code").unwrap();
        let codex_position = final_section.find("| Codex").unwrap();
        let total_position = final_section.find("| TOTAL").unwrap();
        assert!(claude_position < codex_position);
        assert!(codex_position < total_position);

        let lines: Vec<_> = rendered.trim_end().lines().collect();
        let total_line = lines
            .iter()
            .rposition(|line| line.contains("| TOTAL"))
            .unwrap();
        assert_eq!(total_line + 2, lines.len());
        assert!(lines.last().is_some_and(|line| {
            line.starts_with('+') && line.ends_with('+') && !line.contains('|')
        }));
    }
}
