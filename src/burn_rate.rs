use crate::models::DailyUsageMap;
use chrono::{DateTime, Duration, Local, Utc};
use std::collections::HashMap;

/// Token burn rate calculator inspired by ccusage
pub struct BurnRateCalculator {
    daily_usage: DailyUsageMap,
}

/// Burn rate metrics and projections
#[derive(Debug, Clone, serde::Serialize)]
pub struct BurnRateMetrics {
    /// Current burn rate (tokens per hour)
    pub tokens_per_hour: f64,
    /// Current burn rate (cost per hour)
    pub cost_per_hour: f64,
    /// Projected daily tokens at current rate
    pub projected_daily_tokens: u64,
    /// Projected daily cost at current rate
    pub projected_daily_cost: f64,
    /// Projected monthly tokens at current rate
    pub projected_monthly_tokens: u64,
    /// Projected monthly cost at current rate
    pub projected_monthly_cost: f64,
    /// Trend direction (positive = increasing, negative = decreasing)
    pub trend_percentage: f64,
    /// Hours until projected budget limit (if set)
    #[allow(dead_code)]
    pub hours_until_budget_limit: Option<f64>,
}

/// Hourly usage data for burn rate calculation
#[derive(Debug, Clone, Default)]
pub struct HourlyUsage {
    pub tokens: u64,
    pub cost: f64,
    pub timestamp: DateTime<Utc>,
}

impl BurnRateCalculator {
    pub fn new(daily_usage: DailyUsageMap) -> Self {
        Self { daily_usage }
    }

    /// Calculate burn rate metrics based on recent usage
    pub fn calculate_burn_rate(&self, hours_lookback: i64) -> Option<BurnRateMetrics> {
        let now = Local::now();
        let lookback_time = now - Duration::hours(hours_lookback);

        // Get hourly usage data
        let hourly_data = self.get_hourly_usage(lookback_time);

        if hourly_data.is_empty() {
            return None;
        }

        // Calculate average burn rates
        let total_hours = hours_lookback as f64;
        let total_tokens: u64 = hourly_data.values().map(|h| h.tokens).sum();
        let total_cost: f64 = hourly_data.values().map(|h| h.cost).sum();

        let tokens_per_hour = total_tokens as f64 / total_hours;
        let cost_per_hour = total_cost / total_hours;

        // Calculate projections
        let projected_daily_tokens = (tokens_per_hour * 24.0) as u64;
        let projected_daily_cost = cost_per_hour * 24.0;
        let projected_monthly_tokens = (tokens_per_hour * 24.0 * 30.0) as u64;
        let projected_monthly_cost = projected_daily_cost * 30.0;

        // Calculate trend (compare recent vs overall average)
        let trend_percentage = self.calculate_trend(&hourly_data, hours_lookback);

        Some(BurnRateMetrics {
            tokens_per_hour,
            cost_per_hour,
            projected_daily_tokens,
            projected_daily_cost,
            projected_monthly_tokens,
            projected_monthly_cost,
            trend_percentage,
            hours_until_budget_limit: None, // Can be implemented with budget configuration
        })
    }

    /// Get hourly usage data from daily aggregates
    fn get_hourly_usage(&self, since: DateTime<Local>) -> HashMap<String, HourlyUsage> {
        let mut hourly_data = HashMap::new();
        let since_date = since.date_naive();

        // Distribute daily usage across active hours (assuming 8-10 hours of active usage per day)
        // This provides a more realistic burn rate than dividing by 24
        const ACTIVE_HOURS_PER_DAY: u64 = 9; // Average active hours

        for (date, usage) in &self.daily_usage {
            if *date >= since_date {
                let tokens_per_active_hour = usage.total_tokens() / ACTIVE_HOURS_PER_DAY;
                let cost_per_active_hour = usage.total_cost / ACTIVE_HOURS_PER_DAY as f64;

                // Distribute usage across typical working hours (9 AM to 6 PM)
                for hour in 9..18 {
                    let hour_key = format!("{}_h{:02}", date, hour);
                    hourly_data.insert(
                        hour_key,
                        HourlyUsage {
                            tokens: tokens_per_active_hour,
                            cost: cost_per_active_hour,
                            timestamp: DateTime::from_naive_utc_and_offset(
                                date.and_hms_opt(hour, 0, 0).unwrap(),
                                Utc,
                            ),
                        },
                    );
                }
            }
        }

        hourly_data
    }

    /// Calculate usage trend as percentage change
    fn calculate_trend(
        &self,
        hourly_data: &HashMap<String, HourlyUsage>,
        hours_lookback: i64,
    ) -> f64 {
        if hours_lookback <= 2 {
            return 0.0;
        }

        let _half_hours = hours_lookback / 2;
        let mut first_half_tokens = 0u64;
        let mut second_half_tokens = 0u64;
        let mut sorted_hours: Vec<_> = hourly_data.values().collect();
        sorted_hours.sort_by_key(|h| h.timestamp);

        let mid_point = sorted_hours.len() / 2;

        for (i, hour) in sorted_hours.iter().enumerate() {
            if i < mid_point {
                first_half_tokens += hour.tokens;
            } else {
                second_half_tokens += hour.tokens;
            }
        }

        if first_half_tokens == 0 {
            return 0.0;
        }

        ((second_half_tokens as f64 - first_half_tokens as f64) / first_half_tokens as f64) * 100.0
    }

    /// Get real-time burn rate for current session
    #[allow(dead_code)]
    pub fn get_session_burn_rate(
        &self,
        session_start: DateTime<Utc>,
        current_tokens: u64,
        current_cost: f64,
    ) -> BurnRateMetrics {
        let now = Utc::now();
        let duration = now - session_start;
        let hours_elapsed = duration.num_seconds() as f64 / 3600.0;

        if hours_elapsed <= 0.0 {
            return BurnRateMetrics {
                tokens_per_hour: 0.0,
                cost_per_hour: 0.0,
                projected_daily_tokens: 0,
                projected_daily_cost: 0.0,
                projected_monthly_tokens: 0,
                projected_monthly_cost: 0.0,
                trend_percentage: 0.0,
                hours_until_budget_limit: None,
            };
        }

        let tokens_per_hour = current_tokens as f64 / hours_elapsed;
        let cost_per_hour = current_cost / hours_elapsed;

        BurnRateMetrics {
            tokens_per_hour,
            cost_per_hour,
            projected_daily_tokens: (tokens_per_hour * 24.0) as u64,
            projected_daily_cost: cost_per_hour * 24.0,
            projected_monthly_tokens: (tokens_per_hour * 24.0 * 30.0) as u64,
            projected_monthly_cost: cost_per_hour * 24.0 * 30.0,
            trend_percentage: 0.0, // No trend for single session
            hours_until_budget_limit: None,
        }
    }
}

/// Format burn rate metrics for display
#[allow(dead_code)]
pub fn format_burn_rate(metrics: &BurnRateMetrics) -> String {
    let trend_arrow = if metrics.trend_percentage > 0.0 {
        "↑"
    } else if metrics.trend_percentage < 0.0 {
        "↓"
    } else {
        "→"
    };

    format!(
        "Burn Rate: {} tok/hr (${:.4}/hr) {} {:.1}%\n\
         Projected: {} tokens/day (${:.2}/day)\n\
         Monthly Projection: ${:.2}",
        format_number(metrics.tokens_per_hour as u64),
        metrics.cost_per_hour,
        trend_arrow,
        metrics.trend_percentage.abs(),
        format_number(metrics.projected_daily_tokens),
        metrics.projected_daily_cost,
        metrics.projected_monthly_cost
    )
}

/// Format number with commas
#[allow(dead_code)]
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
