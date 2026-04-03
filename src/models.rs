use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

// ============================================================================
// TUI COMMAND PALETTE
// ============================================================================

// Command palette actions
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CommandAction {
    SwitchTab(usize),
    #[allow(dead_code)]
    ToggleSort(String),
    #[allow(dead_code)]
    SetFilter(String),
    #[allow(dead_code)]
    ExportData(String),
    #[allow(dead_code)]
    BookmarkSession(String),
    #[allow(dead_code)]
    CompareSelected,
    #[allow(dead_code)]
    ShowBenchmark,
    RefreshData,
    #[allow(dead_code)]
    OpenSessionDetail(String),
    ShowHelp,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub action: CommandAction,
    pub category: String,
}

// ============================================================================
// CLAUDE SESSION METADATA
// ============================================================================

/// Claude session metadata from first line of JSONL file
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ClaudeSessionSummary {
    #[serde(rename = "type")]
    pub record_type: String,
    pub summary: String,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: String,
}

/// Complete Claude session with metadata and messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClaudeSession {
    pub file_path: PathBuf,
    pub project_path: String,
    pub session_id: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub message_count: usize,
    pub usage: TokenUsage,
}

/// Claude conversation message
#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct ClaudeMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub timestamp: DateTime<Utc>,
    pub message: MessageContent,
    pub uuid: String,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// Message content within Claude message
#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct MessageContent {
    pub role: String,
    pub content: Vec<ContentPart>,
    pub usage: Option<Usage>,
}

/// Content part of message
#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}
