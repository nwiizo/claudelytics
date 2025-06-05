use crate::error::{ClaudelyticsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

/// 完全な設定構造体 - アプリケーション全体の設定を管理
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// 基本設定
    pub core: CoreConfig,
    /// 表示設定
    pub display: DisplayConfig,
    /// パフォーマンス設定
    pub performance: PerformanceConfig,
    /// 価格設定
    pub pricing: PricingConfig,
    /// エクスポート設定
    pub export: ExportConfig,
    /// プロファイル設定
    pub profiles: HashMap<String, ProfileConfig>,
}

/// 基本設定
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoreConfig {
    /// Claude ディレクトリのパス
    pub claude_path: Option<PathBuf>,
    /// デフォルトのコマンド
    pub default_command: DefaultCommand,
    /// 設定ファイルのバージョン
    pub config_version: String,
}

/// 表示設定
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DisplayConfig {
    /// 出力形式
    pub output_format: OutputFormat,
    /// 日付形式
    pub date_format: String,
    /// カラー出力の有効/無効
    pub color_enabled: bool,
    /// テーブルのスタイル
    pub table_style: TableStyle,
    /// 通貨記号
    pub currency_symbol: String,
    /// 数値形式の設定
    pub number_format: NumberFormat,
}

/// パフォーマンス設定
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerformanceConfig {
    /// 並列処理のワーカー数（0 = CPU数と同じ）
    pub parallel_workers: usize,
    /// ファイル監視の間隔（秒）
    pub watch_interval_seconds: u64,
    /// メモリ使用量の制限（MB）
    pub memory_limit_mb: Option<usize>,
    /// キャッシュの有効期間（秒）
    pub cache_duration_seconds: u64,
}

/// 価格設定
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PricingConfig {
    /// カスタム価格ファイルのパス
    pub custom_pricing_file: Option<PathBuf>,
    /// 価格計算戦略
    pub calculation_strategy: PricingStrategy,
    /// オンライン価格取得の有効/無効
    pub online_pricing_enabled: bool,
    /// 価格の更新間隔（秒）
    pub price_update_interval_seconds: u64,
}

/// エクスポート設定
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportConfig {
    /// エクスポートディレクトリ
    pub directory: Option<PathBuf>,
    /// CSV の区切り文字
    pub csv_delimiter: char,
    /// ファイル名のテンプレート
    pub filename_template: String,
    /// 圧縮の有効/無効
    pub compression_enabled: bool,
}

/// プロファイル設定（異なる環境や用途に応じた設定セット）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileConfig {
    /// プロファイル名
    pub name: String,
    /// 説明
    pub description: String,
    /// このプロファイル固有の設定オーバーライド
    pub overrides: ConfigOverrides,
}

/// 設定のオーバーライド
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub claude_path: Option<PathBuf>,
    pub output_format: Option<OutputFormat>,
    pub parallel_workers: Option<usize>,
    pub pricing_strategy: Option<PricingStrategy>,
}

// 列挙型の定義

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OutputFormat {
    Enhanced,
    Table,
    Json,
    Csv,
    Minimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DefaultCommand {
    Daily,
    Session,
    Interactive,
    Tui,
    AdvancedTui,
    Cost,
    Watch,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TableStyle {
    Ascii,
    Unicode,
    Rounded,
    Minimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NumberFormat {
    pub thousands_separator: String,
    pub decimal_separator: String,
    pub decimal_places: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PricingStrategy {
    Fallback,
    Configurable,
    Online,
    Composite,
}

// Default implementations

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            display: DisplayConfig::default(),
            performance: PerformanceConfig::default(),
            pricing: PricingConfig::default(),
            export: ExportConfig::default(),
            profiles: HashMap::new(),
        }
    }
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            claude_path: None,
            default_command: DefaultCommand::Daily,
            config_version: "0.3.0".to_string(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Enhanced,
            date_format: "%Y-%m-%d".to_string(),
            color_enabled: true,
            table_style: TableStyle::Unicode,
            currency_symbol: "$".to_string(),
            number_format: NumberFormat::default(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            parallel_workers: 0, // 0 means use number of CPUs
            watch_interval_seconds: 5,
            memory_limit_mb: None,
            cache_duration_seconds: 300, // 5 minutes
        }
    }
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            custom_pricing_file: None,
            calculation_strategy: PricingStrategy::Fallback,
            online_pricing_enabled: false,
            price_update_interval_seconds: 3600, // 1 hour
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            directory: None,
            csv_delimiter: ',',
            filename_template: "claudelytics_{timestamp}_{type}".to_string(),
            compression_enabled: false,
        }
    }
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self {
            thousands_separator: ",".to_string(),
            decimal_separator: ".".to_string(),
            decimal_places: 4,
        }
    }
}

// Display implementations

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Enhanced => write!(f, "enhanced"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Minimal => write!(f, "minimal"),
        }
    }
}

impl fmt::Display for DefaultCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefaultCommand::Daily => write!(f, "daily"),
            DefaultCommand::Session => write!(f, "session"),
            DefaultCommand::Interactive => write!(f, "interactive"),
            DefaultCommand::Tui => write!(f, "tui"),
            DefaultCommand::AdvancedTui => write!(f, "advanced-tui"),
            DefaultCommand::Cost => write!(f, "cost"),
            DefaultCommand::Watch => write!(f, "watch"),
        }
    }
}

// Configuration management

impl AppConfig {
    /// 設定をロード（環境変数、設定ファイル、デフォルト値の順で優先）
    pub fn load() -> Result<Self> {
        let mut config = Self::load_from_file().unwrap_or_default();
        config.apply_environment_variables()?;
        config.validate()?;
        Ok(config)
    }

    /// プロファイル指定でロード
    pub fn load_with_profile(profile_name: &str) -> Result<Self> {
        let mut config = Self::load()?;
        config.apply_profile(profile_name)?;
        Ok(config)
    }

    /// ファイルから設定をロード
    fn load_from_file() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(|e| {
                ClaudelyticsError::config_error(&format!("Failed to read config file: {}", e))
            })?;

            let config: AppConfig = serde_yaml::from_str(&content).map_err(|e| {
                ClaudelyticsError::config_error(&format!("Failed to parse config file: {}", e))
            })?;

            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// 環境変数を適用
    fn apply_environment_variables(&mut self) -> Result<()> {
        if let Ok(path) = env::var("CLAUDELYTICS_CLAUDE_PATH") {
            self.core.claude_path = Some(PathBuf::from(path));
        }

        if let Ok(format) = env::var("CLAUDELYTICS_OUTPUT_FORMAT") {
            self.display.output_format = match format.to_lowercase().as_str() {
                "enhanced" => OutputFormat::Enhanced,
                "table" => OutputFormat::Table,
                "json" => OutputFormat::Json,
                "csv" => OutputFormat::Csv,
                "minimal" => OutputFormat::Minimal,
                _ => {
                    return Err(ClaudelyticsError::config_error(&format!(
                        "Invalid output format: {}",
                        format
                    )));
                }
            };
        }

        if let Ok(workers) = env::var("CLAUDELYTICS_PARALLEL_WORKERS") {
            self.performance.parallel_workers = workers
                .parse()
                .map_err(|_| ClaudelyticsError::config_error("Invalid parallel workers count"))?;
        }

        if let Ok(color) = env::var("CLAUDELYTICS_COLOR") {
            self.display.color_enabled = color.to_lowercase() == "true" || color == "1";
        }

        Ok(())
    }

    /// プロファイルを適用
    fn apply_profile(&mut self, profile_name: &str) -> Result<()> {
        if let Some(profile) = self.profiles.get(profile_name) {
            let overrides = &profile.overrides;

            if let Some(claude_path) = &overrides.claude_path {
                self.core.claude_path = Some(claude_path.clone());
            }

            if let Some(output_format) = &overrides.output_format {
                self.display.output_format = output_format.clone();
            }

            if let Some(parallel_workers) = overrides.parallel_workers {
                self.performance.parallel_workers = parallel_workers;
            }

            if let Some(pricing_strategy) = &overrides.pricing_strategy {
                self.pricing.calculation_strategy = pricing_strategy.clone();
            }
        } else {
            return Err(ClaudelyticsError::config_error(&format!(
                "Profile '{}' not found",
                profile_name
            )));
        }

        Ok(())
    }

    /// 設定を検証
    fn validate(&self) -> Result<()> {
        if self.performance.parallel_workers > 1000 {
            return Err(ClaudelyticsError::validation_error(
                "parallel_workers",
                "Too many parallel workers (max 1000)",
            ));
        }

        if self.performance.watch_interval_seconds == 0 {
            return Err(ClaudelyticsError::validation_error(
                "watch_interval_seconds",
                "Watch interval must be greater than 0",
            ));
        }

        if let Some(memory_limit) = self.performance.memory_limit_mb {
            if memory_limit < 100 {
                return Err(ClaudelyticsError::validation_error(
                    "memory_limit_mb",
                    "Memory limit must be at least 100MB",
                ));
            }
        }

        Ok(())
    }

    /// 設定を保存
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ClaudelyticsError::config_error(&format!(
                    "Failed to create config directory: {}",
                    e
                ))
            })?;
        }

        let content = serde_yaml::to_string(self).map_err(|e| {
            ClaudelyticsError::config_error(&format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&config_path, content).map_err(|e| {
            ClaudelyticsError::config_error(&format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// 設定ファイルのパスを取得
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
            PathBuf::from(config_home)
        } else if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            return Err(ClaudelyticsError::config_error(
                "Neither XDG_CONFIG_HOME nor HOME environment variable is set",
            ));
        };

        Ok(config_dir.join("claudelytics").join("config.yaml"))
    }

    /// Claude ディレクトリのパスを取得
    pub fn get_claude_path(&self) -> Result<PathBuf> {
        if let Some(path) = &self.core.claude_path {
            Ok(path.clone())
        } else if let Ok(claude_home) = env::var("CLAUDE_HOME") {
            Ok(PathBuf::from(claude_home))
        } else if let Ok(home) = env::var("HOME") {
            Ok(PathBuf::from(home).join(".claude"))
        } else {
            Err(ClaudelyticsError::config_error(
                "Cannot determine Claude directory path",
            ))
        }
    }

    /// エクスポートディレクトリのパスを取得
    pub fn get_export_directory(&self) -> PathBuf {
        self.export
            .directory
            .clone()
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// プロファイルを追加
    pub fn add_profile(&mut self, profile: ProfileConfig) {
        self.profiles.insert(profile.name.clone(), profile);
    }

    /// プロファイルを削除
    pub fn remove_profile(&mut self, name: &str) -> Option<ProfileConfig> {
        self.profiles.remove(name)
    }

    /// 利用可能なプロファイル一覧を取得
    pub fn list_profiles(&self) -> Vec<&String> {
        self.profiles.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.display.output_format.to_string(), "enhanced");
        assert_eq!(config.core.default_command.to_string(), "daily");
        assert_eq!(config.performance.parallel_workers, 0);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        config.performance.watch_interval_seconds = 0;

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_management() {
        let mut config = AppConfig::default();

        let profile = ProfileConfig {
            name: "test".to_string(),
            description: "Test profile".to_string(),
            overrides: ConfigOverrides {
                output_format: Some(OutputFormat::Json),
                ..Default::default()
            },
        };

        config.add_profile(profile);
        assert_eq!(config.list_profiles().len(), 1);

        let result = config.apply_profile("test");
        assert!(result.is_ok());

        matches!(config.display.output_format, OutputFormat::Json);
    }
}
