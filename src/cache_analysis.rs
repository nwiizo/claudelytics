#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, Utc};
use colored::Colorize;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    pub balance_point: Option<usize>,
    pub cold_start_tokens: u64,
    pub ttl_5m_miss_tokens: u64,
    pub ttl_60m_miss_tokens: u64,
    pub normal_churn_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub turn_count: usize,
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
    pub avg_balance_point: f64,
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

        // Only assistant messages with usage and output_tokens > 0
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

fn compute_balance_point(turns: &[CacheTurn]) -> Option<usize> {
    // Find first index where cache_read/(cache_read+cache_creation) > 0.9 for 3 consecutive turns
    if turns.len() < 3 {
        return None;
    }

    for i in 0..=(turns.len() - 3) {
        let all_hit = turns[i..i + 3].iter().all(|t| {
            let total = t.cache_read_tokens + t.cache_creation_tokens;
            if total == 0 {
                return false;
            }
            t.cache_read_tokens as f64 / total as f64 > 0.9
        });
        if all_hit {
            return Some(i);
        }
    }

    None
}

fn classify_session(turns: &[CacheTurn], session_id: &str, project: &str) -> SessionCacheAnalysis {
    let balance_point = compute_balance_point(turns);

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

        match balance_point {
            None => {
                // No balance point found, everything is cold start
                cold_start_tokens += write_tokens;
            }
            Some(bp) if i < bp => {
                cold_start_tokens += write_tokens;
            }
            _ => {
                // After balance point: classify by gap from previous turn
                if i == 0 {
                    cold_start_tokens += write_tokens;
                    continue;
                }
                let prev = &turns[i - 1];
                let gap_minutes = (turn.timestamp - prev.timestamp).num_minutes();

                if gap_minutes > 60 {
                    // 60m TTL miss: gap > 60min and ephemeral_1h data present (or fallback: any write after long gap)
                    if turn.ephemeral_1h_tokens > 0 || turn.ephemeral_5m_tokens == 0 {
                        ttl_60m_miss_tokens += write_tokens;
                    } else {
                        ttl_5m_miss_tokens += write_tokens;
                    }
                } else if gap_minutes > 5 {
                    // 5m TTL miss: gap > 5min and ephemeral_5m data present (or fallback)
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

    SessionCacheAnalysis {
        session_id: session_id.to_string(),
        project: project.to_string(),
        balance_point,
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

pub fn analyze_cache(
    claude_dir: &Path,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    project_filter: Option<&str>,
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
        .filter(|e| {
            // Skip subagents directories
            !e.path().to_string_lossy().contains("/subagents/")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    for file_path in &jsonl_files {
        // mtime pre-filter: skip files modified before since date
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

        // Apply date filter on turns
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

        let analysis = classify_session(&turns, &session_id, &project);
        sessions.push(analysis);
    }

    let total_cold_start: u64 = sessions.iter().map(|s| s.cold_start_tokens).sum();
    let total_5m_miss: u64 = sessions.iter().map(|s| s.ttl_5m_miss_tokens).sum();
    let total_60m_miss: u64 = sessions.iter().map(|s| s.ttl_60m_miss_tokens).sum();
    let total_normal_churn: u64 = sessions.iter().map(|s| s.normal_churn_tokens).sum();
    let total_cache_writes: u64 = sessions.iter().map(|s| s.total_cache_write_tokens).sum();
    let total_cache_reads: u64 = sessions.iter().map(|s| s.total_cache_read_tokens).sum();

    let sessions_with_bp: Vec<usize> = sessions.iter().filter_map(|s| s.balance_point).collect();
    let avg_balance_point = if sessions_with_bp.is_empty() {
        0.0
    } else {
        sessions_with_bp.iter().sum::<usize>() as f64 / sessions_with_bp.len() as f64
    };

    Ok(CacheAnalysis {
        sessions,
        total_cold_start,
        total_5m_miss,
        total_60m_miss,
        total_normal_churn,
        total_cache_writes,
        total_cache_reads,
        avg_balance_point,
    })
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

pub fn display_cache_analysis(analysis: &CacheAnalysis, json: bool, top: usize) {
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

    println!(
        "\n{} {:.1} turns",
        "Average cache balance point:".bold(),
        analysis.avg_balance_point
    );

    let top_n = top.min(analysis.sessions.len());
    if top_n == 0 {
        return;
    }

    println!(
        "\n{}",
        format!("Top {} Sessions by Cache Write Cost", top_n).bold()
    );
    println!("{}", "─".repeat(56));

    let header = format!(
        "{:<10} {:<25} {:>5} {:>4} {:>9} {:>9} {:>9} {:>6}",
        "Session", "Project", "Hit%", "Bal", "5m Miss", "60m Miss", "Cold", "Write$"
    );
    println!("{}", header.bold());
    println!(
        "{} {} {} {} {} {} {} {}",
        "─".repeat(10),
        "─".repeat(25),
        "─".repeat(5),
        "─".repeat(4),
        "─".repeat(9),
        "─".repeat(9),
        "─".repeat(9),
        "─".repeat(6)
    );

    let mut sorted_sessions = analysis.sessions.clone();
    sorted_sessions.sort_by(|a, b| {
        write_cost(b.total_cache_write_tokens)
            .partial_cmp(&write_cost(a.total_cache_write_tokens))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for session in sorted_sessions.iter().take(top_n) {
        let short_id = if session.session_id.len() >= 8 {
            &session.session_id[..8]
        } else {
            &session.session_id
        };

        let total = session.total_cache_write_tokens + session.total_cache_read_tokens;
        let hit_pct = if total > 0 {
            format!(
                "{:.1}%",
                session.total_cache_read_tokens as f64 / total as f64 * 100.0
            )
        } else {
            "-".to_string()
        };

        let bal = session
            .balance_point
            .map(|bp| format!("T{}", bp))
            .unwrap_or_else(|| "-".to_string());

        let project_display = if session.project.len() > 24 {
            format!("...{}", &session.project[session.project.len() - 21..])
        } else {
            session.project.clone()
        };

        println!(
            "{:<10} {:<25} {:>5} {:>4} {:>9} {:>9} {:>9} {:>6}",
            short_id.cyan(),
            project_display,
            hit_pct,
            bal,
            format_tokens(session.ttl_5m_miss_tokens),
            format_tokens(session.ttl_60m_miss_tokens),
            format_tokens(session.cold_start_tokens),
            format!("${:.2}", write_cost(session.total_cache_write_tokens))
        );
    }
    println!();
}
