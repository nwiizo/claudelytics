use std::fmt;
use std::io;

/// アプリケーション全体で使用するエラー型
#[derive(Debug)]
pub enum ClaudelyticsError {
    /// ファイルI/Oエラー
    Io(io::Error),
    /// JSONパースエラー
    JsonParse {
        file_path: String,
        line_number: usize,
        source: serde_json::Error,
    },
    /// 日付パースエラー
    DateParse {
        input: String,
        expected_format: String,
    },
    /// 設定エラー
    Config { message: String },
    /// データ検証エラー
    Validation { field: String, message: String },
    /// ディレクトリが見つからない
    DirectoryNotFound { path: String },
    /// 使用データが見つからない
    NoUsageData { criteria: String },
    /// 価格情報が見つからない
    PricingNotFound { model_name: String },
    /// その他のエラー
    Other { message: String },
}

impl fmt::Display for ClaudelyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClaudelyticsError::Io(err) => write!(f, "I/O error: {}", err),
            ClaudelyticsError::JsonParse {
                file_path,
                line_number,
                source,
            } => write!(
                f,
                "JSON parse error in {} at line {}: {}",
                file_path, line_number, source
            ),
            ClaudelyticsError::DateParse {
                input,
                expected_format,
            } => write!(
                f,
                "Date parse error: '{}' (expected format: {})",
                input, expected_format
            ),
            ClaudelyticsError::Config { message } => write!(f, "Configuration error: {}", message),
            ClaudelyticsError::Validation { field, message } => {
                write!(f, "Validation error in field '{}': {}", field, message)
            }
            ClaudelyticsError::DirectoryNotFound { path } => {
                write!(f, "Directory not found: {}", path)
            }
            ClaudelyticsError::NoUsageData { criteria } => {
                write!(f, "No usage data found for criteria: {}", criteria)
            }
            ClaudelyticsError::PricingNotFound { model_name } => {
                write!(f, "Pricing information not found for model: {}", model_name)
            }
            ClaudelyticsError::Other { message } => write!(f, "Error: {}", message),
        }
    }
}

impl std::error::Error for ClaudelyticsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClaudelyticsError::Io(err) => Some(err),
            ClaudelyticsError::JsonParse { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for ClaudelyticsError {
    fn from(err: io::Error) -> Self {
        ClaudelyticsError::Io(err)
    }
}

impl From<serde_json::Error> for ClaudelyticsError {
    fn from(err: serde_json::Error) -> Self {
        ClaudelyticsError::JsonParse {
            file_path: "unknown".to_string(),
            line_number: 0,
            source: err,
        }
    }
}

impl From<serde_yaml::Error> for ClaudelyticsError {
    fn from(err: serde_yaml::Error) -> Self {
        ClaudelyticsError::Config {
            message: err.to_string(),
        }
    }
}

impl From<csv::Error> for ClaudelyticsError {
    fn from(err: csv::Error) -> Self {
        ClaudelyticsError::Other {
            message: format!("CSV error: {}", err),
        }
    }
}

// crossterm::ErrorKindは新しいバージョンでは削除されているため、コメントアウト
// impl From<crossterm::ErrorKind> for ClaudelyticsError {
//     fn from(err: crossterm::ErrorKind) -> Self {
//         ClaudelyticsError::Other {
//             message: format!("Terminal error: {:?}", err),
//         }
//     }
// }

/// アプリケーション全体で使用するResult型
pub type Result<T> = std::result::Result<T, ClaudelyticsError>;

/// エラーを作成するためのヘルパー関数群
impl ClaudelyticsError {
    pub fn json_parse_error(
        file_path: &str,
        line_number: usize,
        source: serde_json::Error,
    ) -> Self {
        Self::JsonParse {
            file_path: file_path.to_string(),
            line_number,
            source,
        }
    }

    pub fn date_parse_error(input: &str, expected_format: &str) -> Self {
        Self::DateParse {
            input: input.to_string(),
            expected_format: expected_format.to_string(),
        }
    }

    pub fn config_error(message: &str) -> Self {
        Self::Config {
            message: message.to_string(),
        }
    }

    pub fn validation_error(field: &str, message: &str) -> Self {
        Self::Validation {
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    pub fn directory_not_found(path: &str) -> Self {
        Self::DirectoryNotFound {
            path: path.to_string(),
        }
    }

    pub fn no_usage_data(criteria: &str) -> Self {
        Self::NoUsageData {
            criteria: criteria.to_string(),
        }
    }

    pub fn pricing_not_found(model_name: &str) -> Self {
        Self::PricingNotFound {
            model_name: model_name.to_string(),
        }
    }

    pub fn other(message: &str) -> Self {
        Self::Other {
            message: message.to_string(),
        }
    }
}

/// デバッグ用のエラーレポート
impl ClaudelyticsError {
    pub fn detailed_message(&self) -> String {
        match self {
            ClaudelyticsError::JsonParse {
                file_path,
                line_number,
                source,
            } => format!(
                "Failed to parse JSON in file '{}' at line {}\nError: {}\nThis usually indicates corrupted or invalid JSON data.",
                file_path, line_number, source
            ),
            ClaudelyticsError::DirectoryNotFound { path } => format!(
                "Claude directory not found at '{}'\nPlease ensure Claude Code is installed and has been used at least once.",
                path
            ),
            ClaudelyticsError::NoUsageData { criteria } => format!(
                "No usage data found matching the criteria: {}\nThis might mean:\n- No Claude Code usage in the specified time period\n- JSONL files are missing or empty\n- Date range is outside available data",
                criteria
            ),
            _ => self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ClaudelyticsError::date_parse_error("invalid", "YYYYMMDD");
        assert!(err.to_string().contains("Date parse error"));
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("YYYYMMDD"));
    }

    #[test]
    fn test_detailed_message() {
        let err = ClaudelyticsError::no_usage_data("--today");
        let detailed = err.detailed_message();
        assert!(detailed.contains("No usage data found"));
        assert!(detailed.contains("--today"));
        assert!(detailed.contains("might mean"));
    }
}
