use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use serde::Serialize;
use std::collections::HashMap;

use crate::models::TokenUsage;

/// Claude uses 5-hour billing blocks for usage tracking
/// This aligns with how Claude actually bills users
const BILLING_BLOCK_HOURS: i64 = 5;

/// Represents a 5-hour billing block with usage data
#[derive(Debug, Clone, Serialize)]
pub struct BillingBlock {
    /// Start time of the billing block
    pub start_time: DateTime<Utc>,
    /// End time of the billing block (start_time + 5 hours)
    pub end_time: DateTime<Utc>,
    /// Token usage within this block
    pub usage: TokenUsage,
    /// Number of sessions active in this block
    pub session_count: usize,
}

impl BillingBlock {
    /// Create a new billing block starting at the given time
    pub fn new(start_time: DateTime<Utc>) -> Self {
        let block_start = Self::normalize_to_block_start(start_time);
        Self {
            start_time: block_start,
            end_time: block_start + Duration::hours(BILLING_BLOCK_HOURS),
            usage: TokenUsage::default(),
            session_count: 0,
        }
    }

    /// Normalize a timestamp to the start of its billing block
    /// Billing blocks start at 00:00, 05:00, 10:00, 15:00, 20:00 UTC
    pub fn normalize_to_block_start(time: DateTime<Utc>) -> DateTime<Utc> {
        let hour = time.hour() as i64;
        let block_hour = (hour / BILLING_BLOCK_HOURS) * BILLING_BLOCK_HOURS;

        time.date_naive()
            .and_hms_opt(block_hour as u32, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap()
    }

    /// Check if a timestamp falls within this billing block
    pub fn contains(&self, time: DateTime<Utc>) -> bool {
        time >= self.start_time && time < self.end_time
    }

    /// Get a human-readable label for this block (e.g., "00:00-05:00")
    pub fn label(&self) -> String {
        format!(
            "{:02}:00-{:02}:00",
            self.start_time.hour(),
            self.end_time.hour()
        )
    }

    /// Get the block index for the day (0-4)
    #[allow(dead_code)]
    pub fn block_index(&self) -> usize {
        (self.start_time.hour() as usize) / (BILLING_BLOCK_HOURS as usize)
    }
}

/// Manages billing blocks and aggregates usage data
#[derive(Debug, Clone, Default)]
pub struct BillingBlockManager {
    /// Map of date to billing blocks for that day
    blocks: HashMap<NaiveDate, Vec<BillingBlock>>,
}

impl BillingBlockManager {
    /// Create a new billing block manager
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    /// Add usage data to the appropriate billing block
    pub fn add_usage(
        &mut self,
        timestamp: DateTime<Utc>,
        usage: &TokenUsage,
        session_id: Option<&str>,
    ) {
        let date = timestamp.date_naive();
        let _block_start = BillingBlock::normalize_to_block_start(timestamp);

        // Ensure we have blocks for this date
        let day_blocks = self.blocks.entry(date).or_insert_with(|| {
            let mut blocks = Vec::new();
            for i in 0..5 {
                let start = date
                    .and_hms_opt(i * BILLING_BLOCK_HOURS as u32, 0, 0)
                    .unwrap()
                    .and_local_timezone(Utc)
                    .unwrap();
                blocks.push(BillingBlock::new(start));
            }
            blocks
        });

        // Find the correct block and add usage
        for block in day_blocks.iter_mut() {
            if block.contains(timestamp) {
                block.usage.add(usage);
                if session_id.is_some() {
                    block.session_count += 1;
                }
                break;
            }
        }
    }

    /// Get billing blocks for a specific date
    #[allow(dead_code)]
    pub fn get_blocks_for_date(&self, date: NaiveDate) -> Option<&Vec<BillingBlock>> {
        self.blocks.get(&date)
    }

    /// Get all billing blocks sorted by date and block index
    pub fn get_all_blocks(&self) -> Vec<(NaiveDate, &BillingBlock)> {
        let mut all_blocks = Vec::new();

        for (date, blocks) in &self.blocks {
            for block in blocks {
                all_blocks.push((*date, block));
            }
        }

        all_blocks.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.start_time.cmp(&b.1.start_time))
        });

        all_blocks
    }

    /// Get current active billing block
    #[allow(dead_code)]
    pub fn get_current_block(&self) -> Option<&BillingBlock> {
        let now = Utc::now();
        let today = now.date_naive();

        self.blocks
            .get(&today)?
            .iter()
            .find(|block| block.contains(now))
    }

    /// Get billing blocks with usage (non-empty blocks)
    pub fn get_blocks_with_usage(&self) -> Vec<(NaiveDate, &BillingBlock)> {
        self.get_all_blocks()
            .into_iter()
            .filter(|(_, block)| block.usage.total_tokens() > 0)
            .collect()
    }

    /// Calculate total usage across all billing blocks
    pub fn total_usage(&self) -> TokenUsage {
        let mut total = TokenUsage::default();

        for blocks in self.blocks.values() {
            for block in blocks {
                total.add(&block.usage);
            }
        }

        total
    }

    /// Get usage summary by billing block time (aggregated across days)
    /// Returns a map of block time (e.g., "00:00-05:00") to total usage
    pub fn usage_by_block_time(&self) -> HashMap<String, TokenUsage> {
        let mut usage_by_time = HashMap::new();

        for blocks in self.blocks.values() {
            for block in blocks {
                let label = block.label();
                usage_by_time
                    .entry(label)
                    .or_insert_with(TokenUsage::default)
                    .add(&block.usage);
            }
        }

        usage_by_time
    }

    /// Get peak usage billing block
    pub fn peak_usage_block(&self) -> Option<(NaiveDate, &BillingBlock)> {
        self.get_blocks_with_usage()
            .into_iter()
            .max_by(|a, b| a.1.usage.total_tokens().cmp(&b.1.usage.total_tokens()))
    }

    /// Calculate average usage per billing block (excluding empty blocks)
    pub fn average_usage_per_block(&self) -> TokenUsage {
        let blocks_with_usage = self.get_blocks_with_usage();
        if blocks_with_usage.is_empty() {
            return TokenUsage::default();
        }

        let total = self.total_usage();
        let count = blocks_with_usage.len() as f64;

        TokenUsage {
            input_tokens: (total.input_tokens as f64 / count) as u64,
            output_tokens: (total.output_tokens as f64 / count) as u64,
            cache_creation_tokens: (total.cache_creation_tokens as f64 / count) as u64,
            cache_read_tokens: (total.cache_read_tokens as f64 / count) as u64,
            total_cost: total.total_cost / count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BillingBlockReport {
    /// Summary of all billing blocks
    pub blocks: Vec<BillingBlockSummary>,
    /// Total usage across all blocks
    pub total_usage: TokenUsage,
    /// Peak usage block
    pub peak_block: Option<BillingBlockSummary>,
    /// Average usage per active block
    pub average_per_block: TokenUsage,
    /// Usage pattern by time of day
    pub usage_by_time: HashMap<String, TokenUsage>,
}

#[derive(Debug, Serialize, Clone)]
pub struct BillingBlockSummary {
    pub date: String,
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    #[serde(flatten)]
    pub usage: TokenUsage,
    pub session_count: usize,
}

impl From<(NaiveDate, &BillingBlock)> for BillingBlockSummary {
    fn from((date, block): (NaiveDate, &BillingBlock)) -> Self {
        Self {
            date: date.format("%Y-%m-%d").to_string(),
            time_range: block.label(),
            start_time: block.start_time.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            end_time: block.end_time.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            usage: block.usage.clone(),
            session_count: block.session_count,
        }
    }
}

impl BillingBlockManager {
    /// Generate a comprehensive billing block report
    pub fn generate_report(&self) -> BillingBlockReport {
        let blocks_with_usage = self.get_blocks_with_usage();
        let total_usage = self.total_usage();
        let peak_block = self
            .peak_usage_block()
            .map(|(date, block)| (date, block).into());
        let average_per_block = self.average_usage_per_block();
        let usage_by_time = self.usage_by_block_time();

        let blocks: Vec<BillingBlockSummary> = blocks_with_usage
            .into_iter()
            .map(|(date, block)| (date, block).into())
            .collect();

        BillingBlockReport {
            blocks,
            total_usage,
            peak_block,
            average_per_block,
            usage_by_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_billing_block_normalization() {
        // Test various times normalize to correct block starts
        let test_cases = vec![
            ("2024-01-01T00:30:00Z", "2024-01-01T00:00:00Z"),
            ("2024-01-01T04:59:59Z", "2024-01-01T00:00:00Z"),
            ("2024-01-01T05:00:00Z", "2024-01-01T05:00:00Z"),
            ("2024-01-01T09:30:00Z", "2024-01-01T05:00:00Z"),
            ("2024-01-01T10:00:00Z", "2024-01-01T10:00:00Z"),
            ("2024-01-01T14:45:00Z", "2024-01-01T10:00:00Z"),
            ("2024-01-01T15:00:00Z", "2024-01-01T15:00:00Z"),
            ("2024-01-01T19:59:59Z", "2024-01-01T15:00:00Z"),
            ("2024-01-01T20:00:00Z", "2024-01-01T20:00:00Z"),
            ("2024-01-01T23:59:59Z", "2024-01-01T20:00:00Z"),
        ];

        for (input, expected) in test_cases {
            let time = DateTime::parse_from_rfc3339(input)
                .unwrap()
                .with_timezone(&Utc);
            let expected_time = DateTime::parse_from_rfc3339(expected)
                .unwrap()
                .with_timezone(&Utc);
            let normalized = BillingBlock::normalize_to_block_start(time);
            assert_eq!(normalized, expected_time, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_billing_block_contains() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let block = BillingBlock::new(start);

        // Test times within the block
        assert!(block.contains(Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()));
        assert!(block.contains(Utc.with_ymd_and_hms(2024, 1, 1, 12, 30, 0).unwrap()));
        assert!(block.contains(Utc.with_ymd_and_hms(2024, 1, 1, 14, 59, 59).unwrap()));

        // Test times outside the block
        assert!(!block.contains(Utc.with_ymd_and_hms(2024, 1, 1, 9, 59, 59).unwrap()));
        assert!(!block.contains(Utc.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap()));
    }

    #[test]
    fn test_billing_block_manager() {
        let mut manager = BillingBlockManager::new();

        // Add some usage data
        let time1 = Utc.with_ymd_and_hms(2024, 1, 1, 2, 30, 0).unwrap();
        let time2 = Utc.with_ymd_and_hms(2024, 1, 1, 7, 45, 0).unwrap();
        let time3 = Utc.with_ymd_and_hms(2024, 1, 1, 7, 50, 0).unwrap();

        let usage1 = TokenUsage {
            input_tokens: 100,
            output_tokens: 200,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.001,
        };

        let usage2 = TokenUsage {
            input_tokens: 150,
            output_tokens: 250,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.002,
        };

        manager.add_usage(time1, &usage1, Some("session1"));
        manager.add_usage(time2, &usage2, Some("session2"));
        manager.add_usage(time3, &usage2, Some("session3"));

        // Check that blocks were created correctly
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let blocks = manager.get_blocks_for_date(date).unwrap();
        assert_eq!(blocks.len(), 5);

        // Check first block (00:00-05:00) has usage1
        assert_eq!(blocks[0].usage.input_tokens, 100);
        assert_eq!(blocks[0].usage.output_tokens, 200);
        assert_eq!(blocks[0].session_count, 1);

        // Check second block (05:00-10:00) has usage2 twice
        assert_eq!(blocks[1].usage.input_tokens, 300);
        assert_eq!(blocks[1].usage.output_tokens, 500);
        assert_eq!(blocks[1].session_count, 2);

        // Check total usage
        let total = manager.total_usage();
        assert_eq!(total.input_tokens, 400);
        assert_eq!(total.output_tokens, 700);
        assert_eq!(total.total_cost, 0.005);
    }
}
