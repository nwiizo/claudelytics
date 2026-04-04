use chrono::NaiveDate;
use serde::Serialize;

use super::types::TokenUsage;

#[derive(Debug, Serialize, Clone)]
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

#[derive(Debug, Serialize, Clone)]
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

#[derive(Debug, Serialize, Clone)]
pub struct DailyReport {
    pub daily: Vec<DailyUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize, Clone)]
pub struct MonthlyUsage {
    pub month: String,
    pub year: u32,
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
    #[serde(rename = "daysActive")]
    pub days_active: u32,
    #[serde(rename = "avgDailyCost")]
    pub avg_daily_cost: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct MonthlyReport {
    pub monthly: Vec<MonthlyUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize, Clone)]
pub struct WeeklyUsage {
    #[serde(rename = "weekStart")]
    pub week_start: String,
    #[serde(rename = "weekEnd")]
    pub week_end: String,
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
    #[serde(rename = "daysActive")]
    pub days_active: u32,
    #[serde(rename = "avgDailyCost")]
    pub avg_daily_cost: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct WeeklyReport {
    pub weekly: Vec<WeeklyUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize, Clone)]
pub struct SessionReport {
    pub sessions: Vec<SessionUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize, Clone)]
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
