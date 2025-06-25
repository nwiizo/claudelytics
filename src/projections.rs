use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;

use crate::models::DailyUsageMap;

/// Time series data point for projections
#[derive(Debug, Clone, Serialize)]
pub struct DataPoint {
    pub date: NaiveDate,
    pub value: f64,
}

/// Projection result with confidence intervals
#[derive(Debug, Clone, Serialize)]
pub struct Projection {
    pub date: NaiveDate,
    pub value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub confidence: f64,
}

/// Usage projection analysis
#[derive(Debug, Clone, Serialize)]
pub struct UsageProjection {
    pub daily_average: f64,
    pub weekly_average: f64,
    pub monthly_average: f64,
    pub trend: TrendDirection,
    pub growth_rate: f64,
    pub projections: Vec<Projection>,
    pub estimated_monthly_cost: f64,
    pub days_until_limit: Option<i64>,
    pub limit_date: Option<NaiveDate>,
}

/// Trend direction for usage patterns
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Projection calculator for usage forecasting
pub struct ProjectionCalculator {
    history_days: i64,
    projection_days: i64,
    token_limit: Option<u64>,
    cost_limit: Option<f64>,
}

impl ProjectionCalculator {
    pub fn new() -> Self {
        Self {
            history_days: 30,
            projection_days: 30,
            token_limit: None,
            cost_limit: None,
        }
    }

    pub fn with_limits(mut self, token_limit: Option<u64>, cost_limit: Option<f64>) -> Self {
        self.token_limit = token_limit;
        self.cost_limit = cost_limit;
        self
    }

    pub fn with_projection_days(mut self, days: i64) -> Self {
        self.projection_days = days;
        self
    }

    /// Calculate usage projections from daily data
    pub fn calculate_projections(&self, daily_usage: &DailyUsageMap) -> UsageProjection {
        let today = Utc::now().date_naive();
        let start_date = today - Duration::days(self.history_days);

        // Collect recent data points
        let mut data_points: Vec<DataPoint> = daily_usage
            .iter()
            .filter(|(date, _)| **date >= start_date)
            .map(|(date, usage)| DataPoint {
                date: *date,
                value: usage.total_cost,
            })
            .collect();

        data_points.sort_by_key(|p| p.date);

        // Calculate averages
        let daily_average = self.calculate_daily_average(&data_points);
        let weekly_average = self.calculate_weekly_average(&data_points);
        let monthly_average = self.calculate_monthly_average(&data_points);

        // Calculate trend and growth rate
        let (trend, growth_rate) = self.calculate_trend(&data_points);

        // Generate projections
        let projections = self.generate_projections(&data_points, growth_rate);

        // Calculate when limits will be reached
        let (days_until_limit, limit_date) =
            self.calculate_limit_timing(&data_points, daily_average, growth_rate);

        // Estimate monthly cost based on projections
        let estimated_monthly_cost = self.estimate_monthly_cost(&projections, daily_average);

        UsageProjection {
            daily_average,
            weekly_average,
            monthly_average,
            trend,
            growth_rate,
            projections,
            estimated_monthly_cost,
            days_until_limit,
            limit_date,
        }
    }

    /// Calculate daily average from recent data
    fn calculate_daily_average(&self, data_points: &[DataPoint]) -> f64 {
        if data_points.is_empty() {
            return 0.0;
        }

        let total: f64 = data_points.iter().map(|p| p.value).sum();
        total / data_points.len() as f64
    }

    /// Calculate weekly average
    fn calculate_weekly_average(&self, data_points: &[DataPoint]) -> f64 {
        if data_points.len() < 7 {
            return self.calculate_daily_average(data_points) * 7.0;
        }

        // Get last 7 days
        let last_week: Vec<&DataPoint> = data_points.iter().rev().take(7).collect();
        let weekly_total: f64 = last_week.iter().map(|p| p.value).sum();
        weekly_total
    }

    /// Calculate monthly average
    fn calculate_monthly_average(&self, data_points: &[DataPoint]) -> f64 {
        if data_points.len() < 30 {
            return self.calculate_daily_average(data_points) * 30.0;
        }

        // Get last 30 days
        let last_month: Vec<&DataPoint> = data_points.iter().rev().take(30).collect();
        let monthly_total: f64 = last_month.iter().map(|p| p.value).sum();
        monthly_total
    }

    /// Calculate trend direction and growth rate using linear regression
    fn calculate_trend(&self, data_points: &[DataPoint]) -> (TrendDirection, f64) {
        if data_points.len() < 3 {
            return (TrendDirection::Stable, 0.0);
        }

        // Simple linear regression
        let n = data_points.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (i, point) in data_points.iter().enumerate() {
            let x = i as f64;
            let y = point.value;

            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let average = sum_y / n;

        // Calculate growth rate as percentage
        let growth_rate = if average > 0.0 {
            (slope / average) * 100.0
        } else {
            0.0
        };

        let trend = if growth_rate > 5.0 {
            TrendDirection::Increasing
        } else if growth_rate < -5.0 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        (trend, growth_rate)
    }

    /// Generate future projections
    fn generate_projections(&self, data_points: &[DataPoint], growth_rate: f64) -> Vec<Projection> {
        if data_points.is_empty() {
            return vec![];
        }

        let mut projections = Vec::new();
        let last_date = data_points.last().unwrap().date;
        let last_value = data_points.last().unwrap().value;
        let daily_growth = 1.0 + (growth_rate / 100.0);

        // Calculate standard deviation for confidence intervals
        let avg = self.calculate_daily_average(data_points);
        let variance: f64 = data_points
            .iter()
            .map(|p| (p.value - avg).powi(2))
            .sum::<f64>()
            / data_points.len() as f64;
        let std_dev = variance.sqrt();

        for i in 1..=self.projection_days {
            let date = last_date + Duration::days(i);
            let value = last_value * daily_growth.powi(i as i32);

            // Confidence decreases as we project further into the future
            let confidence = 1.0 / (1.0 + (i as f64 * 0.03));

            // Wider bounds for further projections
            let bound_multiplier = 1.96 * (i as f64).sqrt(); // 95% confidence interval
            let lower_bound = (value - std_dev * bound_multiplier).max(0.0);
            let upper_bound = value + std_dev * bound_multiplier;

            projections.push(Projection {
                date,
                value,
                lower_bound,
                upper_bound,
                confidence,
            });
        }

        projections
    }

    /// Calculate when limits will be reached
    fn calculate_limit_timing(
        &self,
        data_points: &[DataPoint],
        daily_average: f64,
        growth_rate: f64,
    ) -> (Option<i64>, Option<NaiveDate>) {
        if self.cost_limit.is_none() || daily_average <= 0.0 {
            return (None, None);
        }

        let cost_limit = self.cost_limit.unwrap();
        let today = Utc::now().date_naive();

        // Calculate cumulative cost for current month
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let current_month_cost: f64 = data_points
            .iter()
            .filter(|p| p.date >= month_start)
            .map(|p| p.value)
            .sum();

        if current_month_cost >= cost_limit {
            return (Some(0), Some(today));
        }

        let remaining_budget = cost_limit - current_month_cost;
        let daily_burn = daily_average * (1.0 + growth_rate / 100.0);

        if daily_burn <= 0.0 {
            return (None, None);
        }

        let days_until_limit = (remaining_budget / daily_burn).ceil() as i64;
        let limit_date = today + Duration::days(days_until_limit);

        (Some(days_until_limit), Some(limit_date))
    }

    /// Estimate monthly cost based on projections
    fn estimate_monthly_cost(&self, projections: &[Projection], daily_average: f64) -> f64 {
        if projections.is_empty() {
            return daily_average * 30.0;
        }

        // Take next 30 days of projections
        let next_month: f64 = projections.iter().take(30).map(|p| p.value).sum();

        next_month
    }
}

/// Token-based projections (similar structure but for token counts)
#[derive(Debug, Clone, Serialize)]
pub struct TokenProjection {
    pub daily_average_tokens: u64,
    pub weekly_average_tokens: u64,
    pub monthly_average_tokens: u64,
    pub trend: TrendDirection,
    pub growth_rate: f64,
    pub days_until_token_limit: Option<i64>,
    pub token_limit_date: Option<NaiveDate>,
}

impl ProjectionCalculator {
    /// Calculate token-based projections
    pub fn calculate_token_projections(&self, daily_usage: &DailyUsageMap) -> TokenProjection {
        let today = Utc::now().date_naive();
        let start_date = today - Duration::days(self.history_days);

        // Collect token data points
        let mut token_points: Vec<(NaiveDate, u64)> = daily_usage
            .iter()
            .filter(|(date, _)| **date >= start_date)
            .map(|(date, usage)| (*date, usage.total_tokens()))
            .collect();

        token_points.sort_by_key(|(date, _)| *date);

        // Calculate averages
        let daily_average = if !token_points.is_empty() {
            let total: u64 = token_points.iter().map(|(_, tokens)| tokens).sum();
            total / token_points.len() as u64
        } else {
            0
        };

        let weekly_average = if token_points.len() >= 7 {
            token_points.iter().rev().take(7).map(|(_, t)| t).sum()
        } else {
            daily_average * 7
        };

        let monthly_average = if token_points.len() >= 30 {
            token_points.iter().rev().take(30).map(|(_, t)| t).sum()
        } else {
            daily_average * 30
        };

        // Calculate trend using float values
        let float_points: Vec<DataPoint> = token_points
            .iter()
            .map(|(date, tokens)| DataPoint {
                date: *date,
                value: *tokens as f64,
            })
            .collect();

        let (trend, growth_rate) = self.calculate_trend(&float_points);

        // Calculate token limit timing
        let (days_until_token_limit, token_limit_date) = if let Some(limit) = self.token_limit {
            let current_month_tokens: u64 = token_points
                .iter()
                .filter(|(date, _)| date.month() == today.month() && date.year() == today.year())
                .map(|(_, tokens)| tokens)
                .sum();

            if current_month_tokens >= limit {
                (Some(0), Some(today))
            } else {
                let remaining = limit - current_month_tokens;
                let daily_burn = (daily_average as f64 * (1.0 + growth_rate / 100.0)) as u64;

                if daily_burn > 0 {
                    let days = (remaining / daily_burn) as i64;
                    (Some(days), Some(today + Duration::days(days)))
                } else {
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        TokenProjection {
            daily_average_tokens: daily_average,
            weekly_average_tokens: weekly_average,
            monthly_average_tokens: monthly_average,
            trend,
            growth_rate,
            days_until_token_limit,
            token_limit_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TokenUsage;

    #[test]
    fn test_projection_calculation() {
        let mut daily_usage = DailyUsageMap::new();

        // Add some test data
        for i in 0..10 {
            let date = Utc::now().date_naive() - Duration::days(i);
            let usage = TokenUsage {
                input_tokens: 1000,
                output_tokens: 2000,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_cost: 0.15 + ((9 - i) as f64 * 0.01), // Increasing cost over time
            };
            daily_usage.insert(date, usage);
        }

        let calculator = ProjectionCalculator::new();
        let projection = calculator.calculate_projections(&daily_usage);

        assert!(projection.daily_average > 0.0);
        assert_eq!(projection.trend, TrendDirection::Increasing);
        assert!(!projection.projections.is_empty());
    }

    #[test]
    fn test_limit_calculation() {
        let mut daily_usage = DailyUsageMap::new();

        // Add consistent usage
        for i in 0..5 {
            let date = Utc::now().date_naive() - Duration::days(i);
            let usage = TokenUsage {
                input_tokens: 10000,
                output_tokens: 20000,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_cost: 1.0,
            };
            daily_usage.insert(date, usage);
        }

        let calculator = ProjectionCalculator::new().with_limits(Some(1000000), Some(50.0));

        let projection = calculator.calculate_projections(&daily_usage);

        // With $1/day and $50 limit, should have ~45 days left (5 days already used)
        assert!(projection.days_until_limit.is_some());
        assert!(projection.limit_date.is_some());
    }
}
