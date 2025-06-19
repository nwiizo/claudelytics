use crate::models::{ClaudeMessage, ClaudeSession, ClaudeSessionSummary, TokenUsage};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Parser for Claude session files
#[allow(dead_code)]
pub struct ClaudeSessionParser {
    claude_path: PathBuf,
}

#[allow(dead_code)]
impl ClaudeSessionParser {
    pub fn new(claude_path: Option<PathBuf>) -> Self {
        let claude_path = claude_path.unwrap_or_else(|| {
            dirs::home_dir()
                .expect("Could not find home directory")
                .join(".claude")
        });

        Self { claude_path }
    }

    /// Parse all Claude sessions from all projects
    pub fn parse_all_sessions(&self) -> Result<Vec<ClaudeSession>> {
        let projects_dir = self.claude_path.join("projects");

        if !projects_dir.exists() {
            return Ok(vec![]);
        }

        // Get all project directories
        let project_dirs: Vec<_> = fs::read_dir(&projects_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if entry.file_type().ok()?.is_dir() {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .collect();

        // Parse sessions in parallel
        let sessions: Vec<ClaudeSession> = project_dirs
            .par_iter()
            .flat_map(|project_dir| self.parse_project_sessions(project_dir).unwrap_or_default())
            .collect();

        Ok(sessions)
    }

    /// Parse sessions from a specific project
    pub fn parse_project_sessions(&self, project_dir: &Path) -> Result<Vec<ClaudeSession>> {
        let session_files: Vec<_> = fs::read_dir(project_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "jsonl" {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        let project_name = project_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let sessions: Vec<ClaudeSession> = session_files
            .into_par_iter()
            .filter_map(|session_file| self.parse_session_file(&session_file, &project_name).ok())
            .collect();

        Ok(sessions)
    }

    /// Parse a single session file
    pub fn parse_session_file(
        &self,
        file_path: &Path,
        project_name: &str,
    ) -> Result<ClaudeSession> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open session file: {:?}", file_path))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line contains the session summary
        let first_line = lines
            .next()
            .context("Session file is empty")?
            .context("Failed to read first line")?;

        let summary: ClaudeSessionSummary =
            serde_json::from_str(&first_line).context("Failed to parse session summary")?;

        // Parse metadata
        let metadata = fs::metadata(file_path)?;
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, 0))
            .unwrap_or_else(Utc::now);

        let session_id = file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Count messages and calculate usage
        let mut message_count = 0;
        let mut total_usage = TokenUsage::default();
        let mut first_timestamp: Option<DateTime<Utc>> = None;

        for line in lines.map_while(Result::ok) {
            if let Ok(message) = serde_json::from_str::<ClaudeMessage>(&line) {
                message_count += 1;

                if first_timestamp.is_none() {
                    first_timestamp = Some(message.timestamp);
                }

                if let Some(usage) = message.message.usage {
                    total_usage.input_tokens += usage.input_tokens;
                    total_usage.output_tokens += usage.output_tokens;
                    total_usage.cache_creation_tokens += usage.cache_creation_input_tokens;
                    total_usage.cache_read_tokens += usage.cache_read_input_tokens;
                }
            }
        }

        let created_at = first_timestamp.unwrap_or(modified_at);

        Ok(ClaudeSession {
            file_path: file_path.to_path_buf(),
            project_path: project_name.to_string(),
            session_id,
            summary: summary.summary,
            created_at,
            modified_at,
            message_count,
            usage: total_usage,
        })
    }

    /// Get recent sessions sorted by modification time
    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<ClaudeSession>> {
        let mut sessions = self.parse_all_sessions()?;
        sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        sessions.truncate(limit);
        Ok(sessions)
    }

    /// Open a Claude session in the browser
    pub fn open_session(&self, session: &ClaudeSession) -> Result<()> {
        // Decode the project path to get the original directory
        let decoded_project = session
            .project_path
            .trim_start_matches('-')
            .replace('-', "/");

        // Construct the Claude URL
        let claude_url = format!("https://claude.ai/code/{}", decoded_project);

        // Open the URL in the default browser
        #[cfg(target_os = "macos")]
        std::process::Command::new("open")
            .arg(&claude_url)
            .spawn()
            .context("Failed to open browser")?;

        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open")
            .arg(&claude_url)
            .spawn()
            .context("Failed to open browser")?;

        #[cfg(target_os = "windows")]
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &claude_url])
            .spawn()
            .context("Failed to open browser")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_parser_creation() {
        let parser = ClaudeSessionParser::new(None);
        assert!(parser.claude_path.ends_with(".claude"));
    }
}
