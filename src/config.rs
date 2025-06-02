use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration settings for Claudelytics
///
/// Stores user preferences that persist between runs, including:
/// - Claude directory location
/// - Default output format (enhanced/classic/json)
/// - Default command to run
/// - Watch interval for real-time monitoring
/// - Export directory for CSV files
/// - Date format preferences
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Custom path to Claude directory (default: ~/.claude)
    pub claude_path: Option<PathBuf>,
    /// Default output format for reports
    pub default_output_format: OutputFormat,
    /// Default command to execute when none specified
    pub default_command: DefaultCommand,
    /// Interval in seconds for watch mode file monitoring
    pub watch_interval_seconds: u64,
    /// Directory for CSV exports (default: current directory)
    pub export_directory: Option<PathBuf>,
    /// Date format string for display (strftime format)
    pub date_format: String,
}

/// Output format options for reports
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OutputFormat {
    /// Enhanced format with visual cards and summaries (default)
    Enhanced,
    /// Classic ASCII table format
    Table,
    /// JSON format for scripting and automation
    Json,
}

/// Default command to execute when none specified
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DefaultCommand {
    /// Show daily usage report (default)
    Daily,
    /// Show session-based usage report
    Session,
    /// Launch interactive session selector
    Interactive,
    /// Launch terminal user interface
    Tui,
    /// Launch advanced analytics TUI
    AdvancedTui,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude_path: None,
            default_output_format: OutputFormat::Enhanced,
            default_command: DefaultCommand::Daily,
            watch_interval_seconds: 5,
            export_directory: None,
            date_format: "%Y-%m-%d".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("claudelytics")
            .join("config.yaml"))
    }

    pub fn get_claude_path(&self) -> Result<PathBuf> {
        if let Some(path) = &self.claude_path {
            Ok(path.clone())
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
            Ok(PathBuf::from(home).join(".claude"))
        }
    }

    pub fn set_claude_path(&mut self, path: PathBuf) {
        self.claude_path = Some(path);
    }

    pub fn get_export_directory(&self) -> PathBuf {
        self.export_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}
