use crate::models::{DailyUsageMap, SessionUsageMap, TokenUsage};
use crate::models_registry::ModelsRegistry;
use crate::pricing::{PricingFetcher, get_fallback_pricing};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use memchr::memmem;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

const UNKNOWN_CODEX_MODEL: &str = "unknown-codex-model";
const METADATA_SCAN_LINES: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct CodexRawUsage {
    input_tokens: u64,
    cache_write_input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_output_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CodexUsageEvent {
    session_id: String,
    timestamp: String,
    model: String,
    usage: CodexRawUsage,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CodexEventFingerprint {
    session_id: usize,
    timestamp: usize,
    model: usize,
    usage: CodexRawUsage,
}

#[derive(Debug, Default)]
struct StringInterner {
    indexes: HashMap<String, usize>,
}

impl StringInterner {
    fn intern(&mut self, value: &str) -> usize {
        if let Some(index) = self.indexes.get(value) {
            return *index;
        }
        let index = self.indexes.len();
        self.indexes.insert(value.to_owned(), index);
        index
    }
}

#[derive(Debug, Default)]
struct CodexReplayPlan {
    replayed_usage_by_child: HashMap<PathBuf, CodexReplayPrefix>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct CodexReplayPrefix {
    usage: Arc<[CodexRawUsage]>,
    len: usize,
}

#[derive(Debug, Default)]
struct CodexSessionMetadata {
    session_id: Option<String>,
    parent_id: Option<String>,
    forked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
pub struct CodexUsageData {
    pub daily: DailyUsageMap,
    pub sessions: SessionUsageMap,
    pub models: HashMap<(String, String), TokenUsage>,
    pub warnings: Vec<String>,
}

impl CodexReplayPlan {
    fn new(sessions_dir: &Path, all_files: &[PathBuf], child_files: &[PathBuf]) -> Self {
        let mut warnings = Vec::new();
        let selected_metadata: Vec<_> = child_files
            .iter()
            .map(|path| {
                read_codex_session_metadata(path).unwrap_or_else(|error| {
                    warnings.push(error.to_string());
                    CodexSessionMetadata::default()
                })
            })
            .collect();
        let files_by_session_id: HashMap<_, _> = all_files
            .iter()
            .filter_map(|path| codex_session_id_from_filename(path).map(|id| (id, path)))
            .collect();
        let mut replayed_usage_by_child = HashMap::new();
        let mut parent_events_by_path = HashMap::new();
        let mut parent_usage_by_path = HashMap::new();

        for (child_path, child_metadata) in child_files.iter().zip(&selected_metadata) {
            let Some(parent_id) = child_metadata.parent_id.as_deref() else {
                continue;
            };
            let Some(parent_path) = files_by_session_id
                .get(parent_id)
                .copied()
                .or_else(|| {
                    child_files
                        .iter()
                        .zip(&selected_metadata)
                        .find(|(_, metadata)| metadata.session_id.as_deref() == Some(parent_id))
                        .map(|(path, _)| path)
                })
                .filter(|parent_path| parent_path.as_path() != child_path.as_path())
            else {
                continue;
            };

            if !parent_events_by_path.contains_key(parent_path) {
                match read_codex_session_file(sessions_dir, parent_path) {
                    Ok(events) => {
                        parent_events_by_path.insert((*parent_path).clone(), events);
                    }
                    Err(error) => {
                        warnings.push(error.to_string());
                        continue;
                    }
                }
            }
            let Some(parent_events) = parent_events_by_path.get(parent_path) else {
                continue;
            };
            let replayed_len = parent_events
                .iter()
                .take_while(|event| {
                    child_metadata.forked_at.as_ref().is_none_or(|forked_at| {
                        parse_codex_datetime(&event.timestamp)
                            .is_none_or(|timestamp| timestamp <= *forked_at)
                    })
                })
                .count();
            let parent_usage = parent_usage_by_path
                .entry((*parent_path).clone())
                .or_insert_with(|| {
                    Arc::from(
                        parent_events
                            .iter()
                            .map(|event| event.usage.clone())
                            .collect::<Vec<_>>(),
                    )
                });
            replayed_usage_by_child.insert(
                child_path.clone(),
                CodexReplayPrefix {
                    usage: Arc::clone(parent_usage),
                    len: replayed_len,
                },
            );
        }

        Self {
            replayed_usage_by_child,
            warnings,
        }
    }

    fn replayed_prefix(&self, child: &Path) -> Option<&[CodexRawUsage]> {
        self.replayed_usage_by_child
            .get(child)
            .map(|prefix| &prefix.usage[..prefix.len])
    }
}

pub fn parse_codex_model_usage(
    since: Option<&str>,
    until: Option<&str>,
    timezone: Option<&str>,
    model_filter: Option<&str>,
    calculate_costs: bool,
) -> Result<HashMap<(String, String), TokenUsage>> {
    Ok(parse_codex_usage(since, until, timezone, model_filter, calculate_costs)?.models)
}

pub fn parse_codex_usage(
    since: Option<&str>,
    until: Option<&str>,
    timezone: Option<&str>,
    model_filter: Option<&str>,
    calculate_costs: bool,
) -> Result<CodexUsageData> {
    let timezone = parse_timezone(timezone)?;
    let since = since.map(date_key);
    let until = until.map(date_key);
    let pricing = get_fallback_pricing();
    let fetcher = PricingFetcher::new();
    let registry = ModelsRegistry::new();
    let mut seen = HashSet::new();
    let mut session_ids = StringInterner::default();
    let mut timestamp_ids = StringInterner::default();
    let mut model_ids = StringInterner::default();
    let mut result = CodexUsageData::default();

    for sessions_dir in codex_usage_paths()? {
        let replay_files = codex_jsonl_files(&sessions_dir, None, None)?;
        let mut selected_files = Vec::new();
        for path in &replay_files {
            match codex_file_may_overlap(path, since.as_deref(), until.as_deref()) {
                Ok(true) => selected_files.push(path.clone()),
                Ok(false) => {}
                Err(error) => {
                    result.warnings.push(error.to_string());
                    selected_files.push(path.clone());
                }
            }
        }
        let replay_plan = CodexReplayPlan::new(&sessions_dir, &replay_files, &selected_files);
        result.warnings.extend(replay_plan.warnings.iter().cloned());
        let chunk_size = rayon::current_num_threads().saturating_mul(2).max(1);
        for file_chunk in selected_files.chunks(chunk_size) {
            let parsed_files: Vec<_> = file_chunk
                .par_iter()
                .map(|file_path| {
                    read_codex_session_file_with_replay(
                        &sessions_dir,
                        file_path,
                        replay_plan.replayed_prefix(file_path),
                    )
                })
                .collect();
            for parsed_file in parsed_files {
                let events = match parsed_file {
                    Ok(events) => events,
                    Err(error) => {
                        result.warnings.push(error.to_string());
                        continue;
                    }
                };
                for event in events {
                    if model_filter
                        .is_some_and(|filter| !registry.matches_filter(&event.model, filter))
                    {
                        continue;
                    }

                    let Some(event_timestamp) = parse_codex_datetime(&event.timestamp) else {
                        continue;
                    };
                    let event_date = event_timestamp.with_timezone(&timezone).date_naive();
                    let event_date_key = event_date.format("%Y%m%d").to_string();
                    if since.as_ref().is_some_and(|since| &event_date_key < since)
                        || until.as_ref().is_some_and(|until| &event_date_key > until)
                    {
                        continue;
                    }

                    let fingerprint = CodexEventFingerprint {
                        session_id: session_ids.intern(&event.session_id),
                        timestamp: timestamp_ids.intern(&event.timestamp),
                        model: model_ids.intern(&event.model),
                        usage: event.usage.clone(),
                    };
                    if !seen.insert(fingerprint) {
                        continue;
                    }

                    accumulate_codex_event(
                        &mut result,
                        &event,
                        event_timestamp,
                        event_date,
                        &pricing,
                        &fetcher,
                        calculate_costs,
                    );
                }
            }
        }
    }

    Ok(result)
}

fn accumulate_codex_event(
    result: &mut CodexUsageData,
    event: &CodexUsageEvent,
    event_timestamp: DateTime<Utc>,
    event_date: NaiveDate,
    pricing: &HashMap<String, crate::pricing::ModelPricing>,
    fetcher: &PricingFetcher,
    calculate_costs: bool,
) {
    let mut usage = codex_usage_to_token_usage(&event.usage);
    if calculate_costs && let Some(model_pricing) = fetcher.get_model_pricing(pricing, &event.model)
    {
        usage.total_cost = fetcher.calculate_cost(
            model_pricing,
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_creation_tokens,
            usage.cache_read_tokens,
        );
    }

    result.daily.entry(event_date).or_default().add(&usage);

    let session_key = format!("Codex/{}", event.session_id);
    let session = result
        .sessions
        .entry(session_key)
        .or_insert_with(|| (TokenUsage::default(), event_timestamp));
    session.0.add(&usage);
    if event_timestamp > session.1 {
        session.1 = event_timestamp;
    }

    result
        .models
        .entry(("Codex".to_string(), event.model.clone()))
        .or_default()
        .add(&usage);
}

pub fn has_codex_usage() -> bool {
    codex_usage_paths()
        .map(|paths| {
            paths.into_iter().any(|path| {
                codex_jsonl_files(&path, None, None).is_ok_and(|files| !files.is_empty())
            })
        })
        .unwrap_or(false)
}

fn codex_usage_paths() -> Result<Vec<PathBuf>> {
    if let Ok(paths) = std::env::var("CODEX_HOME") {
        return Ok(paths
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect());
    }

    let home = dirs::home_dir().context("home directory is not set")?;
    Ok(vec![home.join(".codex")])
}

fn codex_jsonl_files(
    codex_path: &Path,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let root = if codex_path.join("sessions").is_dir() {
        codex_path.join("sessions")
    } else {
        codex_path.to_path_buf()
    };

    if !root.exists() {
        return Ok(Vec::new());
    }

    if let Some(files) = codex_jsonl_files_by_date_dirs(&root, since, until)? {
        return Ok(files);
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(&root) {
        let entry =
            entry.with_context(|| format!("Failed to scan Codex directory {}", root.display()))?;
        if entry.file_type().is_file()
            && is_jsonl_file(entry.path())
            && path_date_in_range(entry.path(), since, until)
        {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

fn codex_jsonl_files_by_date_dirs(
    root: &Path,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Option<Vec<PathBuf>>> {
    let (Some(since), Some(until)) = (
        since.and_then(parse_date_key),
        until.and_then(parse_date_key),
    ) else {
        return Ok(None);
    };
    if since > until {
        return Ok(Some(Vec::new()));
    }

    let days = (until - since).num_days();
    if days > 3660 {
        return Ok(None);
    }

    let mut files = Vec::new();
    for offset in 0..=days {
        let date = since + Duration::days(offset);
        let day_dir = root
            .join(date.format("%Y").to_string())
            .join(date.format("%m").to_string())
            .join(date.format("%d").to_string());
        if !day_dir.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&day_dir) {
            let entry = entry
                .with_context(|| format!("Failed to scan Codex directory {}", day_dir.display()))?;
            if entry.file_type().is_file() && is_jsonl_file(entry.path()) {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    Ok(Some(files))
}

fn is_jsonl_file(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("jsonl")
}

fn path_date_in_range(path: &Path, since: Option<&str>, until: Option<&str>) -> bool {
    let Some(path_date) = date_key_from_path(path) else {
        return true;
    };
    since.is_none_or(|since| path_date.as_str() >= since)
        && until.is_none_or(|until| path_date.as_str() <= until)
}

fn codex_file_may_overlap(path: &Path, since: Option<&str>, until: Option<&str>) -> Result<bool> {
    let started_at = date_key_from_path(path).and_then(|date| parse_date_key(&date));
    let padded_until = until
        .and_then(parse_date_key)
        .and_then(|date| date.checked_add_signed(Duration::days(1)));
    if padded_until.is_some_and(|end| started_at.is_some_and(|start| start > end)) {
        return Ok(false);
    }

    let padded_since = since
        .and_then(parse_date_key)
        .and_then(|date| date.checked_sub_signed(Duration::days(1)));
    let Some(padded_since) = padded_since else {
        return Ok(true);
    };
    if started_at.is_none_or(|start| start >= padded_since) {
        return Ok(true);
    }

    let modified_at = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .with_context(|| format!("Failed to read Codex metadata {}", path.display()))
        .map(DateTime::<Utc>::from)?
        .date_naive();
    if modified_at < padded_since {
        return Ok(false);
    }

    Ok(codex_last_activity_date(path)?.is_none_or(|last_activity| last_activity >= padded_since))
}

fn codex_last_activity_date(path: &Path) -> Result<Option<NaiveDate>> {
    // Token-count and session metadata records are small. A bounded tail keeps
    // copied archives cheap to reject; if the final record is unusually large,
    // returning None deliberately falls back to the correctness-first path.
    const TAIL_BYTES: u64 = 16 * 1024;

    let mut file = File::open(path)
        .with_context(|| format!("Failed to open Codex session tail {}", path.display()))?;
    let length = file
        .metadata()
        .with_context(|| format!("Failed to read Codex metadata {}", path.display()))?
        .len();
    file.seek(SeekFrom::Start(length.saturating_sub(TAIL_BYTES)))
        .with_context(|| format!("Failed to seek Codex session tail {}", path.display()))?;
    let mut bytes = Vec::with_capacity(length.min(TAIL_BYTES) as usize);
    file.read_to_end(&mut bytes)
        .with_context(|| format!("Failed to read Codex session tail {}", path.display()))?;
    let tail = String::from_utf8_lossy(&bytes);

    let last_complete_record = tail
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<Value>(line).ok());
    Ok(last_complete_record
        .and_then(|record| codex_timestamp_from_result(&record))
        .as_deref()
        .and_then(parse_codex_datetime)
        .map(|timestamp| timestamp.date_naive()))
}

fn date_key_from_path(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    if let Some(date_part) = filename
        .strip_prefix("rollout-")
        .and_then(|rest| rest.get(..10))
    {
        let key = date_part.replace('-', "");
        if key.len() == 8 && key.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(key);
        }
    }

    let components: Vec<_> = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();
    for window in components.windows(3) {
        if window[0].len() == 4
            && window[1].len() == 2
            && window[2].len() == 2
            && window
                .iter()
                .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
        {
            return Some(format!("{}{}{}", window[0], window[1], window[2]));
        }
    }

    None
}

fn read_codex_session_metadata(path: &Path) -> Result<CodexSessionMetadata> {
    // session_meta is a rollout header. Scan only complete initial records so
    // headless JSONL files avoid a full pass without truncating valid metadata.
    let file = File::open(path)
        .with_context(|| format!("Failed to open Codex session {}", path.display()))?;

    for (line_number, line) in BufReader::new(file)
        .lines()
        .take(METADATA_SCAN_LINES)
        .enumerate()
    {
        let line = line.with_context(|| {
            format!(
                "Failed to read Codex session {} at line {}",
                path.display(),
                line_number + 1
            )
        })?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let payload = value.get("payload");
        let parent_id = payload
            .and_then(|payload| payload.get("forked_from_id"))
            .and_then(Value::as_str)
            .or_else(|| {
                payload
                    .and_then(|payload| {
                        payload.pointer("/source/subagent/thread_spawn/parent_thread_id")
                    })
                    .and_then(Value::as_str)
            })
            .map(str::trim)
            .filter(|parent_id| !parent_id.is_empty())
            .map(str::to_string);

        return Ok(CodexSessionMetadata {
            session_id: payload
                .and_then(|payload| payload.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string),
            parent_id,
            forked_at: normalize_codex_timestamp(value.get("timestamp"))
                .as_deref()
                .and_then(parse_codex_datetime),
        });
    }

    Ok(CodexSessionMetadata::default())
}

fn read_codex_session_file_with_replay(
    sessions_dir: &Path,
    path: &Path,
    replayed_prefix: Option<&[CodexRawUsage]>,
) -> Result<Vec<CodexUsageEvent>> {
    let events = read_codex_session_file(sessions_dir, path)?;
    let Some(replayed_prefix) = replayed_prefix else {
        return Ok(events);
    };

    let matched = events
        .iter()
        .zip(replayed_prefix)
        .take_while(|(event, replayed)| event.usage == **replayed)
        .count();
    if !replayed_prefix.is_empty() && matched == replayed_prefix.len() {
        return Ok(events.into_iter().skip(replayed_prefix.len()).collect());
    }

    Ok(events)
}

fn read_codex_session_file(sessions_dir: &Path, path: &Path) -> Result<Vec<CodexUsageEvent>> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open Codex session {}", path.display()))?;
    let fallback_timestamp = file_modified_timestamp(path);
    let session_id = codex_session_id(sessions_dir, path);
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut line = Vec::with_capacity(16 * 1024);
    let mut line_number = 0_usize;
    let mut previous_totals: Option<CodexRawUsage> = None;
    let mut current_model: Option<String> = None;
    let mut events = Vec::new();

    loop {
        line.clear();
        let bytes_read = reader.read_until(b'\n', &mut line).with_context(|| {
            format!(
                "Failed to read Codex session {} at line {}",
                path.display(),
                line_number + 1
            )
        })?;
        if bytes_read == 0 {
            break;
        }
        line_number += 1;
        if !line_may_contain_usage(&line) {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };

        if value.get("type").and_then(Value::as_str) == Some("turn_context") {
            if let Some(model) = codex_model_from_payload(value.get("payload")) {
                current_model = Some(model);
            }
            continue;
        }

        if value.get("type").and_then(Value::as_str) == Some("event_msg")
            && value
                .get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str)
                == Some("token_count")
        {
            let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) else {
                continue;
            };
            let Some(payload) = value.get("payload") else {
                continue;
            };
            let info = payload.get("info");
            let total_usage = info
                .and_then(|info| info.get("total_token_usage"))
                .and_then(normalize_codex_raw_usage);
            let cumulative_advanced = total_usage
                .as_ref()
                .is_none_or(|total_usage| previous_totals.as_ref() != Some(total_usage));
            let raw_usage = info
                .and_then(|info| info.get("last_token_usage"))
                .and_then(normalize_codex_raw_usage)
                .filter(|_| cumulative_advanced)
                .or_else(|| {
                    total_usage
                        .as_ref()
                        .map(|usage| subtract_codex_raw_usage(usage, previous_totals.as_ref()))
                });
            if let Some(total_usage) = total_usage {
                previous_totals = Some(total_usage);
            }
            let Some(raw_usage) = raw_usage.filter(has_usage) else {
                continue;
            };
            let parsed_model = codex_model_from_payload(Some(payload))
                .or_else(|| info.and_then(|info| codex_model_from_payload(Some(info))));
            if let Some(model) = parsed_model.clone() {
                current_model = Some(model);
            }
            let model = parsed_model
                .or_else(|| current_model.clone())
                .unwrap_or_else(|| UNKNOWN_CODEX_MODEL.to_string());
            events.push(CodexUsageEvent {
                session_id: session_id.clone(),
                timestamp: timestamp.to_string(),
                model,
                usage: raw_usage,
            });
            continue;
        }

        if let Some(raw_usage) = normalize_headless_codex_usage(&value).filter(has_usage) {
            let parsed_model = codex_model_from_result(&value);
            if let Some(model) = parsed_model.clone() {
                current_model = Some(model);
            }
            let model = parsed_model
                .or_else(|| current_model.clone())
                .unwrap_or_else(|| UNKNOWN_CODEX_MODEL.to_string());
            events.push(CodexUsageEvent {
                session_id: session_id.clone(),
                timestamp: codex_timestamp_from_result(&value)
                    .unwrap_or_else(|| fallback_timestamp.clone()),
                model,
                usage: raw_usage,
            });
        }
    }

    Ok(events)
}

fn line_may_contain_usage(line: &[u8]) -> bool {
    [b"turn_context".as_slice(), b"token_count", br#""usage""#]
        .into_iter()
        .any(|needle| memmem::find(line, needle).is_some())
}

fn codex_session_id(sessions_dir: &Path, path: &Path) -> String {
    path.strip_prefix(sessions_dir)
        .unwrap_or(path)
        .with_extension("")
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn codex_session_id_from_filename(path: &Path) -> Option<&str> {
    let suffix = path.file_stem()?.to_str()?.strip_prefix("rollout-")?;
    let session_id = suffix.get(20..)?;
    (!session_id.is_empty()).then_some(session_id)
}

fn codex_model_from_payload(value: Option<&Value>) -> Option<String> {
    let value = value?;
    ["model", "model_name"]
        .into_iter()
        .find_map(|key| string_field(value.get(key)))
        .or_else(|| {
            value
                .get("metadata")
                .and_then(|metadata| string_field(metadata.get("model")))
        })
}

fn codex_model_from_result(value: &Value) -> Option<String> {
    codex_model_from_payload(Some(value))
        .or_else(|| codex_model_from_payload(value.get("data")))
        .or_else(|| codex_model_from_payload(value.get("result")))
        .or_else(|| codex_model_from_payload(value.get("response")))
}

fn normalize_headless_codex_usage(value: &Value) -> Option<CodexRawUsage> {
    usage_from_result(value).and_then(normalize_codex_raw_usage)
}

fn usage_from_result(value: &Value) -> Option<&Value> {
    value
        .get("usage")
        .or_else(|| value.get("data").and_then(|data| data.get("usage")))
        .or_else(|| value.get("result").and_then(|result| result.get("usage")))
        .or_else(|| {
            value
                .get("response")
                .and_then(|response| response.get("usage"))
        })
}

fn normalize_codex_raw_usage(value: &Value) -> Option<CodexRawUsage> {
    let input_tokens = json_u64(value, &["input_tokens", "inputTokens", "prompt_tokens"])?;
    let cached_input_tokens = json_u64(value, &["cached_input_tokens", "cachedInputTokens"])
        .or_else(|| {
            value
                .get("input_tokens_details")
                .and_then(|details| json_u64(details, &["cached_tokens"]))
        })
        .or_else(|| {
            value
                .get("prompt_tokens_details")
                .and_then(|details| json_u64(details, &["cached_tokens"]))
        })
        .unwrap_or_default();
    let cache_write_input_tokens = json_u64(
        value,
        &[
            "cache_write_tokens",
            "cacheWriteTokens",
            "cache_write_input_tokens",
        ],
    )
    .or_else(|| {
        value
            .get("input_tokens_details")
            .and_then(|details| json_u64(details, &["cache_write_tokens"]))
    })
    .or_else(|| {
        value
            .get("prompt_tokens_details")
            .and_then(|details| json_u64(details, &["cache_write_tokens"]))
    })
    .unwrap_or_default();
    let output_tokens = json_u64(
        value,
        &["output_tokens", "outputTokens", "completion_tokens"],
    )
    .unwrap_or_default();
    let reasoning_output_tokens =
        json_u64(value, &["reasoning_output_tokens", "reasoningOutputTokens"])
            .or_else(|| {
                value
                    .get("output_tokens_details")
                    .and_then(|details| json_u64(details, &["reasoning_tokens"]))
            })
            .or_else(|| {
                value
                    .get("completion_tokens_details")
                    .and_then(|details| json_u64(details, &["reasoning_tokens"]))
            })
            .unwrap_or_default();
    // OpenAI reports cached input inside input_tokens and reasoning inside
    // output_tokens, so neither bucket should be added a second time.
    let total_tokens = json_u64(value, &["total_tokens", "totalTokens"])
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

    Some(CodexRawUsage {
        input_tokens,
        cache_write_input_tokens,
        cached_input_tokens,
        output_tokens,
        reasoning_output_tokens,
        total_tokens,
    })
}

fn subtract_codex_raw_usage(
    current: &CodexRawUsage,
    previous: Option<&CodexRawUsage>,
) -> CodexRawUsage {
    let Some(previous) = previous else {
        return current.clone();
    };
    let counters_advanced = current.input_tokens >= previous.input_tokens
        && current.cache_write_input_tokens >= previous.cache_write_input_tokens
        && current.cached_input_tokens >= previous.cached_input_tokens
        && current.output_tokens >= previous.output_tokens
        && current.reasoning_output_tokens >= previous.reasoning_output_tokens
        && current.total_tokens >= previous.total_tokens;
    if !counters_advanced {
        return current.clone();
    }

    CodexRawUsage {
        input_tokens: current.input_tokens - previous.input_tokens,
        cache_write_input_tokens: current.cache_write_input_tokens
            - previous.cache_write_input_tokens,
        cached_input_tokens: current.cached_input_tokens - previous.cached_input_tokens,
        output_tokens: current.output_tokens - previous.output_tokens,
        reasoning_output_tokens: current.reasoning_output_tokens - previous.reasoning_output_tokens,
        total_tokens: current.total_tokens - previous.total_tokens,
    }
}

fn has_usage(usage: &CodexRawUsage) -> bool {
    usage.input_tokens > 0
        || usage.cache_write_input_tokens > 0
        || usage.cached_input_tokens > 0
        || usage.output_tokens > 0
        || usage.reasoning_output_tokens > 0
}

fn codex_usage_to_token_usage(usage: &CodexRawUsage) -> TokenUsage {
    let cached_input_tokens = usage.cached_input_tokens.min(usage.input_tokens);
    let remaining_input_tokens = usage.input_tokens.saturating_sub(cached_input_tokens);
    let cache_write_input_tokens = usage.cache_write_input_tokens.min(remaining_input_tokens);
    TokenUsage {
        input_tokens: remaining_input_tokens.saturating_sub(cache_write_input_tokens),
        output_tokens: usage.output_tokens,
        cache_creation_tokens: cache_write_input_tokens,
        cache_read_tokens: cached_input_tokens,
        total_cost: 0.0,
        fast_mode_cost: 0.0,
    }
}

fn json_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_u64))
}

fn string_field(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn codex_timestamp_from_result(value: &Value) -> Option<String> {
    ["timestamp", "created_at", "createdAt"]
        .into_iter()
        .find_map(|key| normalize_codex_timestamp(value.get(key)))
        .or_else(|| {
            value
                .get("data")
                .and_then(codex_timestamp_from_result_object)
        })
        .or_else(|| {
            value
                .get("result")
                .and_then(codex_timestamp_from_result_object)
        })
        .or_else(|| {
            value
                .get("response")
                .and_then(codex_timestamp_from_result_object)
        })
}

fn codex_timestamp_from_result_object(value: &Value) -> Option<String> {
    ["timestamp", "created_at", "createdAt"]
        .into_iter()
        .find_map(|key| normalize_codex_timestamp(value.get(key)))
}

fn normalize_codex_timestamp(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Number(n) => n.as_i64().map(|seconds| {
            DateTime::<Utc>::from_timestamp(seconds, 0)
                .unwrap_or_else(Utc::now)
                .to_rfc3339()
        }),
        _ => None,
    }
}

fn parse_codex_datetime(timestamp: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn file_modified_timestamp(path: &Path) -> String {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now())
        .to_rfc3339()
}

fn date_key(date: &str) -> String {
    date.replace('-', "")
}

fn parse_date_key(date: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&date_key(date), "%Y%m%d").ok()
}

fn parse_timezone(timezone: Option<&str>) -> Result<Tz> {
    match timezone {
        None | Some("") | Some("local") | Some("Local") => {
            let local_name = std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string());
            Ok(local_name.parse::<Tz>().unwrap_or(chrono_tz::UTC))
        }
        Some(tz) => tz
            .parse::<Tz>()
            .with_context(|| format!("Invalid timezone '{}'", tz)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{Duration as StdDuration, Instant};
    use tempfile::TempDir;

    fn create_session_file(root: &Path, date: &str, name: &str) -> PathBuf {
        let day_dir = root
            .join("sessions")
            .join(&date[0..4])
            .join(&date[4..6])
            .join(&date[6..8]);
        fs::create_dir_all(&day_dir).expect("Failed to create Codex day dir");
        let file_path = day_dir.join(name);
        let mut file = File::create(&file_path).expect("Failed to create Codex session file");
        writeln!(
            file,
            r#"{{"type":"event_msg","timestamp":"{}-{}-{}T12:00:00Z","payload":{{"type":"token_count","info":{{"last_token_usage":{{"input_tokens":10,"output_tokens":5}}}}}}}}"#,
            &date[0..4],
            &date[4..6],
            &date[6..8]
        )
        .expect("Failed to write Codex session file");
        file_path
    }

    #[test]
    fn test_codex_date_filtered_file_discovery_performance() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path();
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        for offset in 0..180 {
            let date = base + Duration::days(offset);
            create_session_file(
                root,
                &date.format("%Y%m%d").to_string(),
                &format!("rollout-{}T12-00-00-test.jsonl", date.format("%Y-%m-%d")),
            );
        }

        let start = Instant::now();
        let files =
            codex_jsonl_files(root, Some("20260521"), Some("20260521")).expect("scan succeeds");
        let elapsed = start.elapsed();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("/2026/05/21/"));
        assert!(
            elapsed < StdDuration::from_millis(200),
            "date-filtered Codex discovery took {elapsed:?}"
        );
    }

    #[test]
    fn test_date_key_from_codex_path() {
        let path =
            Path::new("/tmp/.codex/sessions/2026/05/21/rollout-2026-05-21T12-00-00-test.jsonl");
        assert_eq!(date_key_from_path(path).as_deref(), Some("20260521"));
        assert_eq!(codex_session_id_from_filename(path), Some("test"));
    }

    #[test]
    fn string_interner_reuses_compact_ids() {
        let mut interner = StringInterner::default();

        assert_eq!(interner.intern("session-a"), 0);
        assert_eq!(interner.intern("model-a"), 1);
        assert_eq!(interner.intern("session-a"), 0);
        let utc_timestamp = interner.intern("2026-07-10T08:00:00Z");
        let offset_timestamp = interner.intern("2026-07-10T09:00:00+01:00");
        assert_ne!(utc_timestamp, offset_timestamp);
        assert_eq!(interner.indexes.len(), 4);
    }

    #[test]
    fn session_metadata_reads_a_complete_large_header() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path().join("large-header.jsonl");
        let mut file = File::create(&path).expect("Failed to create Codex fixture");
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "type": "session_meta",
                "payload": {
                    "id": "child",
                    "forked_from_id": "parent",
                    "large_ignored_field": "x".repeat(300 * 1024)
                }
            })
        )
        .expect("Failed to write large metadata");
        drop(file);

        let metadata = read_codex_session_metadata(&path).expect("metadata scan should succeed");
        assert_eq!(metadata.session_id.as_deref(), Some("child"));
        assert_eq!(metadata.parent_id.as_deref(), Some("parent"));
    }

    #[test]
    fn session_metadata_scan_is_limited_to_the_rollout_header() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path().join("headless.jsonl");
        let mut file = File::create(&path).expect("Failed to create Codex fixture");
        for _ in 0..METADATA_SCAN_LINES {
            writeln!(file, r#"{{"type":"event_msg"}}"#).expect("Failed to write Codex fixture");
        }
        writeln!(
            file,
            r#"{{"type":"session_meta","payload":{{"id":"late","forked_from_id":"parent"}}}}"#
        )
        .expect("Failed to write late metadata");
        drop(file);

        let metadata = read_codex_session_metadata(&path).expect("metadata scan should succeed");
        assert!(metadata.session_id.is_none());
        assert!(metadata.parent_id.is_none());
    }

    #[test]
    fn test_normalize_openai_prompt_cache_details() {
        let value: Value = serde_json::from_str(
            r#"{
                "prompt_tokens": 2006,
                "completion_tokens": 300,
                "cache_write_tokens": 40,
                "prompt_tokens_details": {
                    "cached_tokens": 1920
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 12
                }
            }"#,
        )
        .expect("Failed to parse fixture");

        let usage = normalize_codex_raw_usage(&value).expect("Expected usage");

        assert_eq!(usage.input_tokens, 2006);
        assert_eq!(usage.cache_write_input_tokens, 40);
        assert_eq!(usage.cached_input_tokens, 1920);
        assert_eq!(usage.output_tokens, 300);
        assert_eq!(usage.reasoning_output_tokens, 12);
        assert_eq!(usage.total_tokens, 2306);
        let normalized = codex_usage_to_token_usage(&usage);
        assert_eq!(normalized.input_tokens, 46);
        assert_eq!(normalized.cache_creation_tokens, 40);
        assert_eq!(normalized.cache_read_tokens, 1920);
        assert_eq!(normalized.output_tokens, 300);
    }

    #[test]
    fn aggregates_codex_events_into_daily_session_and_model_audits() {
        let mut result = CodexUsageData::default();
        let event = CodexUsageEvent {
            session_id: "session-1".to_string(),
            timestamp: "2026-07-10T08:00:00Z".to_string(),
            model: "gpt-5.6-sol".to_string(),
            usage: CodexRawUsage {
                input_tokens: 1_000,
                cache_write_input_tokens: 0,
                cached_input_tokens: 100,
                output_tokens: 50,
                reasoning_output_tokens: 10,
                total_tokens: 1_050,
            },
        };
        let timestamp = parse_codex_datetime(&event.timestamp).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 7, 10).unwrap();
        let pricing = get_fallback_pricing();

        accumulate_codex_event(
            &mut result,
            &event,
            timestamp,
            date,
            &pricing,
            &PricingFetcher::new(),
            true,
        );

        let daily = result.daily.get(&date).unwrap();
        let session = &result.sessions.get("Codex/session-1").unwrap().0;
        let model = result
            .models
            .get(&("Codex".to_string(), "gpt-5.6-sol".to_string()))
            .unwrap();
        for usage in [daily, session, model] {
            assert_eq!(usage.input_tokens, 900);
            assert_eq!(usage.cache_read_tokens, 100);
            assert_eq!(usage.output_tokens, 50);
            assert!((usage.total_cost - 0.00605).abs() < 1e-10);
        }
    }

    #[test]
    fn prices_explicit_gpt_cache_writes_separately() {
        let mut result = CodexUsageData::default();
        let event = CodexUsageEvent {
            session_id: "cache-write".to_string(),
            timestamp: "2026-07-10T08:00:00Z".to_string(),
            model: "gpt-5.6-sol".to_string(),
            usage: CodexRawUsage {
                input_tokens: 1_000,
                cache_write_input_tokens: 200,
                cached_input_tokens: 100,
                output_tokens: 50,
                reasoning_output_tokens: 0,
                total_tokens: 1_050,
            },
        };
        accumulate_codex_event(
            &mut result,
            &event,
            parse_codex_datetime(&event.timestamp).unwrap(),
            NaiveDate::from_ymd_opt(2026, 7, 10).unwrap(),
            &get_fallback_pricing(),
            &PricingFetcher::new(),
            true,
        );

        let usage = result
            .models
            .get(&("Codex".to_string(), "gpt-5.6-sol".to_string()))
            .unwrap();
        assert_eq!(usage.input_tokens, 700);
        assert_eq!(usage.cache_creation_tokens, 200);
        assert_eq!(usage.cache_read_tokens, 100);
        assert!((usage.total_cost - 0.0063).abs() < 1e-10);
    }

    #[test]
    fn skips_repeated_last_usage_when_cumulative_total_is_unchanged() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("session.jsonl");
        let mut file = File::create(&file_path).expect("Failed to create session file");
        let usage = serde_json::json!({
            "last_token_usage": {
                "input_tokens":100,
                "cached_input_tokens":20,
                "output_tokens":10,
                "total_tokens":110
            },
            "total_token_usage": {
                "input_tokens":100,
                "cached_input_tokens":20,
                "output_tokens":10,
                "total_tokens":110
            },
            "model":"gpt-5.6-sol"
        });
        let event = |timestamp: &str| {
            serde_json::json!({
                "timestamp": timestamp,
                "type": "event_msg",
                "payload": {"type": "token_count", "info": usage.clone()}
            })
        };
        writeln!(file, "{}", event("2026-07-10T08:00:00Z"))
            .expect("Failed to write first usage event");
        writeln!(file, "{}", event("2026-07-10T08:00:01Z"))
            .expect("Failed to write repeated usage event");
        drop(file);

        let events =
            read_codex_session_file(temp_dir.path(), &file_path).expect("session should parse");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].usage.input_tokens, 100);
    }

    #[test]
    fn skips_forked_parent_usage_replayed_across_multiple_seconds() {
        fn token_count(timestamp: &str, input: u64, total_input: u64) -> Value {
            serde_json::json!({
                "timestamp": timestamp,
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "info": {
                        "model": "gpt-5.6-sol",
                        "last_token_usage": {
                            "input_tokens": input,
                            "output_tokens": 1,
                            "total_tokens": input + 1
                        },
                        "total_token_usage": {
                            "input_tokens": total_input,
                            "output_tokens": 1,
                            "total_tokens": total_input + 1
                        }
                    }
                }
            })
        }

        fn write_jsonl(path: &Path, entries: &[Value]) {
            let mut file = File::create(path).expect("Failed to create Codex fixture");
            for entry in entries {
                writeln!(file, "{entry}").expect("Failed to write Codex fixture");
            }
        }

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let parent_path = temp_dir
            .path()
            .join("rollout-2026-07-10T08-00-00-parent.jsonl");
        let child_path = temp_dir
            .path()
            .join("rollout-2026-07-10T09-00-00-child.jsonl");
        write_jsonl(
            &parent_path,
            &[
                serde_json::json!({
                    "timestamp": "2026-07-10T08:00:00Z",
                    "type": "session_meta",
                    "payload": {"id": "parent"}
                }),
                token_count("2026-07-10T08:01:00Z", 100, 100),
                token_count("2026-07-10T08:02:00Z", 200, 300),
            ],
        );
        write_jsonl(
            &child_path,
            &[
                serde_json::json!({
                    "timestamp": "2026-07-10T09:00:00Z",
                    "type": "session_meta",
                    "payload": {"id": "child", "forked_from_id": "parent"}
                }),
                token_count("2026-07-10T09:00:00.100Z", 100, 100),
                token_count("2026-07-10T09:00:01.100Z", 200, 300),
                token_count("2026-07-10T09:00:02.100Z", 50, 350),
            ],
        );

        let files = vec![parent_path.clone(), child_path.clone()];
        let selected_files = vec![child_path.clone()];
        let replay_plan = CodexReplayPlan::new(temp_dir.path(), &files, &selected_files);
        let parent_events =
            read_codex_session_file_with_replay(temp_dir.path(), &parent_path, None)
                .expect("parent should parse");
        let child_events = read_codex_session_file_with_replay(
            temp_dir.path(),
            &child_path,
            replay_plan.replayed_prefix(&child_path),
        )
        .expect("child should parse");

        assert_eq!(parent_events.len(), 2);
        assert_eq!(child_events.len(), 1);
        assert_eq!(child_events[0].usage.input_tokens, 50);
        assert_eq!(
            parent_events
                .iter()
                .chain(&child_events)
                .map(|event| event.usage.input_tokens)
                .sum::<u64>(),
            350
        );
    }

    #[test]
    fn preserves_child_usage_when_replay_prefix_does_not_fully_match() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("child.jsonl");
        let mut file = File::create(&file_path).expect("Failed to create Codex fixture");
        for (timestamp, input_tokens) in
            [("2026-07-10T09:00:00Z", 100), ("2026-07-10T09:00:01Z", 50)]
        {
            writeln!(
                file,
                "{}",
                serde_json::json!({
                    "timestamp": timestamp,
                    "model": "gpt-5.6-sol",
                    "usage": {
                        "input_tokens": input_tokens,
                        "output_tokens": 1,
                        "total_tokens": input_tokens + 1
                    }
                })
            )
            .expect("Failed to write Codex fixture");
        }
        drop(file);
        let unmatched_prefix = [CodexRawUsage {
            input_tokens: 999,
            cache_write_input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 1,
            reasoning_output_tokens: 0,
            total_tokens: 1_000,
        }];

        let events = read_codex_session_file_with_replay(
            temp_dir.path(),
            &file_path,
            Some(&unmatched_prefix),
        )
        .expect("child should parse");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].usage.input_tokens, 100);
        assert_eq!(events[1].usage.input_tokens, 50);
    }

    #[test]
    fn includes_long_running_session_that_started_before_requested_day() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let today = Utc::now().date_naive();
        let started = today - Duration::days(10);
        let path = temp_dir.path().join(format!(
            "rollout-{}T08-00-00-long-running.jsonl",
            started.format("%Y-%m-%d")
        ));
        let mut file = File::create(&path).expect("Failed to create Codex fixture");
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "timestamp": format!("{}T08:00:00Z", today.format("%Y-%m-%d")),
                "model": "gpt-5.6-sol",
                "usage": {"input_tokens": 10, "output_tokens": 1}
            })
        )
        .expect("Failed to write Codex fixture");
        drop(file);
        let requested = today.format("%Y%m%d").to_string();

        assert!(
            codex_file_may_overlap(&path, Some(&requested), Some(&requested))
                .expect("candidate check should succeed")
        );
    }

    #[test]
    fn excludes_copied_old_session_using_last_event_timestamp() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let today = Utc::now().date_naive();
        let started = today - Duration::days(10);
        let path = temp_dir.path().join(format!(
            "rollout-{}T08-00-00-old.jsonl",
            started.format("%Y-%m-%d")
        ));
        let mut file = File::create(&path).expect("Failed to create Codex fixture");
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "timestamp": format!("{}T09:00:00Z", started.format("%Y-%m-%d")),
                "model": "gpt-5.6-sol",
                "usage": {"input_tokens": 10, "output_tokens": 1}
            })
        )
        .expect("Failed to write Codex fixture");
        drop(file);
        let requested = today.format("%Y%m%d").to_string();

        assert!(
            !codex_file_may_overlap(&path, Some(&requested), Some(&requested))
                .expect("candidate check should succeed")
        );
    }

    #[test]
    fn uses_top_level_timestamp_instead_of_nested_payload_timestamp() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path().join("nested-timestamp.jsonl");
        let mut file = File::create(&path).expect("Failed to create Codex fixture");
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "timestamp": "2026-07-26T10:00:00Z",
                "payload": {"created_at": "2026-07-10T09:00:00Z"}
            })
        )
        .expect("Failed to write Codex fixture");
        drop(file);

        assert_eq!(
            codex_last_activity_date(&path).expect("tail should parse"),
            Some(NaiveDate::from_ymd_opt(2026, 7, 26).unwrap())
        );
    }

    #[test]
    fn treats_counter_regression_as_a_new_snapshot() {
        let previous = CodexRawUsage {
            input_tokens: 500,
            cache_write_input_tokens: 0,
            cached_input_tokens: 200,
            output_tokens: 50,
            reasoning_output_tokens: 10,
            total_tokens: 550,
        };
        let reset = CodexRawUsage {
            input_tokens: 40,
            cache_write_input_tokens: 0,
            cached_input_tokens: 10,
            output_tokens: 5,
            reasoning_output_tokens: 1,
            total_tokens: 45,
        };

        assert_eq!(subtract_codex_raw_usage(&reset, Some(&previous)), reset);
    }

    #[test]
    fn unknown_model_is_not_charged_at_a_default_rate() {
        let mut result = CodexUsageData::default();
        let event = CodexUsageEvent {
            session_id: "unknown-model".to_string(),
            timestamp: "2026-07-10T08:00:00Z".to_string(),
            model: UNKNOWN_CODEX_MODEL.to_string(),
            usage: CodexRawUsage {
                input_tokens: 1_000,
                cache_write_input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 100,
                reasoning_output_tokens: 0,
                total_tokens: 1_100,
            },
        };
        accumulate_codex_event(
            &mut result,
            &event,
            parse_codex_datetime(&event.timestamp).unwrap(),
            NaiveDate::from_ymd_opt(2026, 7, 10).unwrap(),
            &get_fallback_pricing(),
            &PricingFetcher::new(),
            true,
        );

        assert_eq!(
            result
                .models
                .get(&("Codex".to_string(), UNKNOWN_CODEX_MODEL.to_string()))
                .unwrap()
                .total_cost,
            0.0
        );
    }

    #[test]
    fn display_cost_mode_keeps_codex_calculated_cost_at_zero() {
        let mut result = CodexUsageData::default();
        let event = CodexUsageEvent {
            session_id: "display-cost".to_string(),
            timestamp: "2026-07-10T08:00:00Z".to_string(),
            model: "gpt-5.6-sol".to_string(),
            usage: CodexRawUsage {
                input_tokens: 1_000,
                cache_write_input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 100,
                reasoning_output_tokens: 0,
                total_tokens: 1_100,
            },
        };
        accumulate_codex_event(
            &mut result,
            &event,
            parse_codex_datetime(&event.timestamp).unwrap(),
            NaiveDate::from_ymd_opt(2026, 7, 10).unwrap(),
            &get_fallback_pricing(),
            &PricingFetcher::new(),
            false,
        );

        assert_eq!(
            result
                .models
                .get(&("Codex".to_string(), "gpt-5.6-sol".to_string()))
                .unwrap()
                .total_cost,
            0.0
        );
    }
}
