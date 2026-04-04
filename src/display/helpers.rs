use colored::*;

pub(crate) fn format_number(num: u64) -> String {
    if num == 0 {
        "0".to_string()
    } else {
        // Manual comma formatting since Rust doesn't support {:,} format
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

pub(crate) fn format_currency(amount: f64) -> String {
    if amount >= 100.0 {
        format!("${:.2}", amount)
    } else {
        format!("${:.4}", amount)
    }
}

pub(crate) fn truncate_path(path: &str, max_length: usize) -> String {
    if path.len() <= max_length {
        path.to_string()
    } else {
        format!("...{}", &path[path.len().saturating_sub(max_length - 3)..])
    }
}

pub(crate) fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

pub fn print_warning(message: &str) {
    eprintln!("{} {}", "Warning:".yellow(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "Error:".red(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "Info:".blue(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency(0.0), "$0.0000");
        assert_eq!(format_currency(99.9999), "$99.9999");
        assert_eq!(format_currency(100.0), "$100.00");
        assert_eq!(format_currency(123.45), "$123.45");
        assert_eq!(format_currency(0.1), "$0.1000");
        assert_eq!(format_currency(999.99), "$999.99");
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short", 10), "short");
        assert_eq!(
            truncate_path("this/is/a/very/long/path", 15),
            "...ry/long/path"
        );
        assert_eq!(truncate_path("", 10), "");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 10), "short");
        assert_eq!(truncate_text("very-long-session-id", 10), "very-lo...");
    }
}
