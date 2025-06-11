use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiSessionState {
    pub mode: TuiMode,
    pub last_tab: Option<usize>,
    pub last_session_path: Option<String>,
    pub last_search_query: Option<String>,
    pub bookmarked_sessions: Vec<String>,
    pub comparison_sessions: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TuiMode {
    Basic,
    Advanced,
    Analytics, // For future use
}

impl Default for TuiSessionState {
    fn default() -> Self {
        Self {
            mode: TuiMode::Basic,
            last_tab: None,
            last_session_path: None,
            last_search_query: None,
            bookmarked_sessions: Vec::new(),
            comparison_sessions: Vec::new(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl TuiSessionState {
    pub fn save(&self) -> Result<()> {
        let state_path = Self::get_state_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(state_path, json)?;
        Ok(())
    }

    pub fn load() -> Result<Self> {
        let state_path = Self::get_state_path()?;

        if !state_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(state_path)?;
        let state: TuiSessionState = serde_json::from_str(&contents)?;
        Ok(state)
    }

    #[allow(dead_code)]
    pub fn clear() -> Result<()> {
        let state_path = Self::get_state_path()?;
        if state_path.exists() {
            fs::remove_file(state_path)?;
        }
        Ok(())
    }

    fn get_state_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let state_dir = PathBuf::from(home).join(".claude").join("claudelytics");
        Ok(state_dir.join("tui_session.json"))
    }

    pub fn should_resume(&self) -> bool {
        // Resume if the last session was within the last 24 hours
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(self.timestamp);
        duration.num_hours() < 24
    }

    pub fn update_timestamp(&mut self) {
        self.timestamp = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = TuiSessionState::default();
        assert!(matches!(state.mode, TuiMode::Basic));
        assert!(state.last_tab.is_none());
        assert!(state.bookmarked_sessions.is_empty());
    }

    #[test]
    fn test_serialization() {
        let mut state = TuiSessionState {
            mode: TuiMode::Advanced,
            last_tab: Some(2),
            ..Default::default()
        };
        state.bookmarked_sessions.push("test/session".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: TuiSessionState = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized.mode, TuiMode::Advanced));
        assert_eq!(deserialized.last_tab, Some(2));
        assert_eq!(deserialized.bookmarked_sessions.len(), 1);
    }
}
