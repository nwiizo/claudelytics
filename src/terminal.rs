use std::io::{self, Write};

/// Terminal utilities for responsive display
pub struct Terminal;

impl Terminal {
    /// Get terminal width, with fallback to 80 columns
    pub fn width() -> u16 {
        terminal_size::terminal_size()
            .map(|(width, _)| width.0)
            .unwrap_or(80)
    }

    /// Get terminal height, with fallback to 24 rows
    pub fn height() -> u16 {
        terminal_size::terminal_size()
            .map(|(_, height)| height.0)
            .unwrap_or(24)
    }

    /// Check if terminal is narrow (less than 100 columns)
    pub fn is_narrow() -> bool {
        Self::width() < 100
    }

    /// Check if terminal is wide (more than 120 columns)
    pub fn is_wide() -> bool {
        Self::width() > 120
    }

    /// Create a separator line that fits the terminal width
    pub fn separator(char: char) -> String {
        char.to_string().repeat(Self::width() as usize)
    }

    /// Create a centered text line
    pub fn center_text(text: &str) -> String {
        let width = Self::width() as usize;
        let text_len = strip_ansi_codes(text).len();
        if text_len >= width {
            text.to_string()
        } else {
            let padding = (width - text_len) / 2;
            format!("{}{}", " ".repeat(padding), text)
        }
    }

    /// Truncate text to fit terminal width with ellipsis
    pub fn truncate_to_width(text: &str, max_width: Option<usize>) -> String {
        let width = max_width.unwrap_or(Self::width() as usize);
        let text_len = strip_ansi_codes(text).len();

        if text_len <= width {
            text.to_string()
        } else {
            let truncated = &text[..width.saturating_sub(3)];
            format!("{}...", truncated)
        }
    }

    /// Format table columns to fit terminal width
    pub fn format_columns(columns: &[(&str, usize)]) -> Vec<usize> {
        let width = Self::width() as usize;
        let num_columns = columns.len();
        let separators = num_columns.saturating_sub(1) * 3; // " | " between columns
        let available_width = width.saturating_sub(separators);

        let total_requested: usize = columns.iter().map(|(_, w)| *w).sum();

        if total_requested <= available_width {
            columns.iter().map(|(_, w)| *w).collect()
        } else {
            // Scale down proportionally
            columns
                .iter()
                .map(|(_, w)| (*w * available_width / total_requested).max(5))
                .collect()
        }
    }

    /// Clear current line
    pub fn clear_line() -> io::Result<()> {
        print!("\r{}\r", " ".repeat(Self::width() as usize));
        io::stdout().flush()
    }

    /// Print progress bar
    pub fn progress_bar(current: usize, total: usize, width: Option<usize>) -> String {
        let bar_width = width.unwrap_or((Self::width() as usize).saturating_sub(20));
        let progress = if total > 0 {
            current as f64 / total as f64
        } else {
            0.0
        };
        let filled = (progress * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);

        format!(
            "[{}{}] {}/{}",
            "█".repeat(filled),
            "░".repeat(empty),
            current,
            total
        )
    }
}

/// Strip ANSI color codes from text for accurate length calculation
fn strip_ansi_codes(text: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(text, "").to_string()
}

/// Display mode based on terminal width
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Compact, // < 100 columns
    Normal,  // 100-120 columns
    Wide,    // > 120 columns
}

impl DisplayMode {
    pub fn detect() -> Self {
        let width = Terminal::width();
        if width < 100 {
            DisplayMode::Compact
        } else if width > 120 {
            DisplayMode::Wide
        } else {
            DisplayMode::Normal
        }
    }

    pub fn should_show_cache_columns(&self) -> bool {
        matches!(self, DisplayMode::Normal | DisplayMode::Wide)
    }

    pub fn should_show_efficiency(&self) -> bool {
        matches!(self, DisplayMode::Wide)
    }

    pub fn date_format(&self) -> &'static str {
        match self {
            DisplayMode::Compact => "%m-%d",
            _ => "%Y-%m-%d",
        }
    }

    pub fn number_format(&self) -> NumberFormat {
        match self {
            DisplayMode::Compact => NumberFormat::Short,
            _ => NumberFormat::Full,
        }
    }
}

/// Number formatting options
#[derive(Debug, Clone, Copy)]
pub enum NumberFormat {
    Full,  // 1,234,567
    Short, // 1.2M
}

impl NumberFormat {
    pub fn format(&self, num: u64) -> String {
        match self {
            NumberFormat::Full => format_number_full(num),
            NumberFormat::Short => format_number_short(num),
        }
    }
}

fn format_number_full(num: u64) -> String {
    let num_str = num.to_string();
    let chars: Vec<char> = num_str.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

fn format_number_short(num: u64) -> String {
    if num >= 1_000_000_000 {
        format!("{:.1}B", num as f64 / 1_000_000_000.0)
    } else if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number_short() {
        assert_eq!(format_number_short(999), "999");
        assert_eq!(format_number_short(1_000), "1.0K");
        assert_eq!(format_number_short(1_500), "1.5K");
        assert_eq!(format_number_short(1_000_000), "1.0M");
        assert_eq!(format_number_short(1_500_000), "1.5M");
        assert_eq!(format_number_short(1_000_000_000), "1.0B");
    }

    #[test]
    fn test_strip_ansi_codes() {
        let colored = "\x1b[31mRed Text\x1b[0m";
        assert_eq!(strip_ansi_codes(colored), "Red Text");
    }
}
