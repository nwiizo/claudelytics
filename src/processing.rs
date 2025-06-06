use crate::domain::*;
use crate::error::{ClaudelyticsError, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// JSONLファイルから読み込む生データ
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RawUsageRecord {
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub message: Option<RawMessageData>,
    #[serde(rename = "costUSD", default)]
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RawMessageData {
    #[serde(default)]
    pub usage: Option<RawUsage>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RawUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

/// ファイル処理を担当するコンポーネント
#[allow(dead_code)]
pub struct FileProcessor {
    claude_dir: PathBuf,
}

#[allow(dead_code)]
impl FileProcessor {
    pub fn new(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// すべてのJSONLファイルを見つける
    pub fn find_jsonl_files(&self) -> Result<Vec<PathBuf>> {
        let projects_dir = self.claude_dir.join("projects");

        if !projects_dir.exists() {
            return Err(ClaudelyticsError::directory_not_found(
                &self.claude_dir.display().to_string(),
            ));
        }

        let files: Vec<PathBuf> = WalkDir::new(projects_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();

        Ok(files)
    }

    /// ファイルからレコードのストリームを作成
    pub fn read_records(&self, file_path: &Path) -> Result<Vec<RawUsageRecord>> {
        let file = File::open(file_path).map_err(ClaudelyticsError::Io)?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        for (line_number, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<RawUsageRecord>(&line) {
                Ok(record) => records.push(record),
                Err(err) => {
                    // ログ出力のみで続行（元の動作を維持）
                    eprintln!(
                        "Warning: Failed to parse line {} in {}: {}",
                        line_number + 1,
                        file_path.display(),
                        err
                    );
                }
            }
        }

        Ok(records)
    }

    /// セッション情報を抽出
    pub fn extract_session_id(&self, file_path: &Path) -> Result<SessionId> {
        let projects_dir = self.claude_dir.join("projects");
        let relative_path = file_path.strip_prefix(&projects_dir).map_err(|_| {
            ClaudelyticsError::validation_error(
                "file_path",
                "File path is not within projects directory",
            )
        })?;

        let mut components: Vec<&str> = relative_path
            .components()
            .filter_map(|comp| comp.as_os_str().to_str())
            .collect();

        if components.is_empty() {
            return Err(ClaudelyticsError::validation_error(
                "file_path",
                "Invalid file path structure",
            ));
        }

        // ファイル名を削除してセッションディレクトリを取得
        components.pop();

        if components.is_empty() {
            return Err(ClaudelyticsError::validation_error(
                "session_path",
                "Invalid session path structure",
            ));
        }

        Ok(SessionId(components.join("/")))
    }
}

/// レコード検証を担当するコンポーネント
#[allow(dead_code)]
pub struct RecordValidator {
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
}

#[allow(dead_code)]
impl RecordValidator {
    pub fn new(since: Option<NaiveDate>, until: Option<NaiveDate>) -> Result<Self> {
        if let (Some(since), Some(until)) = (since, until) {
            if since > until {
                return Err(ClaudelyticsError::validation_error(
                    "date_range",
                    "Since date must be before or equal to until date",
                ));
            }
        }

        Ok(Self { since, until })
    }

    /// レコードが有効かどうかを検証
    pub fn is_valid(&self, record: &RawUsageRecord) -> bool {
        // タイムスタンプとusageデータが必須
        let timestamp = match record.timestamp {
            Some(ts) => ts,
            None => return false,
        };

        let usage = match record.message.as_ref().and_then(|m| m.usage.as_ref()) {
            Some(usage) => usage,
            None => return false,
        };

        // トークン数が0以上であることを確認
        if usage.input_tokens == 0
            && usage.output_tokens == 0
            && usage.cache_creation_input_tokens == 0
            && usage.cache_read_input_tokens == 0
        {
            return false;
        }

        // 日付範囲のフィルタリング
        let date = Local.from_utc_datetime(&timestamp.naive_utc()).date_naive();

        if let Some(since) = self.since {
            if date < since {
                return false;
            }
        }

        if let Some(until) = self.until {
            if date > until {
                return false;
            }
        }

        true
    }
}

/// 生データをドメインオブジェクトに変換
#[allow(dead_code)]
pub struct RecordConverter {
    cost_calculator: Box<dyn CostCalculator + Send + Sync>,
}

#[allow(dead_code)]
impl RecordConverter {
    pub fn new(cost_calculator: Box<dyn CostCalculator + Send + Sync>) -> Self {
        Self { cost_calculator }
    }

    /// 生レコードをUsageEventに変換
    pub fn convert_to_event(
        &self,
        record: &RawUsageRecord,
        session_id: SessionId,
    ) -> Option<UsageEvent> {
        let timestamp = record.timestamp?;
        let raw_usage = record.message.as_ref()?.usage.as_ref()?;

        let token_usage = TokenUsage::new(
            raw_usage.input_tokens,
            raw_usage.output_tokens,
            raw_usage.cache_creation_input_tokens,
            raw_usage.cache_read_input_tokens,
        );

        let mut event = UsageEvent::new(timestamp, token_usage.clone(), session_id);

        // モデル名があれば設定
        if let Some(model_str) = record.message.as_ref()?.model.as_ref() {
            let model = ModelName(model_str.clone());
            event = event.with_model(model.clone());

            // コスト計算
            let cost = if let Some(cost_usd) = record.cost_usd {
                Cost(cost_usd)
            } else {
                self.cost_calculator
                    .calculate_cost(&model, &token_usage)
                    .unwrap_or(Cost(0.0))
            };
            event = event.with_cost(cost);
        } else if let Some(cost_usd) = record.cost_usd {
            event = event.with_cost(Cost(cost_usd));
        }

        Some(event)
    }
}

/// データ集約を担当するコンポーネント
#[allow(dead_code)]
pub struct DataAggregator;

#[allow(dead_code)]
impl DataAggregator {
    pub fn aggregate_by_date(events: Vec<UsageEvent>) -> HashMap<NaiveDate, UsageMetrics> {
        let mut daily_metrics = HashMap::new();

        for event in events {
            let date = event.date();
            let cost = event.cost.unwrap_or(Cost(0.0));
            let metrics = UsageMetrics::new(event.token_usage, cost);

            daily_metrics
                .entry(date)
                .or_insert_with(UsageMetrics::default)
                .add(&metrics);
        }

        daily_metrics
    }

    pub fn aggregate_by_session(
        events: Vec<UsageEvent>,
    ) -> HashMap<SessionId, (UsageMetrics, DateTime<Utc>)> {
        let mut session_metrics = HashMap::new();

        for event in events {
            let session_id = event.session_id.clone();
            let cost = event.cost.unwrap_or(Cost(0.0));
            let metrics = UsageMetrics::new(event.token_usage, cost);

            let entry = session_metrics
                .entry(session_id)
                .or_insert((UsageMetrics::default(), event.timestamp));

            entry.0.add(&metrics);
            if event.timestamp > entry.1 {
                entry.1 = event.timestamp;
            }
        }

        session_metrics
    }
}

/// 並列処理を管理するコンポーネント
#[allow(dead_code)]
pub struct ParallelProcessor {
    file_processor: FileProcessor,
    validator: RecordValidator,
    converter: RecordConverter,
}

#[allow(dead_code)]
impl ParallelProcessor {
    pub fn new(
        claude_dir: PathBuf,
        since: Option<NaiveDate>,
        until: Option<NaiveDate>,
        cost_calculator: Box<dyn CostCalculator + Send + Sync>,
    ) -> Result<Self> {
        let file_processor = FileProcessor::new(claude_dir);
        let validator = RecordValidator::new(since, until)?;
        let converter = RecordConverter::new(cost_calculator);

        Ok(Self {
            file_processor,
            validator,
            converter,
        })
    }

    /// すべてのファイルを並列処理
    #[allow(clippy::type_complexity)]
    pub fn process_all_files(
        &self,
    ) -> Result<(
        HashMap<NaiveDate, UsageMetrics>,
        HashMap<SessionId, (UsageMetrics, DateTime<Utc>)>,
    )> {
        let files = self.file_processor.find_jsonl_files()?;

        if files.is_empty() {
            return Err(ClaudelyticsError::no_usage_data(
                "any files in Claude directory",
            ));
        }

        let all_events: Vec<UsageEvent> = files
            .par_iter()
            .filter_map(|file_path| match self.process_single_file(file_path) {
                Ok(events) => Some(events),
                Err(e) => {
                    eprintln!("Warning: Failed to process {}: {}", file_path.display(), e);
                    None
                }
            })
            .flatten()
            .collect();

        if all_events.is_empty() {
            return Err(ClaudelyticsError::no_usage_data(
                "valid records in any files",
            ));
        }

        let daily_metrics = DataAggregator::aggregate_by_date(all_events.clone());
        let session_metrics = DataAggregator::aggregate_by_session(all_events);

        Ok((daily_metrics, session_metrics))
    }

    /// 単一ファイルを処理
    fn process_single_file(&self, file_path: &Path) -> Result<Vec<UsageEvent>> {
        let session_id = self.file_processor.extract_session_id(file_path)?;
        let records = self.file_processor.read_records(file_path)?;

        let events = records
            .iter()
            .filter(|record| self.validator.is_valid(record))
            .filter_map(|record| self.converter.convert_to_event(record, session_id.clone()))
            .collect();

        Ok(events)
    }
}

/// 日付パースのユーティリティ
#[allow(dead_code)]
pub fn parse_date(date_str: &str) -> Result<NaiveDate> {
    if date_str.len() != 8 {
        return Err(ClaudelyticsError::date_parse_error(date_str, "YYYYMMDD"));
    }

    let year: i32 = date_str[0..4]
        .parse()
        .map_err(|_| ClaudelyticsError::date_parse_error(date_str, "YYYYMMDD (invalid year)"))?;
    let month: u32 = date_str[4..6]
        .parse()
        .map_err(|_| ClaudelyticsError::date_parse_error(date_str, "YYYYMMDD (invalid month)"))?;
    let day: u32 = date_str[6..8]
        .parse()
        .map_err(|_| ClaudelyticsError::date_parse_error(date_str, "YYYYMMDD (invalid day)"))?;

    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| ClaudelyticsError::date_parse_error(date_str, "YYYYMMDD (invalid date)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_date_valid() {
        let result = parse_date("20240301");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 3);
        assert_eq!(date.day(), 1);
    }

    #[test]
    fn test_parse_date_invalid_format() {
        let result = parse_date("2024-03-01");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_invalid_date() {
        let result = parse_date("20240229"); // 2024 is a leap year, so this should be valid
        assert!(result.is_ok());

        let result = parse_date("20230229"); // 2023 is not a leap year
        assert!(result.is_err());
    }

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage::new(100, 200, 50, 25);
        assert_eq!(usage.total().0, 375);
    }

    #[test]
    fn test_token_usage_empty() {
        let usage = TokenUsage::default();
        assert!(usage.is_empty());
    }
}
