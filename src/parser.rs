use crate::billing_blocks::BillingBlockManager;
use crate::models::{DailyUsageMap, SessionUsageMap, TokenUsage, UsageRecord};
use crate::models_registry::ModelsRegistry;
use crate::pricing::{FAST_MODE_MULTIPLIER, PricingFetcher, get_fallback_pricing};
use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, TimeZone};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

/// Cost calculation mode
#[derive(Clone, Copy, Debug, Default)]
pub enum CostMode {
    /// Use costUSD from JSONL if present, otherwise calculate
    #[default]
    Auto,
    /// Always recalculate from token counts and pricing
    Calculate,
    /// Only show costUSD field from JSONL data
    Display,
}

pub struct UsageParser {
    claude_dirs: Vec<PathBuf>,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    model_filter: Option<String>,
    cost_mode: CostMode,
    pricing_fetcher: PricingFetcher,
    fallback_pricing: HashMap<String, crate::pricing::ModelPricing>,
    models_registry: ModelsRegistry,
}

impl UsageParser {
    /// Create a parser with a single Claude directory (backward compatible)
    pub fn new(
        claude_dir: PathBuf,
        since: Option<String>,
        until: Option<String>,
        model_filter: Option<String>,
    ) -> Result<Self> {
        Self::new_multi(
            vec![claude_dir],
            since,
            until,
            model_filter,
            CostMode::default(),
        )
    }

    /// Create a parser with multiple Claude directories (supports XDG + legacy)
    pub fn new_multi(
        claude_dirs: Vec<PathBuf>,
        since: Option<String>,
        until: Option<String>,
        model_filter: Option<String>,
        cost_mode: CostMode,
    ) -> Result<Self> {
        let since = since.map(|s| parse_date(&s)).transpose()?;
        let until = until.map(|s| parse_date(&s)).transpose()?;

        if let (Some(since), Some(until)) = (since, until)
            && since > until
        {
            anyhow::bail!("Since date must be before or equal to until date");
        }

        Ok(UsageParser {
            claude_dirs,
            since,
            until,
            fallback_pricing: get_fallback_pricing(),
            model_filter,
            cost_mode,
            pricing_fetcher: PricingFetcher::new(),
            models_registry: ModelsRegistry::new(),
        })
    }

    pub fn parse_all(&self) -> Result<(DailyUsageMap, SessionUsageMap, BillingBlockManager)> {
        let jsonl_files = self.find_jsonl_files()?;

        if jsonl_files.is_empty() {
            let dir_list: Vec<String> = self
                .claude_dirs
                .iter()
                .map(|d| d.display().to_string())
                .collect();
            eprintln!("Warning: No JSONL files found in {}", dir_list.join(", "));
            return Ok((HashMap::new(), HashMap::new(), BillingBlockManager::new()));
        }

        // Use thread-safe billing block manager and dedup set
        let billing_manager = Arc::new(Mutex::new(BillingBlockManager::new()));
        let dedup_set: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

        let results: Vec<(DailyUsageMap, SessionUsageMap)> = jsonl_files
            .par_iter()
            .filter_map(|file_path| {
                let billing_manager_clone = Arc::clone(&billing_manager);
                let dedup_clone = Arc::clone(&dedup_set);
                match self.parse_file_with_billing(file_path, billing_manager_clone, dedup_clone) {
                    Ok(result) => Some(result),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
                        None
                    }
                }
            })
            .collect();

        let mut daily_map = HashMap::new();
        let mut session_map = HashMap::new();

        for (daily, sessions) in results {
            for (date, usage) in daily {
                daily_map
                    .entry(date)
                    .or_insert_with(TokenUsage::default)
                    .add(&usage);
            }

            for (session_key, (usage, last_activity)) in sessions {
                let entry = session_map
                    .entry(session_key)
                    .or_insert((TokenUsage::default(), last_activity));
                entry.0.add(&usage);
                if last_activity > entry.1 {
                    entry.1 = last_activity;
                }
            }
        }

        // Extract the billing manager from Arc<Mutex<>>
        let billing_manager = Arc::try_unwrap(billing_manager)
            .map(|mutex| mutex.into_inner().expect("mutex not poisoned"))
            .unwrap_or_else(|arc| arc.lock().expect("mutex not poisoned").clone());

        Ok((daily_map, session_map, billing_manager))
    }

    fn find_jsonl_files(&self) -> Result<Vec<PathBuf>> {
        let mut all_files = Vec::new();
        let mut found_any_dir = false;

        for claude_dir in &self.claude_dirs {
            let projects_dir = claude_dir.join("projects");
            if !projects_dir.exists() {
                continue;
            }
            found_any_dir = true;

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

            all_files.extend(files);
        }

        if !found_any_dir {
            let dir_list: Vec<String> = self
                .claude_dirs
                .iter()
                .map(|d| d.display().to_string())
                .collect();
            eprintln!(
                "Warning: No Claude projects directory found in: {}",
                dir_list.join(", ")
            );
        }

        // Deduplicate by path
        all_files.sort();
        all_files.dedup();

        Ok(all_files)
    }

    fn parse_file_with_billing(
        &self,
        file_path: &Path,
        billing_manager: Arc<Mutex<BillingBlockManager>>,
        dedup_set: Arc<Mutex<HashSet<String>>>,
    ) -> Result<(DailyUsageMap, SessionUsageMap)> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
        let reader = BufReader::new(file);

        let mut daily_map = HashMap::new();
        let mut session_map = HashMap::new();

        let session_info = self.extract_session_info(file_path)?;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<UsageRecord>(&line) {
                Ok(record) => {
                    // Deduplicate by message.id:requestId (matching ccusage behavior)
                    if let Some(hash) = record.dedup_hash()
                        && let Ok(mut set) = dedup_set.lock()
                        && !set.insert(hash)
                    {
                        continue; // Duplicate record, skip
                    }

                    // Skip records without timestamp or usage data
                    if let Some(timestamp) = record.timestamp
                        && record
                            .message
                            .as_ref()
                            .and_then(|m| m.usage.as_ref())
                            .is_some()
                        && self.should_include_record(&record)
                    {
                        let mut usage = TokenUsage::from(&record);
                        let is_fast = Self::is_fast_mode_record(&record);

                        // Calculate cost based on cost mode
                        self.apply_cost_mode(&mut usage, &record, is_fast);

                        let date = Local.from_utc_datetime(&timestamp.naive_utc()).date_naive();

                        // Add to daily map
                        daily_map
                            .entry(date)
                            .or_insert_with(TokenUsage::default)
                            .add(&usage);

                        // Add to session map
                        let session_entry = session_map
                            .entry(session_info.clone())
                            .or_insert((TokenUsage::default(), timestamp));
                        session_entry.0.add(&usage);
                        if timestamp > session_entry.1 {
                            session_entry.1 = timestamp;
                        }

                        // Add to billing blocks
                        if let Ok(mut manager) = billing_manager.lock() {
                            manager.add_usage(timestamp, &usage, Some(&session_info));
                        }
                    }
                }
                Err(_) => {
                    // Silently skip invalid JSON lines as per spec
                    continue;
                }
            }
        }

        Ok((daily_map, session_map))
    }

    fn extract_session_info(&self, file_path: &Path) -> Result<String> {
        for claude_dir in &self.claude_dirs {
            let projects_dir = claude_dir.join("projects");
            if let Ok(relative_path) = file_path.strip_prefix(&projects_dir) {
                let mut components: Vec<&str> = relative_path
                    .components()
                    .filter_map(|comp| comp.as_os_str().to_str())
                    .collect();

                if components.is_empty() {
                    continue;
                }

                // Last component is the filename (e.g., "uuid.jsonl")
                // Strip .jsonl extension to get the session UUID
                if let Some(last) = components.last_mut()
                    && let Some(stem) = last.strip_suffix(".jsonl")
                {
                    *last = stem;
                }

                // Return "project-dir/session-uuid" or "project-dir/subdir/session-uuid"
                return Ok(components.join("/"));
            }
        }

        anyhow::bail!("File path is not within any known projects directory")
    }

    fn should_include_record(&self, record: &UsageRecord) -> bool {
        let timestamp = match record.timestamp {
            Some(ts) => ts,
            None => return false,
        };
        let date = Local.from_utc_datetime(&timestamp.naive_utc()).date_naive();

        if let Some(since) = self.since
            && date < since
        {
            return false;
        }

        if let Some(until) = self.until
            && date > until
        {
            return false;
        }

        // Apply model filter using ModelsRegistry
        if let Some(filter) = &self.model_filter {
            if let Some(model_name) = record.get_model_name() {
                if !self.models_registry.matches_filter(model_name, filter) {
                    return false;
                }
            } else {
                // No model name in record, exclude it when filter is active
                return false;
            }
        }

        true
    }

    /// Calculate cost for a single record from token counts and model pricing.
    /// Returns cost with fast mode multiplier already applied if applicable.
    fn calculate_cost_for_record(&self, record: &UsageRecord, model_name: &str) -> f64 {
        let fallback_pricing = &self.fallback_pricing;

        if let Some(pricing) = self
            .pricing_fetcher
            .get_model_pricing(fallback_pricing, model_name)
            && let Some(usage) = record.message.as_ref().and_then(|m| m.usage.as_ref())
        {
            let base_cost = self.pricing_fetcher.calculate_cost(
                &pricing,
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_creation_input_tokens,
                usage.cache_read_input_tokens,
            );

            // Apply fast mode multiplier if speed is "fast"
            if usage.is_fast_mode() {
                return base_cost * FAST_MODE_MULTIPLIER;
            }

            return base_cost;
        }

        0.0
    }

    /// Check if a record is using fast mode
    fn is_fast_mode_record(record: &UsageRecord) -> bool {
        record
            .message
            .as_ref()
            .and_then(|m| m.usage.as_ref())
            .map(|u| u.is_fast_mode())
            .unwrap_or(false)
    }

    /// Apply cost based on the configured cost mode
    fn apply_cost_mode(&self, usage: &mut TokenUsage, record: &UsageRecord, is_fast: bool) {
        match self.cost_mode {
            CostMode::Display => {
                // Only use costUSD from JSONL
                usage.total_cost = record.cost_usd.unwrap_or(0.0);
            }
            CostMode::Calculate => {
                // Always recalculate from tokens - zero out any costUSD leak
                usage.total_cost = 0.0;
                if let Some(model_name) = record.get_model_name() {
                    let calculated = self.calculate_cost_for_record(record, model_name);
                    if calculated > 0.0 {
                        usage.total_cost = calculated;
                    }
                }
            }
            CostMode::Auto => {
                // Use costUSD if positive, otherwise calculate from tokens
                let has_jsonl_cost = record.cost_usd.is_some_and(|c| c > 0.0);
                if has_jsonl_cost {
                    usage.total_cost = record.cost_usd.unwrap_or(0.0);
                } else if let Some(model_name) = record.get_model_name() {
                    let calculated = self.calculate_cost_for_record(record, model_name);
                    if calculated > 0.0 {
                        usage.total_cost = calculated;
                    }
                }
            }
        }

        // Track fast mode cost separately
        if is_fast {
            usage.fast_mode_cost = usage.total_cost;
        }
    }
}

fn parse_date(date_str: &str) -> Result<NaiveDate> {
    if date_str.len() != 8 {
        anyhow::bail!("Date must be in YYYYMMDD format");
    }

    let year: i32 = date_str[0..4]
        .parse()
        .with_context(|| "Invalid year in date")?;
    let month: u32 = date_str[4..6]
        .parse()
        .with_context(|| "Invalid month in date")?;
    let day: u32 = date_str[6..8]
        .parse()
        .with_context(|| "Invalid day in date")?;

    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", date_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_jsonl_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create test file");
        write!(file, "{}", content).expect("Failed to write test content");
        file_path
    }

    #[allow(dead_code)]
    fn create_test_record(
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
        session_id: &str,
        timestamp: &str,
    ) -> String {
        format!(
            r#"{{"uuid":"test-uuid","type":"response.done","timestamp":"{}","model":"{}","usage":{{"input_tokens":{},"output_tokens":{},"cache_creation_tokens":0,"cache_read_tokens":0}},"sessionId":"{}"}}"#,
            timestamp, model, input_tokens, output_tokens, session_id
        )
    }

    #[test]
    fn test_parse_date_valid() {
        assert_eq!(
            parse_date("20240101").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert_eq!(
            parse_date("20231231").unwrap(),
            NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()
        );
    }

    #[test]
    fn test_parse_date_invalid_format() {
        assert!(parse_date("2024-01-01").is_err());
        assert!(parse_date("240101").is_err());
        assert!(parse_date("202401001").is_err());
    }

    #[test]
    fn test_parse_date_invalid_values() {
        assert!(parse_date("20241301").is_err()); // Invalid month
        assert!(parse_date("20240132").is_err()); // Invalid day
        assert!(parse_date("20240229").is_ok()); // Valid leap year
        assert!(parse_date("20230229").is_err()); // Invalid non-leap year
    }

    #[test]
    fn test_usage_parser_new() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Test valid creation
        let parser = UsageParser::new(
            temp_dir.path().to_path_buf(),
            Some("20240101".to_string()),
            Some("20240131".to_string()),
            Some("claude-3-opus".to_string()),
        );
        assert!(parser.is_ok());

        // Test invalid date range
        let parser = UsageParser::new(
            temp_dir.path().to_path_buf(),
            Some("20240131".to_string()),
            Some("20240101".to_string()),
            None,
        );
        assert!(parser.is_err());
    }

    #[test]
    fn test_find_jsonl_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let projects_dir = temp_dir.path().join("projects");
        fs::create_dir_all(&projects_dir).expect("Failed to create projects dir");

        // Create test files
        create_test_jsonl_file(&projects_dir, "test1.jsonl", "");
        create_test_jsonl_file(&projects_dir, "test2.jsonl", "");
        create_test_jsonl_file(&projects_dir, "test.txt", ""); // Should be ignored

        // Create subdirectory with file
        let sub_dir = projects_dir.join("subproject");
        fs::create_dir_all(&sub_dir).expect("Failed to create sub dir");
        create_test_jsonl_file(&sub_dir, "test3.jsonl", "");

        let parser = UsageParser::new(temp_dir.path().to_path_buf(), None, None, None)
            .expect("Failed to create parser");

        let files = parser.find_jsonl_files().expect("Failed to find files");
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|f| f.extension().unwrap() == "jsonl"));
    }

    #[test]
    fn test_should_include_record() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let parser = UsageParser::new(
            temp_dir.path().to_path_buf(),
            Some("20240101".to_string()),
            Some("20240131".to_string()),
            Some("claude-3-opus".to_string()),
        )
        .expect("Failed to create parser");

        // Create test record JSON string
        let record_str = r#"{
            "uuid": "test",
            "type": "response.done",
            "timestamp": "2024-01-15T12:00:00Z",
            "message": {
                "model": "claude-3-opus-20240229",
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 200,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0
                }
            },
            "sessionId": "session1"
        }"#;

        let record: UsageRecord =
            serde_json::from_str(record_str).expect("Failed to parse test record");
        assert!(parser.should_include_record(&record));

        // Test record outside date range
        let record_str_outside = r#"{
            "uuid": "test",
            "type": "response.done",
            "timestamp": "2024-02-15T12:00:00Z",
            "message": {
                "model": "claude-3-opus-20240229",
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 200,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0
                }
            },
            "sessionId": "session1"
        }"#;

        let record_outside: UsageRecord =
            serde_json::from_str(record_str_outside).expect("Failed to parse test record");
        assert!(!parser.should_include_record(&record_outside));

        // Test record with non-matching model
        let record_str_wrong_model = r#"{
            "uuid": "test",
            "type": "response.done",
            "timestamp": "2024-01-15T12:00:00Z",
            "message": {
                "model": "claude-3-sonnet-20240229",
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 200,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0
                }
            },
            "sessionId": "session1"
        }"#;

        let record_wrong_model: UsageRecord =
            serde_json::from_str(record_str_wrong_model).expect("Failed to parse test record");
        assert!(!parser.should_include_record(&record_wrong_model));
    }

    #[test]
    fn test_extract_session_info() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let parser = UsageParser::new(temp_dir.path().to_path_buf(), None, None, None)
            .expect("Failed to create parser");

        // Test with projects structure
        let projects_dir = temp_dir.path().join("projects");
        fs::create_dir_all(&projects_dir).expect("Failed to create projects dir");

        let test_project_dir = projects_dir.join("test-project");
        fs::create_dir_all(&test_project_dir).expect("Failed to create test project dir");

        let path = test_project_dir.join("test-session.jsonl");
        create_test_jsonl_file(&test_project_dir, "test-session.jsonl", "");

        let session_info = parser
            .extract_session_info(&path)
            .expect("Failed to extract session info");
        assert_eq!(session_info, "test-project/test-session");
    }

    #[test]
    fn test_parse_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let projects_dir = temp_dir.path().join("projects").join("test-project");
        fs::create_dir_all(&projects_dir).expect("Failed to create projects dir");

        // Create test JSONL content with proper format
        let content = format!(
            "{}\n{}\n{}\n",
            r#"{"uuid":"uuid1","type":"response.done","timestamp":"2024-01-15T12:00:00Z","message":{"model":"claude-3-opus-20240229","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#,
            r#"{"uuid":"uuid2","type":"response.done","timestamp":"2024-01-15T12:01:00Z","message":{"model":"claude-3-sonnet-20240229","usage":{"input_tokens":50,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#,
            r#"{"type":"summary","summary":"Test session"}"#
        );

        let file_path = create_test_jsonl_file(&projects_dir, "test.jsonl", &content);

        let parser = UsageParser::new(temp_dir.path().to_path_buf(), None, None, None)
            .expect("Failed to create parser");

        let billing_manager = Arc::new(Mutex::new(BillingBlockManager::new()));
        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (daily_map, session_map) = parser
            .parse_file_with_billing(&file_path, billing_manager, dedup_set)
            .expect("Failed to parse file");

        assert_eq!(daily_map.len(), 1);
        assert_eq!(session_map.len(), 1);

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert!(daily_map.contains_key(&date));
        assert_eq!(daily_map[&date].input_tokens, 150);
        assert_eq!(daily_map[&date].output_tokens, 300);
    }

    #[test]
    fn test_parse_all_integration() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create two separate project directories for different sessions
        let projects_dir1 = temp_dir.path().join("projects").join("test-project1");
        let projects_dir2 = temp_dir.path().join("projects").join("test-project2");
        fs::create_dir_all(&projects_dir1).expect("Failed to create projects dir 1");
        fs::create_dir_all(&projects_dir2).expect("Failed to create projects dir 2");

        // Create test files in separate directories
        let content1 = r#"{"uuid":"uuid1","type":"response.done","timestamp":"2024-01-15T12:00:00Z","message":{"model":"claude-3-opus-20240229","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}
"#;
        let content2 = r#"{"uuid":"uuid2","type":"response.done","timestamp":"2024-01-16T12:00:00Z","message":{"model":"claude-3-sonnet-20240229","usage":{"input_tokens":50,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session2"}
"#;

        create_test_jsonl_file(&projects_dir1, "session1.jsonl", &content1);
        create_test_jsonl_file(&projects_dir2, "session2.jsonl", &content2);

        let parser = UsageParser::new(temp_dir.path().to_path_buf(), None, None, None)
            .expect("Failed to create parser");

        let (daily_map, session_map, billing_manager) =
            parser.parse_all().expect("Failed to parse all files");

        // We should have 2 different days: Jan 15 and Jan 16
        assert_eq!(daily_map.len(), 2, "Expected 2 days in daily_map");
        assert_eq!(session_map.len(), 2, "Expected 2 sessions in session_map");
        assert!(!billing_manager.get_all_blocks().is_empty());
    }

    #[test]
    fn test_model_filter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let projects_dir = temp_dir.path().join("projects").join("test-project");
        fs::create_dir_all(&projects_dir).expect("Failed to create projects dir");

        // Create test content with different models
        let content = format!(
            "{}\n{}\n{}\n",
            r#"{"uuid":"uuid1","type":"response.done","timestamp":"2024-01-15T12:00:00Z","message":{"model":"claude-3-opus-20240229","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#,
            r#"{"uuid":"uuid2","type":"response.done","timestamp":"2024-01-15T12:01:00Z","message":{"model":"claude-3-sonnet-20240229","usage":{"input_tokens":50,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#,
            r#"{"uuid":"uuid3","type":"response.done","timestamp":"2024-01-15T12:02:00Z","message":{"model":"claude-3-haiku-20240307","usage":{"input_tokens":25,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#
        );

        create_test_jsonl_file(&projects_dir, "test.jsonl", &content);

        // Test with opus filter
        let parser = UsageParser::new(
            temp_dir.path().to_path_buf(),
            None,
            None,
            Some("opus".to_string()),
        )
        .expect("Failed to create parser");

        let (daily_map, _, _) = parser.parse_all().expect("Failed to parse");
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        assert_eq!(daily_map[&date].input_tokens, 100);
        assert_eq!(daily_map[&date].output_tokens, 200);
    }

    #[test]
    fn test_date_filtering() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let projects_dir = temp_dir.path().join("projects").join("test-project");
        fs::create_dir_all(&projects_dir).expect("Failed to create projects dir");

        // Create test content across multiple dates
        let content = format!(
            "{}\n{}\n{}\n",
            r#"{"uuid":"uuid1","type":"response.done","timestamp":"2024-01-10T12:00:00Z","message":{"model":"claude-3-opus-20240229","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#,
            r#"{"uuid":"uuid2","type":"response.done","timestamp":"2024-01-15T12:00:00Z","message":{"model":"claude-3-opus-20240229","usage":{"input_tokens":150,"output_tokens":250,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#,
            r#"{"uuid":"uuid3","type":"response.done","timestamp":"2024-01-20T12:00:00Z","message":{"model":"claude-3-opus-20240229","usage":{"input_tokens":200,"output_tokens":300,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"sessionId":"session1"}"#
        );

        create_test_jsonl_file(&projects_dir, "test.jsonl", &content);

        // Test with date range filter
        let parser = UsageParser::new(
            temp_dir.path().to_path_buf(),
            Some("20240112".to_string()),
            Some("20240118".to_string()),
            None,
        )
        .expect("Failed to create parser");

        let (daily_map, _, _) = parser.parse_all().expect("Failed to parse");

        assert_eq!(daily_map.len(), 1);
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert!(daily_map.contains_key(&date));
        assert_eq!(daily_map[&date].input_tokens, 150);
    }
}
