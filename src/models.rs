use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct UsageRecord {
    pub timestamp: DateTime<Utc>,
    pub message: MessageData,
    #[serde(rename = "costUSD")]
    pub cost_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct MessageData {
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_cost: f64,
}

impl TokenUsage {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }
    
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_tokens += other.cache_creation_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.total_cost += other.total_cost;
    }
}

impl From<&UsageRecord> for TokenUsage {
    fn from(record: &UsageRecord) -> Self {
        TokenUsage {
            input_tokens: record.message.usage.input_tokens,
            output_tokens: record.message.usage.output_tokens,
            cache_creation_tokens: record.message.usage.cache_creation_input_tokens,
            cache_read_tokens: record.message.usage.cache_read_input_tokens,
            total_cost: record.cost_usd,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DailyUsage {
    pub date: String,
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,
    #[serde(rename = "cacheCreationTokens")]
    pub cache_creation_tokens: u64,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_tokens: u64,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u64,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
}

impl From<(NaiveDate, &TokenUsage)> for DailyUsage {
    fn from((date, usage): (NaiveDate, &TokenUsage)) -> Self {
        DailyUsage {
            date: date.format("%Y-%m-%d").to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_creation_tokens: usage.cache_creation_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            total_tokens: usage.total_tokens(),
            total_cost: usage.total_cost,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SessionUsage {
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,
    #[serde(rename = "cacheCreationTokens")]
    pub cache_creation_tokens: u64,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_tokens: u64,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u64,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
}

#[derive(Debug, Serialize)]
pub struct DailyReport {
    pub daily: Vec<DailyUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize)]
pub struct SessionReport {
    pub sessions: Vec<SessionUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize)]
pub struct TokenUsageTotals {
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,
    #[serde(rename = "cacheCreationTokens")]
    pub cache_creation_tokens: u64,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_tokens: u64,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u64,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
}

impl From<&TokenUsage> for TokenUsageTotals {
    fn from(usage: &TokenUsage) -> Self {
        TokenUsageTotals {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_creation_tokens: usage.cache_creation_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            total_tokens: usage.total_tokens(),
            total_cost: usage.total_cost,
        }
    }
}

pub type DailyUsageMap = HashMap<NaiveDate, TokenUsage>;
pub type SessionUsageMap = HashMap<String, (TokenUsage, DateTime<Utc>)>;