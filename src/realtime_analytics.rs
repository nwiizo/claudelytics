use crate::burn_rate::{BurnRateCalculator, BurnRateMetrics};
use crate::models::{DailyUsageMap, SessionUsageMap};
use crate::projections::TrendDirection;
use crate::session_analytics::{SessionAnalytics, format_duration};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Timelike, Utc};
use serde::Serialize;
use std::collections::HashMap;

/// Real-time analytics module for comprehensive usage analysis
/// Provides burn rate calculations, budget projections, and session analytics
pub struct RealtimeAnalytics<'a> {
    daily_usage: &'a DailyUsageMap,
    session_usage: &'a SessionUsageMap,
    budget_config: BudgetConfig,
}

/// Budget configuration for projections and alerts
#[derive(Debug, Clone)]
pub struct BudgetConfig {
    pub daily_limit: Option<f64>,
    pub monthly_limit: Option<f64>,
    pub yearly_limit: Option<f64>,
    pub alert_threshold: f64, // Percentage (0.0-1.0) of budget to trigger alert
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            daily_limit: None,
            monthly_limit: None,
            yearly_limit: None,
            alert_threshold: 0.8, // Alert at 80% of budget
        }
    }
}

/// Comprehensive real-time analytics report
#[derive(Debug, Clone, Serialize)]
pub struct RealtimeAnalyticsReport {
    pub burn_rates: BurnRateAnalysis,
    pub budget_projections: BudgetProjections,
    pub session_metrics: SessionMetrics,
    pub alerts: Vec<UsageAlert>,
    pub efficiency_trends: EfficiencyTrends,
}

/// Detailed burn rate analysis with multiple time windows
#[derive(Debug, Clone, Serialize)]
pub struct BurnRateAnalysis {
    pub current_hour: BurnRateMetrics,
    pub last_3_hours: BurnRateMetrics,
    pub last_24_hours: BurnRateMetrics,
    pub tokens_per_minute: f64,
    pub cost_per_minute: f64,
    pub peak_burn_rate: PeakBurnRate,
}

/// Peak burn rate information
#[derive(Debug, Serialize, Clone)]
pub struct PeakBurnRate {
    pub tokens_per_hour: f64,
    pub cost_per_hour: f64,
    pub occurred_at: DateTime<Utc>,
}

/// Budget projection with time estimates
#[derive(Debug, Serialize, Clone)]
pub struct BudgetProjections {
    pub daily_projection: BudgetProjection,
    pub monthly_projection: BudgetProjection,
    pub yearly_projection: BudgetProjection,
    pub time_to_limits: TimeToLimits,
}

/// Individual budget projection
#[derive(Debug, Serialize, Clone)]
pub struct BudgetProjection {
    pub estimated_cost: f64,
    pub budget_limit: Option<f64>,
    pub utilization_percentage: f64,
    pub will_exceed: bool,
    pub margin: f64, // Positive if under budget, negative if over
}

/// Time estimates to reach budget limits
#[derive(Debug, Serialize, Clone)]
pub struct TimeToLimits {
    pub hours_to_daily_limit: Option<f64>,
    pub days_to_monthly_limit: Option<f64>,
    pub days_to_yearly_limit: Option<f64>,
}

/// Session-level metrics and analytics
#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
pub struct SessionMetrics {
    pub active_session_count: usize,
    pub avg_tokens_per_session: f64,
    pub avg_cost_per_session: f64,
    pub avg_session_duration: Duration,
    #[serde(skip)]
    pub current_session_burn_rate: Option<BurnRateMetrics>,
    pub peak_usage_hours: Vec<u32>,
    pub efficiency_score: f64,
}

/// Efficiency trends over time
#[derive(Debug, Serialize, Clone)]
pub struct EfficiencyTrends {
    pub tokens_per_dollar_trend: TrendMetric,
    pub response_time_trend: TrendMetric,
    pub cache_efficiency_trend: TrendMetric,
    pub cost_efficiency_score: f64,
}

/// Trend metric with direction and magnitude
#[derive(Debug, Serialize, Clone)]
pub struct TrendMetric {
    pub current_value: f64,
    pub previous_value: f64,
    pub change_percentage: f64,
    pub direction: TrendDirection,
}

/// Usage alert for unusual patterns or budget concerns
#[derive(Debug, Serialize, Clone)]
pub struct UsageAlert {
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub recommended_action: Option<String>,
}

/// Types of usage alerts
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum AlertType {
    BudgetThreshold,
    UnusualSpike,
    HighBurnRate,
    IneffientUsage,
    ProjectionWarning,
}

/// Alert severity levels
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl<'a> RealtimeAnalytics<'a> {
    /// Create new real-time analytics instance
    pub fn new(
        daily_usage: &'a DailyUsageMap,
        session_usage: &'a SessionUsageMap,
        budget_config: BudgetConfig,
    ) -> Self {
        Self {
            daily_usage,
            session_usage,
            budget_config,
        }
    }

    /// Generate comprehensive real-time analytics report
    pub fn generate_report(&self) -> RealtimeAnalyticsReport {
        let burn_rates = self.calculate_burn_rates();
        let budget_projections = self.calculate_budget_projections(&burn_rates);
        let session_metrics = self.calculate_session_metrics();
        let efficiency_trends = self.calculate_efficiency_trends();
        let alerts = self.generate_alerts(&burn_rates, &budget_projections, &efficiency_trends);

        RealtimeAnalyticsReport {
            burn_rates,
            budget_projections,
            session_metrics,
            alerts,
            efficiency_trends,
        }
    }

    /// Calculate burn rates across multiple time windows
    fn calculate_burn_rates(&self) -> BurnRateAnalysis {
        let calculator = BurnRateCalculator::new(self.daily_usage.clone());

        let current_hour = calculator
            .calculate_burn_rate(1)
            .unwrap_or_else(|| self.empty_burn_rate());

        let last_3_hours = calculator
            .calculate_burn_rate(3)
            .unwrap_or_else(|| self.empty_burn_rate());

        let last_24_hours = calculator
            .calculate_burn_rate(24)
            .unwrap_or_else(|| self.empty_burn_rate());

        // Calculate per-minute rates
        let tokens_per_minute = current_hour.tokens_per_hour / 60.0;
        let cost_per_minute = current_hour.cost_per_hour / 60.0;

        // Find peak burn rate
        let peak_burn_rate = self.find_peak_burn_rate();

        BurnRateAnalysis {
            current_hour,
            last_3_hours,
            last_24_hours,
            tokens_per_minute,
            cost_per_minute,
            peak_burn_rate,
        }
    }

    /// Calculate budget projections based on current usage patterns
    fn calculate_budget_projections(&self, burn_rates: &BurnRateAnalysis) -> BudgetProjections {
        let current_rate = &burn_rates.last_24_hours;

        // Daily projection
        let daily_projection = self.calculate_budget_projection(
            current_rate.projected_daily_cost,
            self.budget_config.daily_limit,
        );

        // Monthly projection
        let monthly_projection = self.calculate_budget_projection(
            current_rate.projected_monthly_cost,
            self.budget_config.monthly_limit,
        );

        // Yearly projection
        let yearly_cost = current_rate.projected_monthly_cost * 12.0;
        let yearly_projection =
            self.calculate_budget_projection(yearly_cost, self.budget_config.yearly_limit);

        // Calculate time to limits
        let time_to_limits = self.calculate_time_to_limits(burn_rates);

        BudgetProjections {
            daily_projection,
            monthly_projection,
            yearly_projection,
            time_to_limits,
        }
    }

    /// Calculate individual budget projection
    fn calculate_budget_projection(
        &self,
        estimated_cost: f64,
        limit: Option<f64>,
    ) -> BudgetProjection {
        match limit {
            Some(budget_limit) => {
                let utilization_percentage = (estimated_cost / budget_limit) * 100.0;
                let will_exceed = estimated_cost > budget_limit;
                let margin = budget_limit - estimated_cost;

                BudgetProjection {
                    estimated_cost,
                    budget_limit: Some(budget_limit),
                    utilization_percentage,
                    will_exceed,
                    margin,
                }
            }
            None => BudgetProjection {
                estimated_cost,
                budget_limit: None,
                utilization_percentage: 0.0,
                will_exceed: false,
                margin: 0.0,
            },
        }
    }

    /// Calculate time remaining to reach budget limits
    fn calculate_time_to_limits(&self, burn_rates: &BurnRateAnalysis) -> TimeToLimits {
        let now = Local::now();
        let today = now.date_naive();

        // Calculate hours to daily limit
        let hours_to_daily_limit = if let Some(daily_limit) = self.budget_config.daily_limit {
            let today_usage = self
                .daily_usage
                .get(&today)
                .map(|u| u.total_cost)
                .unwrap_or(0.0);

            if today_usage >= daily_limit {
                Some(0.0)
            } else {
                let remaining = daily_limit - today_usage;
                let hourly_rate = burn_rates.current_hour.cost_per_hour;
                if hourly_rate > 0.0 {
                    Some(remaining / hourly_rate)
                } else {
                    None
                }
            }
        } else {
            None
        };

        // Calculate days to monthly limit
        let days_to_monthly_limit = if let Some(monthly_limit) = self.budget_config.monthly_limit {
            let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            let month_usage: f64 = self
                .daily_usage
                .iter()
                .filter(|(date, _)| **date >= month_start && **date <= today)
                .map(|(_, usage)| usage.total_cost)
                .sum();

            if month_usage >= monthly_limit {
                Some(0.0)
            } else {
                let remaining = monthly_limit - month_usage;
                let daily_rate = burn_rates.last_24_hours.projected_daily_cost;
                if daily_rate > 0.0 {
                    Some(remaining / daily_rate)
                } else {
                    None
                }
            }
        } else {
            None
        };

        // Calculate days to yearly limit
        let days_to_yearly_limit = if let Some(yearly_limit) = self.budget_config.yearly_limit {
            let year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            let year_usage: f64 = self
                .daily_usage
                .iter()
                .filter(|(date, _)| **date >= year_start && **date <= today)
                .map(|(_, usage)| usage.total_cost)
                .sum();

            if year_usage >= yearly_limit {
                Some(0.0)
            } else {
                let remaining = yearly_limit - year_usage;
                let daily_rate = burn_rates.last_24_hours.projected_daily_cost;
                if daily_rate > 0.0 {
                    Some(remaining / daily_rate)
                } else {
                    None
                }
            }
        } else {
            None
        };

        TimeToLimits {
            hours_to_daily_limit,
            days_to_monthly_limit,
            days_to_yearly_limit,
        }
    }

    /// Calculate session-level metrics
    fn calculate_session_metrics(&self) -> SessionMetrics {
        let analytics = SessionAnalytics::new(self.session_usage);
        let time_analysis = analytics.analyze_time_of_day();
        let duration_analysis = analytics.analyze_session_durations();
        let _frequency_analysis = analytics.analyze_session_frequency();

        // Find active sessions (within last hour)
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let active_sessions: Vec<_> = self
            .session_usage
            .iter()
            .filter(|(_, (_, timestamp))| *timestamp > one_hour_ago)
            .collect();

        let active_session_count = active_sessions.len();

        // Calculate averages
        let total_sessions = self.session_usage.len();
        let (total_tokens, total_cost) = self
            .session_usage
            .values()
            .fold((0u64, 0.0), |(tokens, cost), (usage, _)| {
                (tokens + usage.total_tokens(), cost + usage.total_cost)
            });

        let avg_tokens_per_session = if total_sessions > 0 {
            total_tokens as f64 / total_sessions as f64
        } else {
            0.0
        };

        let avg_cost_per_session = if total_sessions > 0 {
            total_cost / total_sessions as f64
        } else {
            0.0
        };

        // Get current session burn rate if there's an active session
        let current_session_burn_rate =
            if let Some((_, (usage, start_time))) = active_sessions.first() {
                let calculator = BurnRateCalculator::new(self.daily_usage.clone());
                Some(calculator.get_session_burn_rate(
                    *start_time,
                    usage.total_tokens(),
                    usage.total_cost,
                ))
            } else {
                None
            };

        // Find peak usage hours
        let mut peak_hours: Vec<(u32, u64)> = time_analysis
            .hourly_usage
            .iter()
            .map(|(hour, metrics)| (*hour, metrics.usage.total_tokens()))
            .collect();
        peak_hours.sort_by(|a, b| b.1.cmp(&a.1));
        let peak_usage_hours: Vec<u32> = peak_hours.iter().take(3).map(|(h, _)| *h).collect();

        // Calculate efficiency score (tokens per dollar normalized)
        let efficiency_score = if total_cost > 0.0 {
            let tokens_per_dollar = total_tokens as f64 / total_cost;
            // Normalize to 0-100 scale (assuming 100k tokens per dollar is excellent)
            (tokens_per_dollar / 100_000.0 * 100.0).min(100.0)
        } else {
            0.0
        };

        SessionMetrics {
            active_session_count,
            avg_tokens_per_session,
            avg_cost_per_session,
            avg_session_duration: duration_analysis.avg_session_duration,
            current_session_burn_rate,
            peak_usage_hours,
            efficiency_score,
        }
    }

    /// Calculate efficiency trends
    fn calculate_efficiency_trends(&self) -> EfficiencyTrends {
        let now = Local::now().date_naive();
        let yesterday = now - Duration::days(1);
        let week_ago = now - Duration::days(7);

        // Tokens per dollar trend
        let tokens_per_dollar_today = self.calculate_daily_efficiency(now);
        let tokens_per_dollar_yesterday = self.calculate_daily_efficiency(yesterday);
        let tokens_per_dollar_trend =
            self.create_trend_metric(tokens_per_dollar_today, tokens_per_dollar_yesterday);

        // Response time trend (simulated based on token ratios)
        let response_time_today = self.calculate_response_time_metric(now);
        let response_time_yesterday = self.calculate_response_time_metric(yesterday);
        let response_time_trend =
            self.create_trend_metric(response_time_today, response_time_yesterday);

        // Cache efficiency trend
        let cache_efficiency_today = self.calculate_cache_efficiency(now);
        let cache_efficiency_yesterday = self.calculate_cache_efficiency(yesterday);
        let cache_efficiency_trend =
            self.create_trend_metric(cache_efficiency_today, cache_efficiency_yesterday);

        // Overall cost efficiency score
        let recent_efficiency = self.calculate_period_efficiency(week_ago, now);
        let cost_efficiency_score = (recent_efficiency / 100_000.0 * 100.0).min(100.0);

        EfficiencyTrends {
            tokens_per_dollar_trend,
            response_time_trend,
            cache_efficiency_trend,
            cost_efficiency_score,
        }
    }

    /// Generate alerts based on usage patterns
    fn generate_alerts(
        &self,
        burn_rates: &BurnRateAnalysis,
        budget_projections: &BudgetProjections,
        efficiency_trends: &EfficiencyTrends,
    ) -> Vec<UsageAlert> {
        let mut alerts = Vec::new();
        let now = Utc::now();

        // Budget threshold alerts
        if let Some(daily_limit) = self.budget_config.daily_limit {
            let utilization = budget_projections.daily_projection.utilization_percentage / 100.0;
            if utilization >= self.budget_config.alert_threshold {
                alerts.push(UsageAlert {
                    alert_type: AlertType::BudgetThreshold,
                    severity: if utilization >= 1.0 {
                        AlertSeverity::Critical
                    } else {
                        AlertSeverity::Warning
                    },
                    message: format!(
                        "Daily budget utilization at {:.1}% (${:.2} of ${:.2})",
                        utilization * 100.0,
                        budget_projections.daily_projection.estimated_cost,
                        daily_limit
                    ),
                    timestamp: now,
                    recommended_action: Some(
                        "Consider reducing usage or adjusting daily budget".to_string(),
                    ),
                });
            }
        }

        // High burn rate alert
        if burn_rates.current_hour.cost_per_hour > 10.0 {
            alerts.push(UsageAlert {
                alert_type: AlertType::HighBurnRate,
                severity: AlertSeverity::Warning,
                message: format!(
                    "High burn rate detected: ${:.2}/hour ({} tokens/hour)",
                    burn_rates.current_hour.cost_per_hour,
                    burn_rates.current_hour.tokens_per_hour as u64
                ),
                timestamp: now,
                recommended_action: Some(
                    "Review current session activity for optimization opportunities".to_string(),
                ),
            });
        }

        // Unusual spike detection
        if burn_rates.current_hour.trend_percentage > 100.0 {
            alerts.push(UsageAlert {
                alert_type: AlertType::UnusualSpike,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Usage spike detected: {:.1}% increase in burn rate",
                    burn_rates.current_hour.trend_percentage
                ),
                timestamp: now,
                recommended_action: Some(
                    "Check for runaway processes or inefficient queries".to_string(),
                ),
            });
        }

        // Inefficient usage alert
        if efficiency_trends.cost_efficiency_score < 50.0 {
            alerts.push(UsageAlert {
                alert_type: AlertType::IneffientUsage,
                severity: AlertSeverity::Info,
                message: format!(
                    "Low efficiency score: {:.1}/100. Consider optimizing token usage",
                    efficiency_trends.cost_efficiency_score
                ),
                timestamp: now,
                recommended_action: Some(
                    "Review prompts for conciseness and leverage caching".to_string(),
                ),
            });
        }

        // Projection warnings
        if budget_projections.monthly_projection.will_exceed {
            alerts.push(UsageAlert {
                alert_type: AlertType::ProjectionWarning,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Monthly budget projection exceeds limit by ${:.2}",
                    -budget_projections.monthly_projection.margin
                ),
                timestamp: now,
                recommended_action: Some(
                    "Adjust usage patterns to stay within monthly budget".to_string(),
                ),
            });
        }

        alerts
    }

    /// Helper functions
    fn empty_burn_rate(&self) -> BurnRateMetrics {
        BurnRateMetrics {
            tokens_per_hour: 0.0,
            cost_per_hour: 0.0,
            projected_daily_tokens: 0,
            projected_daily_cost: 0.0,
            projected_monthly_tokens: 0,
            projected_monthly_cost: 0.0,
            trend_percentage: 0.0,
            hours_until_budget_limit: None,
        }
    }

    fn find_peak_burn_rate(&self) -> PeakBurnRate {
        let now = Utc::now();
        let mut hourly_rates: HashMap<DateTime<Utc>, (u64, f64)> = HashMap::new();

        // Group by hour
        for (usage, timestamp) in self.session_usage.values() {
            let hour_start = timestamp
                .date_naive()
                .and_hms_opt(timestamp.hour(), 0, 0)
                .unwrap();
            let hour_start = DateTime::from_naive_utc_and_offset(hour_start, Utc);
            let entry = hourly_rates.entry(hour_start).or_insert((0, 0.0));
            entry.0 += usage.total_tokens();
            entry.1 += usage.total_cost;
        }

        // Find peak
        let (peak_time, (peak_tokens, peak_cost)) = hourly_rates
            .iter()
            .max_by(|a, b| a.1.1.partial_cmp(&b.1.1).unwrap())
            .unwrap_or((&now, &(0, 0.0)));

        PeakBurnRate {
            tokens_per_hour: *peak_tokens as f64,
            cost_per_hour: *peak_cost,
            occurred_at: *peak_time,
        }
    }

    fn calculate_daily_efficiency(&self, date: NaiveDate) -> f64 {
        if let Some(usage) = self.daily_usage.get(&date) {
            usage.tokens_per_dollar()
        } else {
            0.0
        }
    }

    fn calculate_response_time_metric(&self, date: NaiveDate) -> f64 {
        if let Some(usage) = self.daily_usage.get(&date) {
            // Higher output/input ratio suggests longer responses (inverse for metric)
            let ratio = usage.output_input_ratio();
            if ratio > 0.0 { 100.0 / ratio } else { 100.0 }
        } else {
            0.0
        }
    }

    fn calculate_cache_efficiency(&self, date: NaiveDate) -> f64 {
        if let Some(usage) = self.daily_usage.get(&date) {
            usage.cache_efficiency()
        } else {
            0.0
        }
    }

    fn calculate_period_efficiency(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        let (total_tokens, total_cost) = self
            .daily_usage
            .iter()
            .filter(|(date, _)| **date >= start && **date <= end)
            .fold((0u64, 0.0), |(tokens, cost), (_, usage)| {
                (tokens + usage.total_tokens(), cost + usage.total_cost)
            });

        if total_cost > 0.0 {
            total_tokens as f64 / total_cost
        } else {
            0.0
        }
    }

    fn create_trend_metric(&self, current: f64, previous: f64) -> TrendMetric {
        let change_percentage = if previous > 0.0 {
            ((current - previous) / previous) * 100.0
        } else {
            0.0
        };

        let direction = if change_percentage > 5.0 {
            TrendDirection::Increasing
        } else if change_percentage < -5.0 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        TrendMetric {
            current_value: current,
            previous_value: previous,
            change_percentage,
            direction,
        }
    }
}

/// Format real-time analytics for display
pub fn format_realtime_analytics(report: &RealtimeAnalyticsReport) -> String {
    let mut output = String::new();

    // Burn Rates Section
    output.push_str("🔥 Burn Rates\n");
    output.push_str("─────────────\n");
    output.push_str(&format!(
        "Current Hour: {} tok/hr (${:.4}/hr) {} {:.1}%\n",
        format_number(report.burn_rates.current_hour.tokens_per_hour as u64),
        report.burn_rates.current_hour.cost_per_hour,
        trend_arrow(report.burn_rates.current_hour.trend_percentage),
        report.burn_rates.current_hour.trend_percentage.abs()
    ));
    output.push_str(&format!(
        "Per Minute: {} tok/min (${:.4}/min)\n",
        format_number(report.burn_rates.tokens_per_minute as u64),
        report.burn_rates.cost_per_minute
    ));
    output.push_str(&format!(
        "24-Hour Avg: {} tok/hr (${:.4}/hr)\n",
        format_number(report.burn_rates.last_24_hours.tokens_per_hour as u64),
        report.burn_rates.last_24_hours.cost_per_hour
    ));
    output.push_str(&format!(
        "Peak Rate: ${:.2}/hr at {}\n\n",
        report.burn_rates.peak_burn_rate.cost_per_hour,
        report.burn_rates.peak_burn_rate.occurred_at.format("%H:%M")
    ));

    // Budget Projections Section
    output.push_str("💰 Budget Projections\n");
    output.push_str("────────────────────\n");
    output.push_str(&format_budget_projection(
        "Daily",
        &report.budget_projections.daily_projection,
    ));
    output.push_str(&format_budget_projection(
        "Monthly",
        &report.budget_projections.monthly_projection,
    ));
    output.push_str(&format_budget_projection(
        "Yearly",
        &report.budget_projections.yearly_projection,
    ));

    // Time to Limits
    if let Some(hours) = report
        .budget_projections
        .time_to_limits
        .hours_to_daily_limit
    {
        output.push_str(&format!("⏱️  Daily limit in: {:.1} hours\n", hours));
    }
    if let Some(days) = report
        .budget_projections
        .time_to_limits
        .days_to_monthly_limit
    {
        output.push_str(&format!("⏱️  Monthly limit in: {:.1} days\n", days));
    }
    output.push('\n');

    // Session Analytics Section
    output.push_str("📊 Session Analytics\n");
    output.push_str("───────────────────\n");
    output.push_str(&format!(
        "Active Sessions: {}\n",
        report.session_metrics.active_session_count
    ));
    output.push_str(&format!(
        "Avg per Session: {} tokens (${:.4})\n",
        format_number(report.session_metrics.avg_tokens_per_session as u64),
        report.session_metrics.avg_cost_per_session
    ));
    output.push_str(&format!(
        "Avg Duration: {}\n",
        format_duration(&report.session_metrics.avg_session_duration)
    ));
    output.push_str(&format!(
        "Peak Hours: {:?}\n",
        report.session_metrics.peak_usage_hours
    ));
    output.push_str(&format!(
        "Efficiency Score: {:.1}/100\n\n",
        report.session_metrics.efficiency_score
    ));

    // Efficiency Trends Section
    output.push_str("📈 Efficiency Trends\n");
    output.push_str("───────────────────\n");
    output.push_str(&format_trend(
        "Tokens/Dollar",
        &report.efficiency_trends.tokens_per_dollar_trend,
    ));
    output.push_str(&format_trend(
        "Response Time",
        &report.efficiency_trends.response_time_trend,
    ));
    output.push_str(&format_trend(
        "Cache Usage",
        &report.efficiency_trends.cache_efficiency_trend,
    ));
    output.push('\n');

    // Alerts Section
    if !report.alerts.is_empty() {
        output.push_str("⚠️  Alerts\n");
        output.push_str("─────────\n");
        for alert in &report.alerts {
            let icon = match alert.severity {
                AlertSeverity::Critical => "🚨",
                AlertSeverity::Warning => "⚠️",
                AlertSeverity::Info => "ℹ️",
            };
            output.push_str(&format!("{} {}\n", icon, alert.message));
            if let Some(action) = &alert.recommended_action {
                output.push_str(&format!("   → {}\n", action));
            }
        }
    }

    output
}

/// Helper formatting functions
fn format_budget_projection(label: &str, projection: &BudgetProjection) -> String {
    if let Some(limit) = projection.budget_limit {
        let status = if projection.will_exceed { "❌" } else { "✅" };
        format!(
            "{}: ${:.2} / ${:.2} ({:.1}%) {}\n",
            label, projection.estimated_cost, limit, projection.utilization_percentage, status
        )
    } else {
        format!(
            "{}: ${:.2} (no limit set)\n",
            label, projection.estimated_cost
        )
    }
}

fn format_trend(label: &str, trend: &TrendMetric) -> String {
    format!(
        "{}: {:.2} {} {:.1}%\n",
        label,
        trend.current_value,
        trend_arrow(trend.change_percentage),
        trend.change_percentage.abs()
    )
}

fn trend_arrow(percentage: f64) -> &'static str {
    if percentage > 0.0 {
        "↑"
    } else if percentage < 0.0 {
        "↓"
    } else {
        "→"
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_projection_calculation() {
        let daily_map = HashMap::new();
        let session_map = HashMap::new();
        let analytics = RealtimeAnalytics::new(&daily_map, &session_map, BudgetConfig::default());

        let projection = analytics.calculate_budget_projection(75.0, Some(100.0));
        assert_eq!(projection.utilization_percentage, 75.0);
        assert!(!projection.will_exceed);
        assert_eq!(projection.margin, 25.0);

        let over_budget = analytics.calculate_budget_projection(120.0, Some(100.0));
        assert!(over_budget.will_exceed);
        assert_eq!(over_budget.margin, -20.0);
    }

    #[test]
    fn test_trend_metric_creation() {
        let daily_map = HashMap::new();
        let session_map = HashMap::new();
        let analytics = RealtimeAnalytics::new(&daily_map, &session_map, BudgetConfig::default());

        let trend = analytics.create_trend_metric(110.0, 100.0);
        assert_eq!(trend.change_percentage, 10.0);
        assert_eq!(trend.direction, TrendDirection::Increasing);

        let down_trend = analytics.create_trend_metric(90.0, 100.0);
        assert_eq!(down_trend.change_percentage, -10.0);
        assert_eq!(down_trend.direction, TrendDirection::Decreasing);
    }
}
