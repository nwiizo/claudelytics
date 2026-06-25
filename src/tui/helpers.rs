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
        if max_length == 0 {
            return String::new();
        }
        if max_length <= 3 {
            return text.chars().take(max_length).collect();
        }
        if text.chars().count() <= max_length {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(max_length.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    }

    pub(crate) fn compact_date(date: &str) -> String {
        if date.len() == 10 && date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-' {
            format!("{}\n{}", &date[..4], &date[5..])
        } else {
            date.to_string()
        }
    }

    pub(crate) fn format_number_adaptive(num: u64, compact: bool) -> String {
        if compact {
            Self::format_tokens_static(num)
        } else {
            Self::format_number(num)
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

    /// Cache hit rate for cacheable input: reads / (reads + writes).
    pub(crate) fn cache_hit_rate_pct(cache_read_tokens: u64, cache_creation_tokens: u64) -> f64 {
        let total_cache_io = cache_read_tokens.saturating_add(cache_creation_tokens);
        if total_cache_io == 0 {
            0.0
        } else {
            cache_read_tokens as f64 / total_cache_io as f64 * 100.0
        }
    }

    /// Agent-aware cache hit rate.
    /// Claude exposes cache writes and reads. Newer OpenAI records may expose
    /// cache writes, while older Codex logs commonly provide reads only.
    pub(crate) fn model_cache_hit_rate_pct(
        agent: &str,
        input_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
    ) -> f64 {
        if Self::is_openai_agent(agent) {
            let prompt_side_tokens = input_tokens
                .saturating_add(cache_creation_tokens)
                .saturating_add(cache_read_tokens);
            if prompt_side_tokens == 0 {
                0.0
            } else {
                cache_read_tokens as f64 / prompt_side_tokens as f64 * 100.0
            }
        } else {
            Self::cache_hit_rate_pct(cache_read_tokens, cache_creation_tokens)
        }
    }

    pub(crate) fn is_openai_agent(agent: &str) -> bool {
        agent.eq_ignore_ascii_case("codex") || agent.eq_ignore_ascii_case("openai")
    }

    /// Share of all billed/processed tokens that are cache reads or writes.
    pub(crate) fn cache_share_pct(
        total_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
    ) -> f64 {
        if total_tokens == 0 {
            0.0
        } else {
            cache_read_tokens.saturating_add(cache_creation_tokens) as f64 / total_tokens as f64
                * 100.0
        }
    }

    pub(crate) fn cache_reuse_ratio(cache_read_tokens: u64, cache_creation_tokens: u64) -> f64 {
        if cache_creation_tokens == 0 {
            if cache_read_tokens > 0 {
                f64::INFINITY
            } else {
                0.0
            }
        } else {
            cache_read_tokens as f64 / cache_creation_tokens as f64
        }
    }

    pub(crate) fn tokens_per_dollar(total_tokens: u64, total_cost: f64) -> f64 {
        if total_cost <= 0.0 {
            0.0
        } else {
            total_tokens as f64 / total_cost
        }
    }

    pub(crate) fn cost_per_million_tokens(total_cost: f64, total_tokens: u64) -> f64 {
        if total_tokens == 0 {
            0.0
        } else {
            total_cost / total_tokens as f64 * 1_000_000.0
        }
    }

    pub(crate) fn format_ratio(value: f64) -> String {
        if value.is_infinite() {
            "inf".to_string()
        } else {
            format!("{value:.1}x")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TuiApp;

    #[test]
    fn test_metric_helpers() {
        assert_eq!(TuiApp::cache_hit_rate_pct(80, 20), 80.0);
        assert_eq!(TuiApp::cache_hit_rate_pct(0, 0), 0.0);
        assert_eq!(TuiApp::model_cache_hit_rate_pct("Codex", 75, 25, 0), 25.0);
        assert_eq!(TuiApp::model_cache_hit_rate_pct("Codex", 25, 50, 25), 50.0);
        assert_eq!(
            TuiApp::model_cache_hit_rate_pct("Claude Code", 75, 25, 75),
            25.0
        );
        assert_eq!(TuiApp::cache_share_pct(200, 50, 50), 50.0);
        assert_eq!(TuiApp::cache_reuse_ratio(200, 100), 2.0);
        assert_eq!(TuiApp::cost_per_million_tokens(2.0, 1_000_000), 2.0);
        assert_eq!(TuiApp::tokens_per_dollar(1_000, 2.0), 500.0);
    }
}
