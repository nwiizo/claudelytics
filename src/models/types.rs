use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Type aliases for better code readability
/// Map of date to daily token usage aggregation
pub type DailyUsageMap = HashMap<NaiveDate, TokenUsage>;
/// Map of session path to (usage, last_activity_time)
pub type SessionUsageMap = HashMap<String, (TokenUsage, DateTime<Utc>)>;

/// Core data structures for Claude Code usage analysis
/// Main structure representing a single usage record from JSONL files
#[derive(Debug, Deserialize)]
pub struct UsageRecord {
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub message: Option<MessageData>,
    #[serde(rename = "costUSD", default)]
    pub cost_usd: Option<f64>,
    /// Request ID for deduplication (paired with message.id)
    #[serde(rename = "requestId", default)]
    pub request_id: Option<String>,
}

/// Message data containing usage information and model details
#[derive(Debug, Deserialize)]
pub struct MessageData {
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub model: Option<String>,
    /// Message ID for deduplication (paired with requestId)
    #[serde(default)]
    pub id: Option<String>,
}

/// Token usage breakdown from API response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    /// Speed mode: "standard" or "fast" (fast mode uses 6x pricing)
    #[serde(default)]
    pub speed: Option<String>,
}

impl Usage {
    /// Check if this usage record is from fast mode
    pub fn is_fast_mode(&self) -> bool {
        self.speed.as_deref() == Some("fast")
    }
}

/// Aggregated token usage with cost calculation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_cost: f64,
    /// Cost attributed to fast mode (6x multiplier) usage
    #[serde(default)]
    pub fast_mode_cost: f64,
}

impl TokenUsage {
    /// Calculate total tokens following ccusage methodology
    /// Includes input, output, cache creation, and cache read tokens
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }

    /// Add another TokenUsage to this one
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_tokens += other.cache_creation_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.total_cost += other.total_cost;
        self.fast_mode_cost += other.fast_mode_cost;
    }

    /// Calculate efficiency metrics
    #[allow(dead_code)]
    pub fn tokens_per_dollar(&self) -> f64 {
        if self.total_cost > 0.0 {
            self.total_tokens() as f64 / self.total_cost
        } else {
            0.0
        }
    }

    /// Calculate output to input ratio for efficiency analysis
    #[allow(dead_code)]
    pub fn output_input_ratio(&self) -> f64 {
        if self.input_tokens > 0 {
            self.output_tokens as f64 / self.input_tokens as f64
        } else {
            0.0
        }
    }

    /// Calculate cache efficiency percentage (cache hits vs cache hits + input)
    #[allow(dead_code)]
    pub fn cache_efficiency(&self) -> f64 {
        if (self.cache_read_tokens + self.input_tokens) > 0 {
            self.cache_read_tokens as f64 / (self.cache_read_tokens + self.input_tokens) as f64
                * 100.0
        } else {
            0.0
        }
    }
}

impl UsageRecord {
    pub fn get_model_name(&self) -> Option<&str> {
        self.message.as_ref()?.model.as_deref()
    }

    /// Create a unique hash for deduplication (matching ccusage behavior).
    /// Returns None if either message.id or requestId is missing,
    /// in which case the record is never deduplicated.
    pub fn dedup_hash(&self) -> Option<String> {
        let message_id = self.message.as_ref()?.id.as_deref()?;
        let request_id = self.request_id.as_deref()?;
        Some(format!("{}:{}", message_id, request_id))
    }
}

impl From<&UsageRecord> for TokenUsage {
    fn from(record: &UsageRecord) -> Self {
        let usage = record.message.as_ref().and_then(|m| m.usage.as_ref());

        match usage {
            Some(u) => TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                cache_creation_tokens: u.cache_creation_input_tokens,
                cache_read_tokens: u.cache_read_input_tokens,
                total_cost: 0.0, // Cost is set by apply_cost_mode
                fast_mode_cost: 0.0,
            },
            None => TokenUsage::default(),
        }
    }
}
