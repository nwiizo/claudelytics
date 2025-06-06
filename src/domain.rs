use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// NewType パターンで型安全性を向上
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TokenCount(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Cost(pub f64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

// Display implementations for better formatting
impl fmt::Display for TokenCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.4}", self.0)
    }
}

impl fmt::Display for ModelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Token usage breakdown with strong typing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: TokenCount,
    pub output_tokens: TokenCount,
    pub cache_creation_tokens: TokenCount,
    pub cache_read_tokens: TokenCount,
}

#[allow(dead_code)]
impl TokenUsage {
    pub fn new(input: u64, output: u64, cache_creation: u64, cache_read: u64) -> Self {
        Self {
            input_tokens: TokenCount(input),
            output_tokens: TokenCount(output),
            cache_creation_tokens: TokenCount(cache_creation),
            cache_read_tokens: TokenCount(cache_read),
        }
    }

    pub fn total(&self) -> TokenCount {
        TokenCount(
            self.input_tokens.0
                + self.output_tokens.0
                + self.cache_creation_tokens.0
                + self.cache_read_tokens.0,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.total().0 == 0
    }
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

// ドメインイベント: 使用状況の記録
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct UsageEvent {
    pub timestamp: DateTime<Utc>,
    pub model: Option<ModelName>,
    pub token_usage: TokenUsage,
    pub session_id: SessionId,
    pub cost: Option<Cost>,
}

#[allow(dead_code)]
impl UsageEvent {
    pub fn new(timestamp: DateTime<Utc>, token_usage: TokenUsage, session_id: SessionId) -> Self {
        Self {
            timestamp,
            model: None,
            token_usage,
            session_id,
            cost: None,
        }
    }

    pub fn with_model(mut self, model: ModelName) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self
    }

    pub fn date(&self) -> NaiveDate {
        self.timestamp.date_naive()
    }
}

// 集計結果のドメインオブジェクト
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub token_usage: TokenUsage,
    pub total_cost: Cost,
}

#[allow(dead_code)]
impl UsageMetrics {
    pub fn new(token_usage: TokenUsage, cost: Cost) -> Self {
        Self {
            token_usage,
            total_cost: cost,
        }
    }

    pub fn add(&mut self, other: &UsageMetrics) {
        self.token_usage.input_tokens.0 += other.token_usage.input_tokens.0;
        self.token_usage.output_tokens.0 += other.token_usage.output_tokens.0;
        self.token_usage.cache_creation_tokens.0 += other.token_usage.cache_creation_tokens.0;
        self.token_usage.cache_read_tokens.0 += other.token_usage.cache_read_tokens.0;
        self.total_cost.0 += other.total_cost.0;
    }

    pub fn input_output_ratio(&self) -> f64 {
        if self.token_usage.input_tokens.0 == 0 {
            return 0.0;
        }
        self.token_usage.output_tokens.0 as f64 / self.token_usage.input_tokens.0 as f64
    }
}

impl Default for UsageMetrics {
    fn default() -> Self {
        Self::new(TokenUsage::default(), Cost(0.0))
    }
}

// 日次レポート用のドメインオブジェクト
#[derive(Debug, Clone, Serialize)]
pub struct DailyUsageReport {
    pub date: NaiveDate,
    pub metrics: UsageMetrics,
    pub session_count: usize,
}

#[allow(dead_code)]
impl DailyUsageReport {
    pub fn new(date: NaiveDate, metrics: UsageMetrics, session_count: usize) -> Self {
        Self {
            date,
            metrics,
            session_count,
        }
    }
}

// セッションレポート用のドメインオブジェクト
#[derive(Debug, Clone, Serialize)]
pub struct SessionUsageReport {
    pub session_id: SessionId,
    pub metrics: UsageMetrics,
    pub last_activity: DateTime<Utc>,
    pub model: Option<ModelName>,
}

#[allow(dead_code)]
impl SessionUsageReport {
    pub fn new(session_id: SessionId, metrics: UsageMetrics, last_activity: DateTime<Utc>) -> Self {
        Self {
            session_id,
            metrics,
            last_activity,
            model: None,
        }
    }

    pub fn with_model(mut self, model: ModelName) -> Self {
        self.model = Some(model);
        self
    }
}

// 価格情報のドメインオブジェクト
#[derive(Debug, Clone, PartialEq)]
pub struct PricingModel {
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub cache_creation_cost_per_token: f64,
    pub cache_read_cost_per_token: f64,
}

impl PricingModel {
    pub fn calculate_cost(&self, usage: &TokenUsage) -> Cost {
        let total = usage.input_tokens.0 as f64 * self.input_cost_per_token
            + usage.output_tokens.0 as f64 * self.output_cost_per_token
            + usage.cache_creation_tokens.0 as f64 * self.cache_creation_cost_per_token
            + usage.cache_read_tokens.0 as f64 * self.cache_read_cost_per_token;

        Cost(total)
    }
}

// 価格計算の戦略インターフェース
pub trait CostCalculator {
    fn calculate_cost(&self, model: &ModelName, usage: &TokenUsage) -> Option<Cost>;
}
