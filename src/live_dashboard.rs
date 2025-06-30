//! Live dashboard module for real-time token usage monitoring
//!
//! Provides a live monitoring interface that displays:
//! - Real-time token burn rate
//! - Active session progress
//! - Cost projections
//! - Time to reach limits

use crate::burn_rate::BurnRateMetrics;
use crate::display::print_info;
use crate::models::{SessionUsageMap, TokenUsage};
use crate::parser::UsageParser;
use crate::session_blocks::{SessionBlockConfig, SessionBlockManager};
use anyhow::Result;
use chrono::{DateTime, Duration, Local, Utc};
use colored::Colorize;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration as StdDuration;
use terminal_size::{Width, terminal_size};

/// Configuration for the live dashboard
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LiveDashboardConfig {
    /// Refresh interval in seconds
    pub refresh_interval: u64,
    /// Token limit for warnings
    pub token_limit: Option<u64>,
    /// Daily cost limit in USD
    pub daily_cost_limit: Option<f64>,
    /// Monthly cost limit in USD
    pub monthly_cost_limit: Option<f64>,
    /// Show detailed session information
    pub show_details: bool,
    /// Enable alerts for high burn rates
    pub enable_alerts: bool,
}

impl Default for LiveDashboardConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 5,
            token_limit: None,
            daily_cost_limit: None,
            monthly_cost_limit: None,
            show_details: true,
            enable_alerts: true,
        }
    }
}

/// Live dashboard for monitoring Claude usage in real-time
pub struct LiveDashboard {
    config: LiveDashboardConfig,
    parser: UsageParser,
    session_manager: SessionBlockManager,
    last_update: DateTime<Local>,
    active_sessions: HashMap<String, ActiveSessionInfo>,
    running: Arc<AtomicBool>,
}

/// Information about an active session
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ActiveSessionInfo {
    pub project_path: String,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub usage: TokenUsage,
    pub burn_rate: Option<BurnRateMetrics>,
}

impl LiveDashboard {
    /// Create a new live dashboard
    pub fn new(claude_dir: &Path, config: LiveDashboardConfig) -> Result<Self> {
        let parser = UsageParser::new(claude_dir.to_path_buf(), None, None, None)?;

        let session_config = SessionBlockConfig {
            block_hours: 1, // 1-hour blocks for fine-grained tracking
            token_limit: config.token_limit,
            cost_limit: config.daily_cost_limit,
        };

        let session_manager = SessionBlockManager::new(session_config);

        Ok(Self {
            config,
            parser,
            session_manager,
            last_update: Local::now(),
            active_sessions: HashMap::new(),
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Start the live dashboard
    pub fn run(&mut self) -> Result<()> {
        // Clear screen and set up terminal
        self.clear_screen();
        self.hide_cursor();

        // Set up Ctrl+C handler
        let running = self.running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })?;

        print_info("🚀 Starting Live Dashboard - Press Ctrl+C to stop");
        thread::sleep(StdDuration::from_secs(1));

        // Main loop
        while self.running.load(Ordering::SeqCst) {
            self.update_data()?;
            self.render_dashboard()?;

            // Sleep for refresh interval
            thread::sleep(StdDuration::from_secs(self.config.refresh_interval));
        }

        // Cleanup
        self.show_cursor();
        self.clear_screen();
        print_info("👋 Live Dashboard stopped");

        Ok(())
    }

    /// Update session data
    fn update_data(&mut self) -> Result<()> {
        self.last_update = Local::now();

        // Parse latest data
        let (_daily_map, session_map, _billing_manager) = self.parser.parse_all()?;

        // Update active sessions
        self.update_active_sessions(&session_map)?;

        // Update session blocks
        self.update_session_blocks(&session_map)?;

        Ok(())
    }

    /// Update active sessions based on recent activity
    fn update_active_sessions(&mut self, session_map: &SessionUsageMap) -> Result<()> {
        let now = Utc::now();
        let active_threshold = now - Duration::minutes(15); // Consider sessions active if used in last 15 minutes

        self.active_sessions.clear();

        for (session_path, (usage, last_activity)) in session_map {
            if *last_activity > active_threshold {
                // Calculate session start time (approximate based on first activity)
                let start_time = *last_activity - Duration::hours(1); // Rough estimate

                // Calculate burn rate for this session
                let duration = now - start_time;
                let hours_elapsed = duration.num_seconds() as f64 / 3600.0;

                let burn_rate = if hours_elapsed > 0.1 {
                    Some(self.calculate_session_burn_rate(usage, hours_elapsed))
                } else {
                    None
                };

                self.active_sessions.insert(
                    session_path.clone(),
                    ActiveSessionInfo {
                        project_path: session_path.clone(),
                        start_time,
                        last_activity: *last_activity,
                        usage: usage.clone(),
                        burn_rate,
                    },
                );
            }
        }

        Ok(())
    }

    /// Update session blocks
    fn update_session_blocks(&mut self, session_map: &SessionUsageMap) -> Result<()> {
        // Clear and rebuild session manager
        let session_config = SessionBlockConfig {
            block_hours: 1,
            token_limit: self.config.token_limit,
            cost_limit: self.config.daily_cost_limit,
        };
        self.session_manager = SessionBlockManager::new(session_config);

        // Add all sessions to blocks
        for (session_id, (usage, timestamp)) in session_map {
            self.session_manager
                .add_usage(*timestamp, usage, session_id);
        }

        // Calculate burn rates
        self.session_manager.calculate_burn_rates();

        Ok(())
    }

    /// Calculate burn rate metrics for a session
    fn calculate_session_burn_rate(
        &self,
        usage: &TokenUsage,
        hours_elapsed: f64,
    ) -> BurnRateMetrics {
        let tokens_per_hour = usage.total_tokens() as f64 / hours_elapsed;
        let cost_per_hour = usage.total_cost / hours_elapsed;

        BurnRateMetrics {
            tokens_per_hour,
            cost_per_hour,
            projected_daily_tokens: (tokens_per_hour * 24.0) as u64,
            projected_daily_cost: cost_per_hour * 24.0,
            projected_monthly_tokens: (tokens_per_hour * 24.0 * 30.0) as u64,
            projected_monthly_cost: cost_per_hour * 24.0 * 30.0,
            trend_percentage: 0.0,
            hours_until_budget_limit: self.calculate_time_to_limit(
                usage,
                cost_per_hour,
                tokens_per_hour,
            ),
        }
    }

    /// Calculate time until limits are reached
    fn calculate_time_to_limit(
        &self,
        current_usage: &TokenUsage,
        cost_per_hour: f64,
        tokens_per_hour: f64,
    ) -> Option<f64> {
        let mut time_to_limit = None;

        // Check token limit
        if let Some(limit) = self.config.token_limit {
            let remaining = limit.saturating_sub(current_usage.total_tokens());
            let hours = remaining as f64 / tokens_per_hour;
            time_to_limit = Some(hours);
        }

        // Check daily cost limit
        if let Some(limit) = self.config.daily_cost_limit {
            let remaining = limit - current_usage.total_cost;
            let hours = remaining / cost_per_hour;
            time_to_limit = Some(time_to_limit.map_or(hours, |t| t.min(hours)));
        }

        time_to_limit.filter(|&h| h > 0.0)
    }

    /// Render the dashboard
    fn render_dashboard(&self) -> Result<()> {
        self.clear_screen();

        // Get terminal width
        let term_width = terminal_size()
            .map(|(Width(w), _)| w as usize)
            .unwrap_or(80);

        // Header
        self.render_header(term_width)?;

        // Active sessions
        self.render_active_sessions(term_width)?;

        // Burn rate summary
        self.render_burn_rate_summary(term_width)?;

        // Projections
        self.render_projections(term_width)?;

        // Alerts
        if self.config.enable_alerts {
            self.render_alerts(term_width)?;
        }

        // Footer
        self.render_footer(term_width)?;

        io::stdout().flush()?;

        Ok(())
    }

    /// Render dashboard header
    fn render_header(&self, width: usize) -> Result<()> {
        let title = " 🔥 CLAUDELYTICS LIVE DASHBOARD 🔥 ";
        let padding = (width.saturating_sub(title.len())) / 2;

        println!("{}", "═".repeat(width).bright_cyan());
        println!(
            "{}{}{}",
            " ".repeat(padding),
            title.bright_white().bold(),
            " ".repeat(padding)
        );
        println!("{}", "═".repeat(width).bright_cyan());

        println!(
            "📅 {}",
            self.last_update
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .bright_yellow()
        );
        println!();

        Ok(())
    }

    /// Render active sessions
    fn render_active_sessions(&self, width: usize) -> Result<()> {
        println!("{}", "ACTIVE SESSIONS".bright_green().bold());
        println!("{}", "─".repeat(width).bright_black());

        if self.active_sessions.is_empty() {
            println!("{}", "No active sessions detected".dimmed());
        } else {
            for (path, session) in &self.active_sessions {
                let duration = Utc::now() - session.start_time;
                let duration_str = format_duration(duration);

                println!(
                    "📂 {} {}",
                    path.bright_white().bold(),
                    format!("({})", duration_str).dimmed()
                );

                if self.config.show_details {
                    println!(
                        "   💰 Cost: ${:.4} | 🔤 Tokens: {} | ⏱️  Last: {}",
                        session.usage.total_cost,
                        format_number(session.usage.total_tokens()),
                        format_time_ago(Utc::now() - session.last_activity)
                    );

                    if let Some(burn_rate) = &session.burn_rate {
                        println!(
                            "   🔥 Burn: {} tok/hr (${:.4}/hr)",
                            format_number(burn_rate.tokens_per_hour as u64),
                            burn_rate.cost_per_hour
                        );
                    }
                }
            }
        }

        println!();
        Ok(())
    }

    /// Render burn rate summary
    fn render_burn_rate_summary(&self, width: usize) -> Result<()> {
        println!("{}", "BURN RATE ANALYSIS".bright_yellow().bold());
        println!("{}", "─".repeat(width).bright_black());

        // Get current active blocks
        let active_blocks = self.session_manager.get_active_blocks();

        if active_blocks.is_empty() {
            println!("{}", "No active usage blocks".dimmed());
        } else {
            // Calculate aggregate burn rate
            let mut total_tokens_per_hour = 0.0;
            let mut total_cost_per_hour = 0.0;
            let mut block_count = 0;

            for block in active_blocks {
                if let Some(burn_rate) = &block.burn_rate {
                    total_tokens_per_hour += burn_rate.tokens_per_hour;
                    total_cost_per_hour += burn_rate.cost_per_hour;
                    block_count += 1;
                }
            }

            if block_count > 0 {
                println!(
                    "🔥 Current Rate: {} tokens/hour (${:.4}/hour)",
                    format_number(total_tokens_per_hour as u64).bright_red(),
                    total_cost_per_hour
                );

                println!(
                    "📊 Per Minute: {} tokens/min (${:.6}/min)",
                    format_number((total_tokens_per_hour / 60.0) as u64),
                    total_cost_per_hour / 60.0
                );

                // Show trend indicator
                let trend = if total_tokens_per_hour > 10000.0 {
                    "⚠️  HIGH".bright_red()
                } else if total_tokens_per_hour > 5000.0 {
                    "⚡ MODERATE".bright_yellow()
                } else {
                    "✅ NORMAL".bright_green()
                };

                println!("📈 Activity Level: {}", trend);
            }
        }

        println!();
        Ok(())
    }

    /// Render projections
    fn render_projections(&self, width: usize) -> Result<()> {
        println!("{}", "PROJECTIONS".bright_magenta().bold());
        println!("{}", "─".repeat(width).bright_black());

        let active_blocks = self.session_manager.get_active_blocks();

        if let Some(block) = active_blocks.first() {
            if let Some(burn_rate) = &block.burn_rate {
                // Daily projection
                println!(
                    "📅 Daily: {} tokens (${:.2})",
                    format_number(burn_rate.projected_daily_tokens).bright_cyan(),
                    burn_rate.projected_daily_cost
                );

                // Monthly projection
                let projected_monthly_tokens = (burn_rate.tokens_per_hour * 24.0 * 30.0) as u64;
                println!(
                    "📆 Monthly: {} tokens (${:.2})",
                    format_number(projected_monthly_tokens).bright_cyan(),
                    burn_rate.projected_monthly_cost
                );

                // Time to limits
                if let Some(time_to_limit) = burn_rate.time_to_limit {
                    let hours = time_to_limit.num_hours();
                    let minutes = time_to_limit.num_minutes() % 60;

                    let time_str = if hours > 0 {
                        format!("{}h {}m", hours, minutes)
                    } else {
                        format!("{}m", minutes)
                    };

                    let color = if hours < 1 {
                        time_str.bright_red().bold()
                    } else if hours < 6 {
                        time_str.bright_yellow()
                    } else {
                        time_str.bright_green()
                    };

                    println!("⏰ Time to limit: {}", color);
                }
            }
        } else {
            println!("{}", "No active usage to project".dimmed());
        }

        println!();
        Ok(())
    }

    /// Render alerts
    fn render_alerts(&self, width: usize) -> Result<()> {
        let mut alerts = Vec::new();

        // Check for high burn rate
        let active_blocks = self.session_manager.get_active_blocks();
        for block in active_blocks {
            if let Some(burn_rate) = &block.burn_rate {
                if burn_rate.tokens_per_hour > 10000.0 {
                    alerts.push(format!(
                        "⚠️  High burn rate detected: {} tokens/hour",
                        format_number(burn_rate.tokens_per_hour as u64)
                    ));
                }

                // Check if approaching limits
                if let Some(time_to_limit) = burn_rate.time_to_limit {
                    if time_to_limit.num_hours() < 2 {
                        alerts.push(format!(
                            "🚨 Approaching limit in {}!",
                            format_duration(time_to_limit)
                        ));
                    }
                }
            }
        }

        if !alerts.is_empty() {
            println!("{}", "ALERTS".bright_red().bold());
            println!("{}", "─".repeat(width).bright_black());

            for alert in alerts {
                println!("{}", alert.bright_red());
            }

            println!();
        }

        Ok(())
    }

    /// Render footer
    fn render_footer(&self, width: usize) -> Result<()> {
        println!("{}", "─".repeat(width).bright_black());
        println!(
            "{} | {} | {}",
            "Ctrl+C to stop".dimmed(),
            format!("Refresh: {}s", self.config.refresh_interval).dimmed(),
            "Live monitoring active".bright_green()
        );

        Ok(())
    }

    /// Clear the terminal screen
    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
    }

    /// Hide terminal cursor
    fn hide_cursor(&self) {
        print!("\x1B[?25l");
        io::stdout().flush().unwrap();
    }

    /// Show terminal cursor
    fn show_cursor(&self) {
        print!("\x1B[?25h");
        io::stdout().flush().unwrap();
    }
}

/// Format a duration for display
fn format_duration(duration: Duration) -> String {
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Format time ago
fn format_time_ago(duration: Duration) -> String {
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds() % 60;

    if minutes > 0 {
        format!("{}m {}s ago", minutes, seconds)
    } else {
        format!("{}s ago", seconds)
    }
}

/// Format number with commas
fn format_number(num: u64) -> String {
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

/// Options for blocks command (reused for live mode)
#[derive(Debug, Clone)]
pub struct LiveDashboardOptions {
    pub refresh: u64,
    pub token_limit: Option<u64>,
    pub cost_limit: Option<f64>,
    pub show_details: bool,
    pub enable_alerts: bool,
}

impl From<LiveDashboardOptions> for LiveDashboardConfig {
    fn from(options: LiveDashboardOptions) -> Self {
        Self {
            refresh_interval: options.refresh,
            token_limit: options.token_limit,
            daily_cost_limit: options.cost_limit,
            monthly_cost_limit: options.cost_limit.map(|c| c * 30.0),
            show_details: options.show_details,
            enable_alerts: options.enable_alerts,
        }
    }
}

/// Run the live dashboard
pub fn run_live_dashboard(claude_dir: &Path, options: LiveDashboardOptions) -> Result<()> {
    let config = LiveDashboardConfig::from(options);
    let mut dashboard = LiveDashboard::new(claude_dir, config)?;
    dashboard.run()
}
