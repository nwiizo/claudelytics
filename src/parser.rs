use crate::models::{DailyUsageMap, SessionUsageMap, TokenUsage, UsageRecord};
use crate::pricing::{PricingFetcher, get_fallback_pricing};
use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, TimeZone};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct UsageParser {
    claude_dir: PathBuf,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    pricing_fetcher: PricingFetcher,
}

impl UsageParser {
    pub fn new(claude_dir: PathBuf, since: Option<String>, until: Option<String>) -> Result<Self> {
        let since = since.map(|s| parse_date(&s)).transpose()?;
        let until = until.map(|s| parse_date(&s)).transpose()?;

        if let (Some(since), Some(until)) = (since, until) {
            if since > until {
                anyhow::bail!("Since date must be before or equal to until date");
            }
        }

        Ok(UsageParser {
            claude_dir,
            since,
            until,
            pricing_fetcher: PricingFetcher::new(),
        })
    }

    pub fn parse_all(&self) -> Result<(DailyUsageMap, SessionUsageMap)> {
        let jsonl_files = self.find_jsonl_files()?;

        if jsonl_files.is_empty() {
            eprintln!(
                "Warning: No JSONL files found in {}",
                self.claude_dir.display()
            );
            return Ok((HashMap::new(), HashMap::new()));
        }

        let results: Vec<(DailyUsageMap, SessionUsageMap)> = jsonl_files
            .par_iter()
            .filter_map(|file_path| match self.parse_file(file_path) {
                Ok(result) => Some(result),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
                    None
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

        Ok((daily_map, session_map))
    }

    fn find_jsonl_files(&self) -> Result<Vec<PathBuf>> {
        let projects_dir = self.claude_dir.join("projects");

        if !projects_dir.exists() {
            anyhow::bail!(
                "Claude directory not found at {}",
                self.claude_dir.display()
            );
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

    fn parse_file(&self, file_path: &Path) -> Result<(DailyUsageMap, SessionUsageMap)> {
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
                    // Skip records without timestamp or usage data
                    if let Some(timestamp) = record.timestamp {
                        if record
                            .message
                            .as_ref()
                            .and_then(|m| m.usage.as_ref())
                            .is_some()
                            && self.should_include_record(&record)
                        {
                            let mut usage = TokenUsage::from(&record);

                            // Always recalculate cost to match ccusage's behavior
                            // This ensures exact alignment with ccusage's calculation methodology
                            if let Some(model_name) = record.get_model_name() {
                                let calculated_cost = self.calculate_cost_for_record(&record, model_name);
                                if calculated_cost > 0.0 {
                                    usage.total_cost = calculated_cost;
                                }
                                // If calculation fails, fall back to costUSD if available
                                else if usage.total_cost == 0.0 && record.cost_usd.is_some() {
                                    usage.total_cost = record.cost_usd.unwrap_or(0.0);
                                }
                            }
                            // If no model name, use costUSD if available
                            else if usage.total_cost == 0.0 && record.cost_usd.is_some() {
                                usage.total_cost = record.cost_usd.unwrap_or(0.0);
                            }

                            let date = Local.from_utc_datetime(&timestamp.naive_utc()).date_naive();

                            daily_map
                                .entry(date)
                                .or_insert_with(TokenUsage::default)
                                .add(&usage);

                            session_map
                                .entry(session_info.clone())
                                .or_insert((TokenUsage::default(), timestamp))
                                .0
                                .add(&usage);

                            let session_entry = session_map.get_mut(&session_info).unwrap();
                            if timestamp > session_entry.1 {
                                session_entry.1 = timestamp;
                            }
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
        let projects_dir = self.claude_dir.join("projects");
        let relative_path = file_path
            .strip_prefix(&projects_dir)
            .with_context(|| "File path is not within projects directory")?;

        let mut components: Vec<&str> = relative_path
            .components()
            .filter_map(|comp| comp.as_os_str().to_str())
            .collect();

        if components.is_empty() {
            anyhow::bail!("Invalid file path structure");
        }

        // Remove the filename to get the session directory
        components.pop();

        if components.is_empty() {
            anyhow::bail!("Invalid session path structure");
        }

        Ok(components.join("/"))
    }

    fn should_include_record(&self, record: &UsageRecord) -> bool {
        let timestamp = match record.timestamp {
            Some(ts) => ts,
            None => return false,
        };
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

    fn calculate_cost_for_record(&self, record: &UsageRecord, model_name: &str) -> f64 {
        // Use fallback pricing for now - online fetching would be added as async feature
        let fallback_pricing = get_fallback_pricing();

        if let Some(pricing) = self
            .pricing_fetcher
            .get_model_pricing(&fallback_pricing, model_name)
        {
            if let Some(usage) = record.message.as_ref().and_then(|m| m.usage.as_ref()) {
                return self.pricing_fetcher.calculate_cost(
                    &pricing,
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.cache_creation_input_tokens,
                    usage.cache_read_input_tokens,
                );
            }
        }

        0.0
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
