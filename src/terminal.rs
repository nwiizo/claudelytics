/// Terminal utilities for responsive display
pub struct Terminal;

impl Terminal {
    /// Get terminal width, with fallback to 80 columns
    pub fn width() -> u16 {
        terminal_size::terminal_size()
            .map(|(width, _)| width.0)
            .unwrap_or(80)
    }

    /// Create a separator line that fits the terminal width
    pub fn separator(char: char) -> String {
        char.to_string().repeat(Self::width() as usize)
    }
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

    pub fn should_show_efficiency(&self) -> bool {
        matches!(self, DisplayMode::Wide)
    }
}
