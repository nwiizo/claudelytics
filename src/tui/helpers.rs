use ratatui::style::Color;

use super::TuiApp;

impl TuiApp {
    pub(crate) fn format_number(num: u64) -> String {
        if num == 0 {
            "0".to_string()
        } else {
            let num_str = num.to_string();
            let chars: Vec<char> = num_str.chars().collect();
            let mut result = String::new();

            for (i, c) in chars.iter().enumerate() {
                if i > 0 && (chars.len() - i).is_multiple_of(3) {
                    result.push(',');
                }
                result.push(*c);
            }

            result
        }
    }

    pub(crate) fn truncate_text(text: &str, max_length: usize) -> String {
        if text.chars().count() <= max_length {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(max_length.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    }

    pub(crate) fn format_tokens_static(n: u64) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.0}K", n as f64 / 1_000.0)
        } else {
            format!("{}", n)
        }
    }

    pub(crate) fn cost_color(cost: f64) -> Color {
        if cost > 1.0 {
            Color::Red
        } else if cost > 0.5 {
            Color::Yellow
        } else {
            Color::Green
        }
    }

    pub(crate) fn cache_hit_color(pct: f64) -> Color {
        if pct > 95.0 {
            Color::Green
        } else if pct > 85.0 {
            Color::Yellow
        } else {
            Color::Red
        }
    }
}
