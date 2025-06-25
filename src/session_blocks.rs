use chrono::{DateTime, Duration, Timelike, Utc};
use serde::Serialize;
use std::collections::HashMap;

use crate::models::TokenUsage;

/// Configurable session block for analyzing usage patterns
#[derive(Debug, Clone, Serialize)]
pub struct SessionBlock {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub usage: TokenUsage,
    pub session_count: usize,
    pub is_active: bool,
    pub burn_rate: Option<BurnRate>,
}

/// Burn rate calculations for active sessions
#[derive(Debug, Clone, Serialize)]
pub struct BurnRate {
    pub tokens_per_hour: f64,
    pub cost_per_hour: f64,
    pub projected_daily_tokens: u64,
    pub projected_daily_cost: f64,
    pub projected_monthly_cost: f64,
    pub time_to_limit: Option<Duration>,
}

/// Session blocks configuration
#[derive(Debug, Clone, Serialize)]
pub struct SessionBlockConfig {
    pub block_hours: i64,
    pub token_limit: Option<u64>,
    pub cost_limit: Option<f64>,
}

impl Default for SessionBlockConfig {
    fn default() -> Self {
        Self {
            block_hours: 8, // Default 8-hour blocks like ccusage
            token_limit: None,
            cost_limit: None,
        }
    }
}

/// Manager for session-based time blocks
pub struct SessionBlockManager {
    blocks: HashMap<String, Vec<SessionBlock>>,
    config: SessionBlockConfig,
}

impl SessionBlockManager {
    pub fn new(config: SessionBlockConfig) -> Self {
        Self {
            blocks: HashMap::new(),
            config,
        }
    }

    /// Add usage record to appropriate session block
    pub fn add_usage(&mut self, timestamp: DateTime<Utc>, usage: &TokenUsage, _session_id: &str) {
        let block_start = self.normalize_to_block(timestamp);
        let block_end = block_start + Duration::hours(self.config.block_hours);
        let block_key = format!(
            "{}-{}",
            block_start.format("%Y%m%d%H"),
            self.config.block_hours
        );

        let is_active = self.is_block_active(block_start, block_end);
        let blocks = self.blocks.entry(block_key.clone()).or_default();

        if let Some(block) = blocks.iter_mut().find(|b| b.start_time == block_start) {
            block.usage.add(usage);
            block.session_count += 1;
        } else {
            blocks.push(SessionBlock {
                start_time: block_start,
                end_time: block_end,
                usage: usage.clone(),
                session_count: 1,
                is_active,
                burn_rate: None,
            });
        }
    }

    /// Normalize timestamp to block boundary
    fn normalize_to_block(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        let hours_since_midnight = timestamp.hour() as i64;
        let block_number = hours_since_midnight / self.config.block_hours;
        let block_hour = block_number * self.config.block_hours;

        timestamp
            .date_naive()
            .and_hms_opt(block_hour as u32, 0, 0)
            .unwrap()
            .and_utc()
    }

    /// Check if block is currently active
    fn is_block_active(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        let now = Utc::now();
        start <= now && now < end
    }

    /// Calculate burn rate for active blocks
    pub fn calculate_burn_rates(&mut self) {
        let now = Utc::now();
        let token_limit = self.config.token_limit;
        let cost_limit = self.config.cost_limit;

        for blocks in self.blocks.values_mut() {
            for block in blocks.iter_mut() {
                if block.is_active {
                    let elapsed = now - block.start_time;
                    let hours_elapsed = elapsed.num_seconds() as f64 / 3600.0;

                    if hours_elapsed > 0.1 {
                        // At least 6 minutes
                        let tokens_per_hour = block.usage.total_tokens() as f64 / hours_elapsed;
                        let cost_per_hour = block.usage.total_cost / hours_elapsed;

                        let projected_daily_tokens = (tokens_per_hour * 24.0) as u64;
                        let projected_daily_cost = cost_per_hour * 24.0;
                        let projected_monthly_cost = projected_daily_cost * 30.0;

                        let time_to_limit = Self::calculate_time_to_limit_static(
                            tokens_per_hour,
                            cost_per_hour,
                            block.usage.total_tokens(),
                            block.usage.total_cost,
                            token_limit,
                            cost_limit,
                        );

                        block.burn_rate = Some(BurnRate {
                            tokens_per_hour,
                            cost_per_hour,
                            projected_daily_tokens,
                            projected_daily_cost,
                            projected_monthly_cost,
                            time_to_limit,
                        });
                    }
                }
            }
        }
    }

    /// Calculate time remaining until limit is reached (static version)
    fn calculate_time_to_limit_static(
        tokens_per_hour: f64,
        cost_per_hour: f64,
        current_tokens: u64,
        current_cost: f64,
        token_limit: Option<u64>,
        cost_limit: Option<f64>,
    ) -> Option<Duration> {
        if let Some(token_limit) = token_limit {
            let remaining_tokens = token_limit.saturating_sub(current_tokens);
            let hours_to_token_limit = remaining_tokens as f64 / tokens_per_hour;

            if let Some(cost_limit) = cost_limit {
                let remaining_cost = cost_limit - current_cost;
                let hours_to_cost_limit = remaining_cost / cost_per_hour;

                // Return whichever limit will be hit first
                let hours = hours_to_token_limit.min(hours_to_cost_limit);
                if hours > 0.0 {
                    return Some(Duration::seconds((hours * 3600.0) as i64));
                }
            } else if hours_to_token_limit > 0.0 {
                return Some(Duration::seconds((hours_to_token_limit * 3600.0) as i64));
            }
        } else if let Some(cost_limit) = cost_limit {
            let remaining_cost = cost_limit - current_cost;
            let hours_to_cost_limit = remaining_cost / cost_per_hour;

            if hours_to_cost_limit > 0.0 {
                return Some(Duration::seconds((hours_to_cost_limit * 3600.0) as i64));
            }
        }

        None
    }

    /// Get all session blocks
    pub fn get_all_blocks(&self) -> Vec<&SessionBlock> {
        self.blocks
            .values()
            .flat_map(|blocks| blocks.iter())
            .collect()
    }

    /// Get active session blocks
    pub fn get_active_blocks(&self) -> Vec<&SessionBlock> {
        self.blocks
            .values()
            .flat_map(|blocks| blocks.iter())
            .filter(|block| block.is_active)
            .collect()
    }

    /// Get recent blocks (last N days)
    pub fn get_recent_blocks(&self, days: i64) -> Vec<&SessionBlock> {
        let cutoff = Utc::now() - Duration::days(days);

        self.blocks
            .values()
            .flat_map(|blocks| blocks.iter())
            .filter(|block| block.start_time > cutoff)
            .collect()
    }

    /// Generate session block report
    pub fn generate_report(&mut self) -> SessionBlockReport {
        self.calculate_burn_rates();

        let all_blocks = self.get_all_blocks();
        let active_blocks = self.get_active_blocks();
        let recent_blocks = self.get_recent_blocks(30);

        let total_usage = all_blocks
            .iter()
            .fold(TokenUsage::default(), |mut acc, block| {
                acc.add(&block.usage);
                acc
            });

        let active_usage = active_blocks
            .iter()
            .fold(TokenUsage::default(), |mut acc, block| {
                acc.add(&block.usage);
                acc
            });

        let current_burn_rate = active_blocks
            .iter()
            .filter_map(|block| block.burn_rate.as_ref())
            .next()
            .cloned();

        SessionBlockReport {
            config: SessionBlockConfig {
                block_hours: self.config.block_hours,
                token_limit: self.config.token_limit,
                cost_limit: self.config.cost_limit,
            },
            total_blocks: all_blocks.len(),
            active_blocks: active_blocks.len(),
            recent_blocks: recent_blocks.len(),
            total_usage,
            active_usage,
            current_burn_rate,
            blocks: all_blocks.into_iter().cloned().collect(),
        }
    }
}

/// Report for session block analysis
#[derive(Debug, Serialize)]
pub struct SessionBlockReport {
    pub config: SessionBlockConfig,
    pub total_blocks: usize,
    pub active_blocks: usize,
    pub recent_blocks: usize,
    pub total_usage: TokenUsage,
    pub active_usage: TokenUsage,
    pub current_burn_rate: Option<BurnRate>,
    pub blocks: Vec<SessionBlock>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_block_normalization() {
        let config = SessionBlockConfig {
            block_hours: 8,
            token_limit: None,
            cost_limit: None,
        };
        let manager = SessionBlockManager::new(config);

        // Test 8-hour blocks
        let test_cases = vec![
            ("2024-01-01T00:30:00Z", "2024-01-01T00:00:00Z"),
            ("2024-01-01T07:59:59Z", "2024-01-01T00:00:00Z"),
            ("2024-01-01T08:00:00Z", "2024-01-01T08:00:00Z"),
            ("2024-01-01T15:30:00Z", "2024-01-01T08:00:00Z"),
            ("2024-01-01T16:00:00Z", "2024-01-01T16:00:00Z"),
            ("2024-01-01T23:59:59Z", "2024-01-01T16:00:00Z"),
        ];

        for (input, expected) in test_cases {
            let timestamp = DateTime::parse_from_rfc3339(input)
                .unwrap()
                .with_timezone(&Utc);
            let expected_time = DateTime::parse_from_rfc3339(expected)
                .unwrap()
                .with_timezone(&Utc);
            let normalized = manager.normalize_to_block(timestamp);

            assert_eq!(normalized, expected_time, "Failed for input {}", input);
        }
    }

    #[test]
    fn test_burn_rate_calculation() {
        let config = SessionBlockConfig {
            block_hours: 8,
            token_limit: Some(100000),
            cost_limit: Some(10.0),
        };
        let mut manager = SessionBlockManager::new(config);

        // Add some usage
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 2000,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.15,
        };

        let now = Utc::now();
        manager.add_usage(now, &usage, "test-session");

        // Calculate burn rates
        manager.calculate_burn_rates();

        // Get active blocks
        let active_blocks = manager.get_active_blocks();
        assert_eq!(active_blocks.len(), 1);

        // Check that burn rate was calculated for active block
        let active_block = active_blocks[0];
        assert!(active_block.burn_rate.is_some());
    }
}
