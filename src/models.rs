use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct UsageRecord {
    pub timestamp: DateTime<Utc>,
    pub message: MessageData,
    #[serde(rename = "costUSD")]
    pub cost_usd: Option<f64>,
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

#[derive(Debug, Clone, Default, Serialize)]
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

    /// Calculate cost based on Claude Opus 4 pricing (2025)
    /// - Input: $15 per million tokens
    /// - Output: $75 per million tokens  
    /// - Cache creation: $18.75 per million tokens
    /// - Cache read: $1.50 per million tokens
    pub fn calculate_cost(&self) -> f64 {
        const PRICE_PER_MILLION: f64 = 1_000_000.0;
        const INPUT_PRICE: f64 = 15.0;
        const OUTPUT_PRICE: f64 = 75.0;
        const CACHE_CREATION_PRICE: f64 = 18.75;
        const CACHE_READ_PRICE: f64 = 1.50;

        let input_cost = (self.input_tokens as f64 / PRICE_PER_MILLION) * INPUT_PRICE;
        let output_cost = (self.output_tokens as f64 / PRICE_PER_MILLION) * OUTPUT_PRICE;
        let cache_creation_cost =
            (self.cache_creation_tokens as f64 / PRICE_PER_MILLION) * CACHE_CREATION_PRICE;
        let cache_read_cost =
            (self.cache_read_tokens as f64 / PRICE_PER_MILLION) * CACHE_READ_PRICE;

        input_cost + output_cost + cache_creation_cost + cache_read_cost
    }
}

impl From<&UsageRecord> for TokenUsage {
    fn from(record: &UsageRecord) -> Self {
        let usage = TokenUsage {
            input_tokens: record.message.usage.input_tokens,
            output_tokens: record.message.usage.output_tokens,
            cache_creation_tokens: record.message.usage.cache_creation_input_tokens,
            cache_read_tokens: record.message.usage.cache_read_input_tokens,
            total_cost: 0.0, // Will be set below
        };

        // Use provided cost if available, otherwise calculate from tokens
        let cost = match record.cost_usd {
            Some(cost) => cost,
            None => usage.calculate_cost(),
        };

        TokenUsage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_creation_tokens: usage.cache_creation_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            total_cost: cost,
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

#[derive(Debug, Clone)]
pub struct SessionDetail {
    pub project_path: String,
    pub session_id: String,
    pub session_path: PathBuf,
    pub usage: TokenUsage,
    pub last_activity: DateTime<Utc>,
    pub message_count: usize,
    pub first_activity: DateTime<Utc>,
    pub duration_hours: f64,
    pub project_name: String,
}

impl Serialize for SessionDetail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SessionDetail", 8)?;
        state.serialize_field("project_path", &self.project_path)?;
        state.serialize_field("session_id", &self.session_id)?;
        state.serialize_field("session_path", &self.session_path.to_string_lossy())?;
        state.serialize_field("usage", &self.usage)?;
        state.serialize_field("last_activity", &self.last_activity)?;
        state.serialize_field("message_count", &self.message_count)?;
        state.serialize_field("first_activity", &self.first_activity)?;
        state.serialize_field("duration_hours", &self.duration_hours)?;
        state.serialize_field("project_name", &self.project_name)?;
        state.end()
    }
}

impl SessionDetail {}

#[derive(Debug, Serialize)]
pub struct DetailedSessionReport {
    pub sessions: Vec<SessionDetail>,
    pub totals: TokenUsageTotals,
}

pub type DailyUsageMap = HashMap<NaiveDate, TokenUsage>;
pub type SessionUsageMap = HashMap<String, (TokenUsage, DateTime<Utc>)>;

// Advanced TUI features - Message-level analysis
#[derive(Debug, Clone, Serialize)]
pub struct MessageDetail {
    pub timestamp: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost: f64,
    pub duration_ms: Option<u64>,
    pub efficiency_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetailedSession {
    pub session_detail: SessionDetail,
    pub messages: Vec<MessageDetail>,
    pub hourly_breakdown: HashMap<u32, TokenUsage>, // hour -> usage
    pub efficiency_metrics: EfficiencyMetrics,
    pub bookmarked: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EfficiencyMetrics {
    pub tokens_per_dollar: f64,
    pub output_input_ratio: f64,
    pub cache_efficiency: f64,
    pub cost_per_message: f64,
    pub peak_hour: u32,
    pub activity_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonReport {
    pub title: String,
    pub sessions: Vec<SessionComparison>,
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionComparison {
    pub name: String,
    pub usage: TokenUsage,
    pub efficiency: EfficiencyMetrics,
    pub relative_performance: f64, // -1.0 to 1.0
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonSummary {
    pub best_efficiency: String,
    pub highest_cost: String,
    pub most_active: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub user_stats: UserBenchmark,
    pub session_rankings: Vec<SessionRanking>,
    pub trends: TrendAnalysis,
    pub recommendations: Vec<OptimizationTip>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserBenchmark {
    pub total_efficiency: f64,
    pub cost_efficiency_percentile: f64,
    pub usage_consistency: f64,
    pub peak_performance_day: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRanking {
    pub session_name: String,
    pub score: f64,
    pub rank: usize,
    pub category: String, // "efficiency", "cost", "volume"
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendAnalysis {
    pub cost_trend: Vec<f64>, // last 30 days
    pub efficiency_trend: Vec<f64>,
    pub volume_trend: Vec<f64>,
    pub prediction_next_week: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizationTip {
    pub category: String,
    pub title: String,
    pub description: String,
    pub potential_savings: f64,
    pub priority: String, // "high", "medium", "low"
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveMetrics {
    pub active_sessions: u32,
    pub current_cost_rate: f64, // per hour
    pub real_time_efficiency: f64,
    pub last_update: DateTime<Utc>,
    pub activity_sparkline: Vec<u32>, // last 24 hours
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapData {
    pub hour_of_day: HashMap<u32, f64>,  // 0-23 -> intensity
    pub day_of_week: HashMap<u32, f64>,  // 0-6 -> intensity
    pub day_of_month: HashMap<u32, f64>, // 1-31 -> intensity
}

// Command palette actions
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
// ADVANCED ANALYTICS - Pattern Analysis & Data Mining
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct UsagePattern {
    pub pattern_id: String,
    pub name: String,
    pub description: String,
    pub frequency: f64,
    pub avg_tokens_per_session: f64,
    pub avg_cost_per_session: f64,
    pub time_of_day_preference: Vec<f64>, // 24 hours
    pub day_of_week_preference: Vec<f64>, // 7 days
    pub typical_session_duration: f64,    // hours
    pub projects_involved: Vec<String>,
    pub efficiency_score: f64,
    pub predictability: f64, // how consistent this pattern is
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternAnalysis {
    pub detected_patterns: Vec<UsagePattern>,
    pub primary_pattern: String,
    pub pattern_stability: f64, // how consistent patterns are over time
    pub anomalies: Vec<UsageAnomaly>,
    pub clustering_confidence: f64,
    pub recommendations: Vec<PatternRecommendation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageAnomaly {
    pub timestamp: DateTime<Utc>,
    pub anomaly_type: String, // "cost_spike", "unusual_pattern", "efficiency_drop"
    pub severity: f64,        // 0.0 to 1.0
    pub description: String,
    pub expected_value: f64,
    pub actual_value: f64,
    pub potential_causes: Vec<String>,
    pub impact_assessment: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternRecommendation {
    pub title: String,
    pub description: String,
    pub pattern_context: String,
    pub expected_improvement: f64,
    pub implementation_difficulty: String, // "easy", "moderate", "complex"
    pub confidence: f64,
}

// ============================================================================
// PRODUCTIVITY ANALYTICS
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ProductivityMetrics {
    pub overall_score: f64,
    pub deep_work_sessions: Vec<DeepWorkSession>,
    pub context_switches: Vec<ContextSwitch>,
    pub focus_periods: Vec<FocusPeriod>,
    pub break_patterns: BreakPattern,
    pub productivity_trends: ProductivityTrend,
    pub efficiency_landscape: EfficiencyLandscape,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeepWorkSession {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_hours: f64,
    pub total_tokens: u64,
    pub projects_involved: Vec<String>,
    pub interruption_count: u32,
    pub focus_quality: f64,  // 0.0 to 1.0
    pub output_quality: f64, // tokens per minute, efficiency metrics
    pub cost_efficiency: f64,
    pub flow_state_indicator: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextSwitch {
    pub timestamp: DateTime<Utc>,
    pub from_project: String,
    pub to_project: String,
    pub switch_cost: f64,      // estimated productivity loss
    pub recovery_time: f64,    // time to regain focus
    pub switch_reason: String, // "scheduled", "interruption", "completion"
}

#[derive(Debug, Clone, Serialize)]
pub struct FocusPeriod {
    pub start_time: DateTime<Utc>,
    pub duration_minutes: f64,
    pub intensity: f64,   // tokens per minute
    pub consistency: f64, // how steady the pace was
    pub project: String,
    pub quality_indicators: FocusQuality,
}

#[derive(Debug, Clone, Serialize)]
pub struct FocusQuality {
    pub input_output_ratio: f64,
    pub cache_utilization: f64,
    pub response_time_consistency: f64,
    pub token_efficiency: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreakPattern {
    pub avg_break_duration: f64,        // minutes
    pub break_frequency: f64,           // breaks per hour
    pub optimal_break_timing: Vec<f64>, // hours from start
    pub break_impact_on_productivity: f64,
    pub recovery_curve: Vec<f64>, // productivity recovery after breaks
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductivityTrend {
    pub daily_productivity_curve: Vec<f64>,    // 24 hours
    pub weekly_productivity_pattern: Vec<f64>, // 7 days
    pub monthly_trends: Vec<f64>,
    pub seasonal_adjustments: HashMap<String, f64>,
    pub peak_performance_times: Vec<TimeRange>,
    pub low_energy_periods: Vec<TimeRange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeRange {
    pub start_hour: u8,
    pub end_hour: u8,
    pub days_of_week: Vec<u8>, // 0=Monday, 6=Sunday
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EfficiencyLandscape {
    pub cost_vs_output_correlation: f64,
    pub time_vs_efficiency_curve: Vec<(f64, f64)>, // (time_of_day, efficiency)
    pub project_efficiency_matrix: HashMap<String, f64>,
    pub token_type_efficiency: TokenTypeEfficiency,
    pub optimization_opportunities: Vec<EfficiencyOpportunity>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenTypeEfficiency {
    pub input_efficiency: f64,  // value per input token
    pub output_efficiency: f64, // quality per output token
    pub cache_roi: f64,         // return on cache investment
    pub optimal_ratios: OptimalRatios,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimalRatios {
    pub input_output_ratio: f64,
    pub cache_to_total_ratio: f64,
    pub session_length_sweet_spot: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EfficiencyOpportunity {
    pub opportunity_type: String,
    pub description: String,
    pub potential_savings: f64, // dollars or tokens
    pub implementation_effort: String,
    pub confidence: f64,
    pub time_to_implement: String,
}

// ============================================================================
// PREDICTIVE ANALYTICS & FORECASTING
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct PredictiveAnalytics {
    pub cost_forecasting: CostForecast,
    pub usage_prediction: UsagePrediction,
    pub trend_analysis: AdvancedTrendAnalysis,
    pub budget_tracking: BudgetTracker,
    pub capacity_planning: CapacityPlanning,
    pub risk_assessment: RiskAssessment,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostForecast {
    pub next_week: ForecastPeriod,
    pub next_month: ForecastPeriod,
    pub next_quarter: ForecastPeriod,
    pub confidence_intervals: HashMap<String, (f64, f64)>, // period -> (low, high)
    pub seasonal_adjustments: HashMap<String, f64>,
    pub growth_rate: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastPeriod {
    pub predicted_cost: f64,
    pub predicted_tokens: u64,
    pub confidence: f64,
    pub factors_considered: Vec<String>,
    pub model_accuracy: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsagePrediction {
    pub peak_usage_times: Vec<PeakPrediction>,
    pub low_usage_periods: Vec<LowUsagePrediction>,
    pub project_growth_rates: HashMap<String, f64>,
    pub usage_velocity: f64,           // rate of change in usage
    pub saturation_point: Option<f64>, // predicted usage plateau
}

#[derive(Debug, Clone, Serialize)]
pub struct PeakPrediction {
    pub predicted_time: DateTime<Utc>,
    pub predicted_intensity: f64,
    pub duration_estimate: f64,
    pub contributing_factors: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LowUsagePrediction {
    pub predicted_period: (DateTime<Utc>, DateTime<Utc>),
    pub intensity_drop: f64,
    pub reasons: Vec<String>,
    pub optimization_opportunity: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdvancedTrendAnalysis {
    pub micro_trends: Vec<MicroTrend>, // short-term patterns
    pub macro_trends: Vec<MacroTrend>, // long-term patterns
    pub cyclical_patterns: Vec<CyclicalPattern>,
    pub trend_strength: f64,
    pub trend_consistency: f64,
    pub turning_points: Vec<TurningPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MicroTrend {
    pub timeframe: String, // "hourly", "daily"
    pub direction: String, // "increasing", "decreasing", "stable"
    pub magnitude: f64,
    pub duration: f64,
    pub significance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MacroTrend {
    pub timeframe: String,  // "weekly", "monthly", "quarterly"
    pub trend_type: String, // "linear", "exponential", "logarithmic"
    pub growth_rate: f64,
    pub r_squared: f64, // goodness of fit
    pub extrapolation_reliability: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CyclicalPattern {
    pub cycle_length: f64, // in days
    pub amplitude: f64,
    pub phase_offset: f64,
    pub strength: f64,
    pub next_peak: DateTime<Utc>,
    pub next_trough: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TurningPoint {
    pub timestamp: DateTime<Utc>,
    pub point_type: String, // "peak", "trough", "inflection"
    pub magnitude: f64,
    pub confidence: f64,
    pub contributing_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BudgetTracker {
    pub monthly_budget: f64,
    pub current_spending: f64,
    pub projected_spending: f64,
    pub budget_utilization: f64, // percentage
    pub burn_rate: f64,          // daily spending rate
    pub days_remaining_in_budget: f64,
    pub spending_alerts: Vec<BudgetAlert>,
    pub optimization_suggestions: Vec<BudgetOptimization>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BudgetAlert {
    pub alert_type: String, // "approaching_limit", "overspend", "unusual_spike"
    pub severity: String,   // "low", "medium", "high"
    pub message: String,
    pub threshold_crossed: f64,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BudgetOptimization {
    pub strategy: String,
    pub potential_savings: f64,
    pub implementation_complexity: String,
    pub impact_on_productivity: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapacityPlanning {
    pub current_capacity_utilization: f64,
    pub projected_capacity_needs: Vec<CapacityNeed>,
    pub scaling_recommendations: Vec<ScalingRecommendation>,
    pub bottleneck_analysis: BottleneckAnalysis,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapacityNeed {
    pub timeframe: String,
    pub projected_usage: f64,
    pub confidence: f64,
    pub growth_drivers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScalingRecommendation {
    pub recommendation_type: String,
    pub timeline: String,
    pub expected_benefit: f64,
    pub cost_impact: f64,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BottleneckAnalysis {
    pub identified_bottlenecks: Vec<Bottleneck>,
    pub capacity_constraints: Vec<CapacityConstraint>,
    pub optimization_priorities: Vec<OptimizationPriority>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bottleneck {
    pub bottleneck_type: String,
    pub impact_level: String,
    pub description: String,
    pub resolution_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapacityConstraint {
    pub constraint_type: String,
    pub current_limit: f64,
    pub utilization: f64,
    pub time_to_exhaustion: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizationPriority {
    pub priority_level: String,
    pub area: String,
    pub expected_impact: f64,
    pub effort_required: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskAssessment {
    pub identified_risks: Vec<UsageRisk>,
    pub overall_risk_score: f64,
    pub mitigation_strategies: Vec<RiskMitigation>,
    pub risk_tolerance: RiskTolerance,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageRisk {
    pub risk_type: String, // "cost_overrun", "usage_spike", "efficiency_decline"
    pub probability: f64,
    pub impact_severity: f64,
    pub risk_score: f64,
    pub description: String,
    pub early_warning_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskMitigation {
    pub risk_addressed: String,
    pub mitigation_strategy: String,
    pub effectiveness: f64,
    pub implementation_cost: f64,
    pub timeline: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskTolerance {
    pub cost_variance_tolerance: f64,
    pub usage_spike_tolerance: f64,
    pub efficiency_decline_tolerance: f64,
    pub alert_thresholds: HashMap<String, f64>,
}

// ============================================================================
// ADVANCED FILTERING & SEARCH
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct AdvancedFilter {
    pub filter_id: String,
    pub name: String,
    pub description: String,
    pub criteria: FilterCriteria,
    pub is_saved: bool,
    pub usage_count: u32,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilterCriteria {
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub cost_range: Option<(f64, f64)>,
    pub token_range: Option<(u64, u64)>,
    pub projects: Option<Vec<String>>,
    pub efficiency_threshold: Option<f64>,
    pub session_duration_range: Option<(f64, f64)>,
    pub usage_patterns: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub regex_patterns: Option<Vec<String>>,
    pub anomaly_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmartSuggestion {
    pub suggestion_type: String, // "filter", "search", "analysis"
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub context: String,
    pub action_data: serde_json::Value,
}

// ============================================================================
// WORKFLOW INTEGRATION & GIT CORRELATION
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowIntegration {
    pub git_correlation: Option<GitCorrelation>,
    pub project_milestones: Vec<ProjectMilestone>,
    pub development_cycle_analysis: DevelopmentCycleAnalysis,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitCorrelation {
    pub commit_patterns: Vec<CommitPattern>,
    pub branch_usage_correlation: HashMap<String, f64>,
    pub commit_cost_correlation: f64,
    pub peak_development_times: Vec<TimeRange>,
    pub code_complexity_vs_usage: Vec<(f64, f64)>, // complexity, usage
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitPattern {
    pub commit_hash: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub files_changed: u32,
    pub lines_added: u32,
    pub lines_removed: u32,
    pub usage_activity_before: f64, // hours before commit
    pub usage_activity_after: f64,  // hours after commit
    pub correlation_strength: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectMilestone {
    pub milestone_id: String,
    pub name: String,
    pub target_date: DateTime<Utc>,
    pub completion_date: Option<DateTime<Utc>>,
    pub associated_usage: TokenUsage,
    pub efficiency_during_milestone: f64,
    pub progress_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevelopmentCycleAnalysis {
    pub phases: Vec<DevelopmentPhase>,
    pub cycle_efficiency: f64,
    pub bottlenecks: Vec<String>,
    pub optimization_opportunities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevelopmentPhase {
    pub phase_name: String,
    pub duration: f64, // hours
    pub usage_characteristics: UsageCharacteristics,
    pub efficiency_metrics: PhaseEfficiency,
    pub typical_activities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageCharacteristics {
    pub avg_session_length: f64,
    pub token_intensity: f64,
    pub cost_per_hour: f64,
    pub context_switch_frequency: f64,
    pub deep_work_ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhaseEfficiency {
    pub output_per_input: f64,
    pub time_efficiency: f64,
    pub cost_efficiency: f64,
    pub quality_indicators: Vec<String>,
    pub improvement_suggestions: Vec<String>,
}

// ============================================================================
// MACHINE LEARNING INSIGHTS
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct MLInsights {
    pub usage_clustering: UsageClustering,
    pub predictive_models: Vec<PredictiveModel>,
    pub automated_insights: Vec<AutomatedInsight>,
    pub model_performance: ModelPerformance,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageClustering {
    pub clusters: Vec<UsageCluster>,
    pub optimal_cluster_count: usize,
    pub silhouette_score: f64,
    pub cluster_stability: f64,
    pub feature_importance: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageCluster {
    pub cluster_id: usize,
    pub name: String,
    pub description: String,
    pub centroid: Vec<f64>,
    pub member_count: usize,
    pub characteristics: ClusterCharacteristics,
    pub representative_sessions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterCharacteristics {
    pub avg_cost: f64,
    pub avg_tokens: u64,
    pub avg_duration: f64,
    pub primary_time_of_day: u8,
    pub primary_day_of_week: u8,
    pub efficiency_score: f64,
    pub predictability: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredictiveModel {
    pub model_type: String, // "cost_prediction", "usage_forecast", "anomaly_detection"
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub feature_names: Vec<String>,
    pub feature_importance: Vec<f64>,
    pub training_samples: usize,
    pub last_trained: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutomatedInsight {
    pub insight_type: String,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub evidence: Vec<Evidence>,
    pub recommendations: Vec<String>,
    pub impact_assessment: ImpactAssessment,
    pub automated_reasoning: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evidence {
    pub evidence_type: String,
    pub data_points: Vec<f64>,
    pub statistical_significance: f64,
    pub correlation_strength: f64,
    pub supporting_context: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactAssessment {
    pub cost_impact: f64,
    pub efficiency_impact: f64,
    pub productivity_impact: f64,
    pub risk_level: String,
    pub implementation_effort: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPerformance {
    pub overall_accuracy: f64,
    pub prediction_confidence: f64,
    pub model_drift_indicator: f64,
    pub last_validation: DateTime<Utc>,
    pub performance_trends: Vec<f64>,
}

// ============================================================================
// INTERACTIVE ANALYSIS & DATA EXPLORATION
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct InteractiveAnalysis {
    pub timeline_data: TimelineData,
    pub correlation_matrix: CorrelationMatrix,
    pub drill_down_paths: Vec<DrillDownPath>,
    pub dynamic_filters: Vec<AdvancedFilter>,
    pub exploration_history: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineData {
    pub events: Vec<TimelineEvent>,
    pub time_buckets: HashMap<String, Vec<f64>>, // bucket_type -> values
    pub trends: HashMap<String, f64>,            // metric -> trend_strength
    pub seasonal_patterns: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub description: String,
    pub impact_magnitude: f64,
    pub related_metrics: HashMap<String, f64>,
    pub context: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorrelationMatrix {
    pub variables: Vec<String>,
    pub correlations: Vec<Vec<f64>>, // NxN matrix
    pub strong_correlations: Vec<StrongCorrelation>,
    pub statistical_significance: Vec<Vec<f64>>,
    pub interpretation: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrongCorrelation {
    pub variable1: String,
    pub variable2: String,
    pub correlation_coefficient: f64,
    pub p_value: f64,
    pub interpretation: String,
    pub practical_significance: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DrillDownPath {
    pub path_id: String,
    pub steps: Vec<DrillDownStep>,
    pub current_step: usize,
    pub breadcrumbs: Vec<String>,
    pub available_dimensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DrillDownStep {
    pub step_name: String,
    pub dimension: String,
    pub filter_value: String,
    pub result_count: usize,
    pub aggregated_metrics: HashMap<String, f64>,
    pub next_possible_steps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,          // 1M tokens = $15
            output_tokens: 1_000_000,         // 1M tokens = $75
            cache_creation_tokens: 1_000_000, // 1M tokens = $18.75
            cache_read_tokens: 1_000_000,     // 1M tokens = $1.50
            total_cost: 0.0,
        };

        let calculated_cost = usage.calculate_cost();
        assert_eq!(calculated_cost, 110.25); // 15 + 75 + 18.75 + 1.50
    }

    #[test]
    fn test_calculate_cost_fractional() {
        let usage = TokenUsage {
            input_tokens: 129_470,  // 0.129470M * $15 = $1.94205
            output_tokens: 973_692, // 0.973692M * $75 = $73.0269
            cache_creation_tokens: 0,
            cache_read_tokens: 194_403_239, // 194.403239M * $1.50 = $291.6048585
            total_cost: 0.0,
        };

        let calculated_cost = usage.calculate_cost();
        let expected = 1.94205 + 73.0269 + 291.6048585;
        assert!((calculated_cost - expected).abs() < 0.0001);
    }

    #[test]
    fn test_from_usage_record_with_cost() {
        let record = UsageRecord {
            timestamp: chrono::Utc::now(),
            message: MessageData {
                usage: Usage {
                    input_tokens: 100,
                    output_tokens: 200,
                    cache_creation_input_tokens: 50,
                    cache_read_input_tokens: 150,
                },
            },
            cost_usd: Some(5.5),
        };

        let usage = TokenUsage::from(&record);
        assert_eq!(usage.total_cost, 5.5); // Should use provided cost
    }

    #[test]
    fn test_from_usage_record_without_cost() {
        let record = UsageRecord {
            timestamp: chrono::Utc::now(),
            message: MessageData {
                usage: Usage {
                    input_tokens: 1_000_000,
                    output_tokens: 1_000_000,
                    cache_creation_input_tokens: 1_000_000,
                    cache_read_input_tokens: 1_000_000,
                },
            },
            cost_usd: None,
        };

        let usage = TokenUsage::from(&record);
        assert_eq!(usage.total_cost, 110.25); // Should calculate cost
    }
}
