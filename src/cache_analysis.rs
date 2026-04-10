use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, Utc};
use colored::Colorize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheSortField {
    WriteCost,
    HitRate,
    Miss5m,
    Miss60m,
    ColdStart,
    NormalChurn,
    BreakevenTurn,
}

#[derive(Debug, Clone)]
struct CacheTurn {
    timestamp: DateTime<Utc>,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    ephemeral_5m_tokens: u64,
    ephemeral_1h_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionCacheAnalysis {
    pub session_id: String,
    pub project: String,
    pub warmup_turn: Option<usize>,
    pub breakeven_turn: Option<usize>,
    pub hit_rate_pct: f64,
    pub cold_start_tokens: u64,
    pub ttl_5m_miss_tokens: u64,
    pub ttl_60m_miss_tokens: u64,
    pub normal_churn_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub turn_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheAggregate {
    pub project: String,
    pub session_count: usize,
    pub total_writes: u64,
    pub total_reads: u64,
    pub hit_rate_pct: f64,
    pub total_cold_start: u64,
    pub total_5m_miss: u64,
    pub total_60m_miss: u64,
    pub total_normal_churn: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheAnalysis {
    pub sessions: Vec<SessionCacheAnalysis>,
    pub total_cold_start: u64,
    pub total_5m_miss: u64,
    pub total_60m_miss: u64,
    pub total_normal_churn: u64,
    pub total_cache_writes: u64,
    pub total_cache_reads: u64,
    pub avg_warmup_turn: f64,
    pub avg_breakeven_turn: f64,
    pub project_aggregates: Vec<ProjectCacheAggregate>,
}

fn parse_cache_turns(file_path: &Path) -> Vec<CacheTurn> {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = BufReader::new(file);
    let mut turns = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }

        let record: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if record.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let message = match record.get("message") {
            Some(m) => m,
            None => continue,
        };

        let usage = match message.get("usage") {
            Some(u) => u,
            None => continue,
        };

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if output_tokens == 0 {
            continue;
        }

        let timestamp = match record
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.with_timezone(&Utc))
        {
            Some(ts) => ts,
            None => continue,
        };

        let cache_creation_tokens = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cache_read_tokens = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let ephemeral_5m_tokens = usage
            .get("cache_creation")
            .and_then(|c| c.get("ephemeral_5m_input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let ephemeral_1h_tokens = usage
            .get("cache_creation")
            .and_then(|c| c.get("ephemeral_1h_input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        turns.push(CacheTurn {
            timestamp,
            cache_creation_tokens,
            cache_read_tokens,
            ephemeral_5m_tokens,
            ephemeral_1h_tokens,
        });
    }

    turns.sort_by_key(|t| t.timestamp);
    turns
}

fn compute_warmup_turn(turns: &[CacheTurn], threshold: f64) -> Option<usize> {
    if turns.len() < 3 {
        return None;
    }

    for i in 0..=(turns.len() - 3) {
        let all_hit = turns[i..i + 3].iter().all(|t| {
            let total = t.cache_read_tokens + t.cache_creation_tokens;
            if total == 0 {
                return false;
            }
            t.cache_read_tokens as f64 / total as f64 > threshold
        });
        if all_hit {
            return Some(i);
        }
    }

    None
}

fn compute_breakeven_turn(turns: &[CacheTurn]) -> Option<usize> {
    let mut cumulative_write_cost = 0.0_f64;
    let mut cumulative_read_savings = 0.0_f64;

    for (i, turn) in turns.iter().enumerate() {
        cumulative_write_cost += write_cost(turn.cache_creation_tokens);
        // Savings = what reads would have cost as writes minus actual read cost
        cumulative_read_savings +=
            write_cost(turn.cache_read_tokens) - read_cost(turn.cache_read_tokens);

        if cumulative_write_cost > 0.0 && cumulative_read_savings >= cumulative_write_cost {
            return Some(i);
        }
    }

    None
}

fn classify_session(
    turns: &[CacheTurn],
    session_id: &str,
    project: &str,
    threshold: f64,
) -> SessionCacheAnalysis {
    let warmup_turn = compute_warmup_turn(turns, threshold);
    let breakeven_turn = compute_breakeven_turn(turns);

    let mut cold_start_tokens = 0u64;
    let mut ttl_5m_miss_tokens = 0u64;
    let mut ttl_60m_miss_tokens = 0u64;
    let mut normal_churn_tokens = 0u64;
    let mut total_cache_write_tokens = 0u64;
    let mut total_cache_read_tokens = 0u64;

    for (i, turn) in turns.iter().enumerate() {
        total_cache_write_tokens += turn.cache_creation_tokens;
        total_cache_read_tokens += turn.cache_read_tokens;

        let write_tokens = turn.cache_creation_tokens;
        if write_tokens == 0 {
            continue;
        }

        match warmup_turn {
            None => {
                cold_start_tokens += write_tokens;
            }
            Some(bp) if i < bp => {
                cold_start_tokens += write_tokens;
            }
            _ => {
                if i == 0 {
                    cold_start_tokens += write_tokens;
                    continue;
                }
                let prev = &turns[i - 1];
                let gap_minutes = (turn.timestamp - prev.timestamp).num_minutes();

                if gap_minutes > 60 {
                    if turn.ephemeral_1h_tokens > 0 || turn.ephemeral_5m_tokens == 0 {
                        ttl_60m_miss_tokens += write_tokens;
                    } else {
                        ttl_5m_miss_tokens += write_tokens;
                    }
                } else if gap_minutes > 5 {
                    if turn.ephemeral_5m_tokens > 0 || turn.ephemeral_1h_tokens == 0 {
                        ttl_5m_miss_tokens += write_tokens;
                    } else {
                        normal_churn_tokens += write_tokens;
                    }
                } else {
                    normal_churn_tokens += write_tokens;
                }
            }
        }
    }

    let total = total_cache_write_tokens + total_cache_read_tokens;
    let hit_rate_pct = if total > 0 {
        total_cache_read_tokens as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    SessionCacheAnalysis {
        session_id: session_id.to_string(),
        project: project.to_string(),
        warmup_turn,
        breakeven_turn,
        hit_rate_pct,
        cold_start_tokens,
        ttl_5m_miss_tokens,
        ttl_60m_miss_tokens,
        normal_churn_tokens,
        total_cache_write_tokens,
        total_cache_read_tokens,
        turn_count: turns.len(),
    }
}

fn extract_project_from_file(file_path: &Path) -> String {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return String::from("unknown"),
    };
    let reader = BufReader::new(file);

    for line in reader.lines().take(50) {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let record: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(cwd) = record.get("cwd").and_then(|v| v.as_str()) {
            let path = Path::new(cwd);
            let display = if let Ok(rel) = path.strip_prefix(&home) {
                format!("~/{}", rel.display())
            } else {
                cwd.to_string()
            };
            return display;
        }
    }

    String::from("unknown")
}

fn build_project_aggregates(sessions: &[SessionCacheAnalysis]) -> Vec<ProjectCacheAggregate> {
    let mut map: HashMap<String, ProjectCacheAggregate> = HashMap::new();

    for s in sessions {
        let agg = map
            .entry(s.project.clone())
            .or_insert_with(|| ProjectCacheAggregate {
                project: s.project.clone(),
                session_count: 0,
                total_writes: 0,
                total_reads: 0,
                hit_rate_pct: 0.0,
                total_cold_start: 0,
                total_5m_miss: 0,
                total_60m_miss: 0,
                total_normal_churn: 0,
            });
        agg.session_count += 1;
        agg.total_writes += s.total_cache_write_tokens;
        agg.total_reads += s.total_cache_read_tokens;
        agg.total_cold_start += s.cold_start_tokens;
        agg.total_5m_miss += s.ttl_5m_miss_tokens;
        agg.total_60m_miss += s.ttl_60m_miss_tokens;
        agg.total_normal_churn += s.normal_churn_tokens;
    }

    let mut aggregates: Vec<ProjectCacheAggregate> = map
        .into_values()
        .map(|mut agg| {
            let total = agg.total_writes + agg.total_reads;
            agg.hit_rate_pct = if total > 0 {
                agg.total_reads as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            agg
        })
        .collect();

    aggregates.sort_by(|a, b| {
        b.total_writes
            .partial_cmp(&a.total_writes)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    aggregates
}

pub fn analyze_cache(
    claude_dir: &Path,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    project_filter: Option<&str>,
    warmup_threshold: f64,
) -> Result<CacheAnalysis> {
    let projects_dir = claude_dir.join("projects");

    if !projects_dir.exists() {
        anyhow::bail!(
            "Claude projects directory not found at {}",
            projects_dir.display()
        );
    }

    let mut sessions = Vec::new();

    let jsonl_files: Vec<PathBuf> = WalkDir::new(&projects_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "jsonl")
                .unwrap_or(false)
        })
        .filter(|e| !e.path().to_string_lossy().contains("/subagents/"))
        .map(|e| e.path().to_path_buf())
        .collect();

    for file_path in &jsonl_files {
        if let Some(since_date) = since
            && let Some(modified) = std::fs::metadata(file_path)
                .ok()
                .and_then(|m| m.modified().ok())
        {
            let modified_date: NaiveDate = chrono::DateTime::<Local>::from(modified).date_naive();
            if modified_date < since_date {
                continue;
            }
        }

        let turns = parse_cache_turns(file_path);
        if turns.is_empty() {
            continue;
        }

        let turns: Vec<CacheTurn> = turns
            .into_iter()
            .filter(|t| {
                let date = t.timestamp.with_timezone(&Local).date_naive();
                if let Some(s) = since
                    && date < s
                {
                    return false;
                }
                if let Some(u) = until
                    && date > u
                {
                    return false;
                }
                true
            })
            .collect();

        if turns.is_empty() {
            continue;
        }

        let project = extract_project_from_file(file_path);

        if let Some(filter) = project_filter
            && !project.contains(filter)
        {
            continue;
        }

        let session_id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let analysis = classify_session(&turns, &session_id, &project, warmup_threshold);
        sessions.push(analysis);
    }

    let total_cold_start: u64 = sessions.iter().map(|s| s.cold_start_tokens).sum();
    let total_5m_miss: u64 = sessions.iter().map(|s| s.ttl_5m_miss_tokens).sum();
    let total_60m_miss: u64 = sessions.iter().map(|s| s.ttl_60m_miss_tokens).sum();
    let total_normal_churn: u64 = sessions.iter().map(|s| s.normal_churn_tokens).sum();
    let total_cache_writes: u64 = sessions.iter().map(|s| s.total_cache_write_tokens).sum();
    let total_cache_reads: u64 = sessions.iter().map(|s| s.total_cache_read_tokens).sum();

    let warmup_turns: Vec<usize> = sessions.iter().filter_map(|s| s.warmup_turn).collect();
    let avg_warmup_turn = if warmup_turns.is_empty() {
        0.0
    } else {
        warmup_turns.iter().sum::<usize>() as f64 / warmup_turns.len() as f64
    };

    let breakeven_turns: Vec<usize> = sessions.iter().filter_map(|s| s.breakeven_turn).collect();
    let avg_breakeven_turn = if breakeven_turns.is_empty() {
        0.0
    } else {
        breakeven_turns.iter().sum::<usize>() as f64 / breakeven_turns.len() as f64
    };

    let project_aggregates = build_project_aggregates(&sessions);

    Ok(CacheAnalysis {
        sessions,
        total_cold_start,
        total_5m_miss,
        total_60m_miss,
        total_normal_churn,
        total_cache_writes,
        total_cache_reads,
        avg_warmup_turn,
        avg_breakeven_turn,
        project_aggregates,
    })
}

#[derive(Debug, Serialize)]
pub struct CacheStatsOutput {
    pub cold_pct: f64,
    pub hit_pct: f64,
    pub churn_tokens_per_turn: Option<u64>,
    pub turn_count: usize,
    pub total_tokens: u64,
    pub cache_read_tokens: u64,
}

pub fn compute_session_cache_stats(file_path: &Path, window: usize) -> CacheStatsOutput {
    struct TurnTokens {
        input: u64,
        creation: u64,
        read: u64,
    }

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => {
            return CacheStatsOutput {
                cold_pct: 0.0,
                hit_pct: 0.0,
                churn_tokens_per_turn: None,
                turn_count: 0,
                total_tokens: 0,
                cache_read_tokens: 0,
            };
        }
    };
    let reader = BufReader::new(file);
    let mut turns: Vec<TurnTokens> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let record: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if record.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let message = match record.get("message") {
            Some(m) => m,
            None => continue,
        };
        let usage = match message.get("usage") {
            Some(u) => u,
            None => continue,
        };
        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if output_tokens == 0 {
            continue;
        }
        let input = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let creation = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let read = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        turns.push(TurnTokens {
            input,
            creation,
            read,
        });
    }

    let turn_count = turns.len();
    let total_input: u64 = turns
        .iter()
        .fold(0u64, |acc, t| acc.saturating_add(t.input));
    let total_creation: u64 = turns
        .iter()
        .fold(0u64, |acc, t| acc.saturating_add(t.creation));
    let total_read: u64 = turns.iter().fold(0u64, |acc, t| acc.saturating_add(t.read));
    let total = total_input
        .saturating_add(total_creation)
        .saturating_add(total_read);

    let cold_pct = if total > 0 {
        total_input as f64 / total as f64
    } else {
        0.0
    };
    let hit_pct = if total > 0 {
        total_read as f64 / total as f64
    } else {
        0.0
    };

    let churn_tokens_per_turn = if turn_count >= window {
        let window_creation: u64 = turns[turn_count - window..]
            .iter()
            .fold(0u64, |acc, t| acc.saturating_add(t.creation));
        Some(window_creation.saturating_div(window as u64))
    } else {
        None
    };

    CacheStatsOutput {
        cold_pct,
        hit_pct,
        churn_tokens_per_turn,
        turn_count,
        total_tokens: total,
        cache_read_tokens: total_read,
    }
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}

fn write_cost(tokens: u64) -> f64 {
    tokens as f64 * 3.75 / 1_000_000.0
}

fn read_cost(tokens: u64) -> f64 {
    tokens as f64 * 0.30 / 1_000_000.0
}

#[allow(clippy::too_many_arguments)]
pub fn display_cache_analysis(
    analysis: &CacheAnalysis,
    json: bool,
    top: usize,
    top_projects: usize,
    sort_field: CacheSortField,
    warmup_threshold: f64,
    sort_asc_override: Option<bool>,
    min_hit: Option<f64>,
    min_churn: Option<f64>,
) {
    if json {
        match serde_json::to_string_pretty(analysis) {
            Ok(s) => println!("{}", s),
            Err(e) => eprintln!("Failed to serialize: {}", e),
        }
        return;
    }

    println!("\n{}", "CACHE ANALYSIS".bold());
    println!("{}", "═".repeat(56));

    println!("\n{}", "Cache Write Breakdown".bold());
    println!("{}", "─".repeat(56));
    let tw = analysis.total_cache_writes.max(1) as f64;
    println!(
        "  {:<16} {:>8} {:>5.1}%  (${:.2})",
        "Cold start:".cyan(),
        format_tokens(analysis.total_cold_start),
        analysis.total_cold_start as f64 / tw * 100.0,
        write_cost(analysis.total_cold_start)
    );
    println!(
        "  {:<16} {:>8} {:>5.1}%  (${:.2})",
        "5m TTL miss:".yellow(),
        format_tokens(analysis.total_5m_miss),
        analysis.total_5m_miss as f64 / tw * 100.0,
        write_cost(analysis.total_5m_miss)
    );
    println!(
        "  {:<16} {:>8} {:>5.1}%  (${:.2})",
        "60m TTL miss:".yellow(),
        format_tokens(analysis.total_60m_miss),
        analysis.total_60m_miss as f64 / tw * 100.0,
        write_cost(analysis.total_60m_miss)
    );
    println!(
        "  {:<16} {:>8} {:>5.1}%  (${:.2})",
        "Normal churn:".green(),
        format_tokens(analysis.total_normal_churn),
        analysis.total_normal_churn as f64 / tw * 100.0,
        write_cost(analysis.total_normal_churn)
    );
    println!("  {}", "─".repeat(40));
    println!(
        "  {:<16} {:>8}         (${:.2})",
        "Total writes:".bold(),
        format_tokens(analysis.total_cache_writes),
        write_cost(analysis.total_cache_writes)
    );
    println!(
        "  {:<16} {:>8}         (${:.2})",
        "Total reads:".bold(),
        format_tokens(analysis.total_cache_reads),
        read_cost(analysis.total_cache_reads)
    );
    let overall_total = analysis.total_cache_writes + analysis.total_cache_reads;
    let overall_hit_rate = if overall_total > 0 {
        analysis.total_cache_reads as f64 / overall_total as f64 * 100.0
    } else {
        0.0
    };
    println!("  {}", "─".repeat(40));
    println!(
        "  {:<16} {:>7.1}%",
        "Avg hit rate:".bold(),
        overall_hit_rate
    );

    let threshold_pct = (warmup_threshold * 100.0) as u32;
    println!(
        "\n{} {:.1} turns",
        format!("Avg warmup turn (>{}% hit rate):", threshold_pct).bold(),
        analysis.avg_warmup_turn
    );
    println!(
        "{} {:.1} turns",
        "Avg break-even turn:".bold(),
        analysis.avg_breakeven_turn
    );

    // Sessions table
    let top_n = top.min(analysis.sessions.len());
    if top_n > 0 {
        let sort_label = match sort_field {
            CacheSortField::WriteCost => "Cache Write Cost",
            CacheSortField::HitRate => "Cache Hit Rate",
            CacheSortField::Miss5m => "5m TTL Miss",
            CacheSortField::Miss60m => "60m TTL Miss",
            CacheSortField::ColdStart => "Cold Start",
            CacheSortField::NormalChurn => "Normal Churn",
            CacheSortField::BreakevenTurn => "Break-even Turn",
        };

        println!(
            "\n{}",
            format!("Top {} Sessions by {}", top_n, sort_label).bold()
        );
        println!("{}", "─".repeat(99));

        let header = format!(
            "{:<10} {:<22} {:>5} {:>5} {:>4} {:>9} {:>9} {:>9} {:>6} {:>7}",
            "Session",
            "Project",
            "Hit%",
            "Warm",
            "BE",
            "5m Miss",
            "60m Miss",
            "Cold",
            "Churn%",
            "Write$"
        );
        println!("{}", header.bold());
        println!(
            "{} {} {} {} {} {} {} {} {} {}",
            "─".repeat(10),
            "─".repeat(22),
            "─".repeat(5),
            "─".repeat(5),
            "─".repeat(4),
            "─".repeat(9),
            "─".repeat(9),
            "─".repeat(9),
            "─".repeat(6),
            "─".repeat(7)
        );

        let mut sorted_sessions = analysis.sessions.clone();
        sorted_sessions.sort_by(|a, b| {
            let (va, vb) = match sort_field {
                CacheSortField::WriteCost => (
                    write_cost(a.total_cache_write_tokens),
                    write_cost(b.total_cache_write_tokens),
                ),
                CacheSortField::HitRate => (a.hit_rate_pct, b.hit_rate_pct),
                CacheSortField::Miss5m => {
                    (a.ttl_5m_miss_tokens as f64, b.ttl_5m_miss_tokens as f64)
                }
                CacheSortField::Miss60m => {
                    (a.ttl_60m_miss_tokens as f64, b.ttl_60m_miss_tokens as f64)
                }
                CacheSortField::ColdStart => {
                    (a.cold_start_tokens as f64, b.cold_start_tokens as f64)
                }
                CacheSortField::NormalChurn => {
                    let churn_a = if a.total_cache_write_tokens > 0 {
                        a.normal_churn_tokens as f64 / a.total_cache_write_tokens as f64
                    } else {
                        0.0
                    };
                    let churn_b = if b.total_cache_write_tokens > 0 {
                        b.normal_churn_tokens as f64 / b.total_cache_write_tokens as f64
                    } else {
                        0.0
                    };
                    (churn_a, churn_b)
                }
                CacheSortField::BreakevenTurn => (
                    a.breakeven_turn.map(|t| t as f64).unwrap_or(f64::MAX),
                    b.breakeven_turn.map(|t| t as f64).unwrap_or(f64::MAX),
                ),
            };
            let default_asc = matches!(sort_field, CacheSortField::BreakevenTurn);
            let ascending = sort_asc_override.unwrap_or(default_asc);
            if ascending {
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        let filtered_sessions: Vec<&SessionCacheAnalysis> = sorted_sessions
            .iter()
            .filter(|s| {
                if let Some(min) = min_hit
                    && s.hit_rate_pct < min
                {
                    return false;
                }
                if let Some(min) = min_churn {
                    let churn_rate = if s.total_cache_write_tokens > 0 {
                        s.normal_churn_tokens as f64 / s.total_cache_write_tokens as f64 * 100.0
                    } else {
                        0.0
                    };
                    if churn_rate < min {
                        return false;
                    }
                }
                true
            })
            .take(top_n)
            .collect();

        for session in &filtered_sessions {
            let short_id = if session.session_id.len() >= 8 {
                &session.session_id[..8]
            } else {
                &session.session_id
            };

            let hit_pct = if session.hit_rate_pct > 0.0 {
                format!("{:.1}%", session.hit_rate_pct)
            } else {
                "-".to_string()
            };

            let warm = session
                .warmup_turn
                .map(|t| format!("T{}", t))
                .unwrap_or_else(|| "-".to_string());

            let be_val = session.breakeven_turn;
            let be_str = be_val
                .map(|t| format!("T{}", t))
                .unwrap_or_else(|| "-".to_string());
            let be_colored = match be_val {
                Some(t) if t <= 10 => be_str.green().to_string(),
                Some(t) if t <= 20 => be_str.yellow().to_string(),
                Some(_) => be_str.red().to_string(),
                None => be_str.dimmed().to_string(),
            };

            let project_display = if session.project.len() > 21 {
                format!("...{}", &session.project[session.project.len() - 18..])
            } else {
                session.project.clone()
            };

            let churn_pct = if session.total_cache_write_tokens > 0 {
                format!(
                    "{:.0}%",
                    session.normal_churn_tokens as f64 / session.total_cache_write_tokens as f64
                        * 100.0
                )
            } else {
                "-".to_string()
            };

            println!(
                "{:<10} {:<22} {:>5} {:>5} {:>4} {:>9} {:>9} {:>9} {:>6} {:>7}",
                short_id.cyan(),
                project_display,
                hit_pct,
                warm,
                be_colored,
                format_tokens(session.ttl_5m_miss_tokens),
                format_tokens(session.ttl_60m_miss_tokens),
                format_tokens(session.cold_start_tokens),
                churn_pct,
                format!("${:.2}", write_cost(session.total_cache_write_tokens))
            );
        }
    }

    // Project aggregates table
    let top_p = top_projects.min(analysis.project_aggregates.len());
    if top_p > 0 {
        println!(
            "\n{}",
            format!("Top {} Projects by Cache Write Cost", top_p).bold()
        );
        println!("{}", "─".repeat(96));

        let header = format!(
            "{:<24} {:>4} {:>5} {:>8} {:>9} {:>9} {:>6} {:>7}",
            "Project", "Sess", "Hit%", "Cold", "5m Miss", "60m Miss", "Churn%", "Write$"
        );
        println!("{}", header.bold());
        println!(
            "{} {} {} {} {} {} {} {}",
            "─".repeat(24),
            "─".repeat(4),
            "─".repeat(5),
            "─".repeat(8),
            "─".repeat(9),
            "─".repeat(9),
            "─".repeat(6),
            "─".repeat(7)
        );

        for agg in analysis.project_aggregates.iter().take(top_p) {
            let project_display = if agg.project.len() > 23 {
                format!("...{}", &agg.project[agg.project.len() - 20..])
            } else {
                agg.project.clone()
            };

            let churn_pct = if agg.total_writes > 0 {
                format!(
                    "{:.0}%",
                    agg.total_normal_churn as f64 / agg.total_writes as f64 * 100.0
                )
            } else {
                "-".to_string()
            };

            println!(
                "{:<24} {:>4} {:>5} {:>8} {:>9} {:>9} {:>6} {:>7}",
                project_display,
                agg.session_count,
                format!("{:.1}%", agg.hit_rate_pct),
                format_tokens(agg.total_cold_start),
                format_tokens(agg.total_5m_miss),
                format_tokens(agg.total_60m_miss),
                churn_pct,
                format!("${:.2}", write_cost(agg.total_writes))
            );
        }
    }

    println!();
}
