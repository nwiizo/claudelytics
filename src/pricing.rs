use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;

/// Fast mode pricing multiplier (Claude Code /fast uses 6x pricing)
pub const FAST_MODE_MULTIPLIER: f64 = 6.0;

/// Token threshold for tiered pricing on 1M context models
pub const TIERED_THRESHOLD: u64 = 200_000;

/// OpenAI switches the whole request to long-context rates above this input size.
pub const OPENAI_LONG_CONTEXT_THRESHOLD: u64 = 272_000;
const SONNET_5_INTRO_PRICING_KEY: &str = "__claude-sonnet-5-intro";
const SONNET_5_INTRO_END: NaiveDate =
    NaiveDate::from_ymd_opt(2026, 8, 31).expect("valid Sonnet 5 pricing date");

pub(crate) fn canonical_model_name(model_name: &str) -> Option<&'static str> {
    if model_name == "gpt-5.6" {
        Some("gpt-5.6-sol")
    } else if model_name.contains("fable-5") {
        Some("claude-fable-5")
    } else if model_name.contains("sonnet-5") {
        Some("claude-sonnet-5")
    } else if model_name.contains("sonnet-4-6") || model_name.contains("sonnet-4.6") {
        Some("claude-sonnet-4-6")
    } else if model_name.contains("sonnet-4") {
        Some("claude-sonnet-4-20250514")
    } else if model_name.contains("opus-5") {
        Some("claude-opus-5")
    } else if model_name.contains("opus-4-8") || model_name.contains("opus-4.8") {
        Some("claude-opus-4-8")
    } else if model_name.contains("opus-4-7") || model_name.contains("opus-4.7") {
        Some("claude-opus-4-7")
    } else if model_name.contains("opus-4-6") || model_name.contains("opus-4.6") {
        Some("claude-opus-4-6")
    } else if model_name.contains("opus-4") {
        Some("claude-opus-4-20250514")
    } else if model_name.contains("haiku-4-5") || model_name.contains("haiku-4.5") {
        Some("claude-haiku-4-5-20251001")
    } else {
        None
    }
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct ModelPricing {
    pub input_cost_per_token: Option<f64>,
    pub output_cost_per_token: Option<f64>,
    pub cache_creation_input_token_cost: Option<f64>,
    pub cache_read_input_token_cost: Option<f64>,
    /// Tiered pricing for tokens above 200k (1M context models)
    #[serde(default)]
    pub input_cost_per_token_above_200k: Option<f64>,
    #[serde(default)]
    pub output_cost_per_token_above_200k: Option<f64>,
    #[serde(default)]
    pub cache_creation_cost_above_200k: Option<f64>,
    #[serde(default)]
    pub cache_read_cost_above_200k: Option<f64>,
    /// Provider-specific threshold for whole-request two-stage pricing.
    #[serde(default)]
    pub long_context_threshold: Option<u64>,
}

pub struct PricingFetcher;

impl Default for PricingFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl PricingFetcher {
    pub fn new() -> Self {
        Self
    }

    pub fn get_model_pricing<'a>(
        &self,
        pricing_data: &'a HashMap<String, ModelPricing>,
        model_name: &str,
    ) -> Option<&'a ModelPricing> {
        self.get_model_pricing_at(pricing_data, model_name, Utc::now().date_naive())
    }

    pub fn get_model_pricing_at<'a>(
        &self,
        pricing_data: &'a HashMap<String, ModelPricing>,
        model_name: &str,
        usage_date: NaiveDate,
    ) -> Option<&'a ModelPricing> {
        if canonical_model_name(model_name) == Some("claude-sonnet-5")
            && usage_date <= SONNET_5_INTRO_END
            && let Some(pricing) = pricing_data.get(SONNET_5_INTRO_PRICING_KEY)
        {
            return Some(pricing);
        }

        // Direct match first
        if let Some(pricing) = pricing_data.get(model_name) {
            return Some(pricing);
        }

        // Resolve current provider aliases without allocating temporary strings
        // on every usage record.
        if let Some(pricing) = canonical_model_name(model_name)
            .and_then(|canonical_name| pricing_data.get(canonical_name))
        {
            return Some(pricing);
        }

        // Prefer the most-specific fallback to make overlapping model names
        // deterministic regardless of HashMap iteration order.
        pricing_data
            .iter()
            .filter(|(key, _)| key.contains(model_name) || model_name.contains(key.as_str()))
            .max_by(|(left, _), (right, _)| {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            })
            .map(|(_, pricing)| pricing)
    }

    pub fn calculate_cost(
        &self,
        pricing: &ModelPricing,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
    ) -> f64 {
        let total_input_context = input_tokens
            .saturating_add(cache_read_tokens)
            .saturating_add(cache_creation_tokens);
        if let Some(threshold) = pricing.long_context_threshold {
            return self.calculate_whole_request_cost(
                pricing,
                input_tokens,
                output_tokens,
                cache_creation_tokens,
                cache_read_tokens,
                total_input_context > threshold,
            );
        }

        let use_tiered = total_input_context > TIERED_THRESHOLD
            && pricing.input_cost_per_token_above_200k.is_some();

        if use_tiered {
            self.calculate_tiered_cost(
                pricing,
                input_tokens,
                output_tokens,
                cache_creation_tokens,
                cache_read_tokens,
                total_input_context,
            )
        } else {
            self.calculate_flat_cost(
                pricing,
                input_tokens,
                output_tokens,
                cache_creation_tokens,
                cache_read_tokens,
            )
        }
    }

    fn calculate_whole_request_cost(
        &self,
        pricing: &ModelPricing,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
        use_long_context_rates: bool,
    ) -> f64 {
        let rate = |base: Option<f64>, long: Option<f64>| {
            if use_long_context_rates {
                long.or(base)
            } else {
                base
            }
        };

        input_tokens as f64
            * rate(
                pricing.input_cost_per_token,
                pricing.input_cost_per_token_above_200k,
            )
            .unwrap_or_default()
            + output_tokens as f64
                * rate(
                    pricing.output_cost_per_token,
                    pricing.output_cost_per_token_above_200k,
                )
                .unwrap_or_default()
            + cache_creation_tokens as f64
                * rate(
                    pricing.cache_creation_input_token_cost,
                    pricing.cache_creation_cost_above_200k,
                )
                .unwrap_or_default()
            + cache_read_tokens as f64
                * rate(
                    pricing.cache_read_input_token_cost,
                    pricing.cache_read_cost_above_200k,
                )
                .unwrap_or_default()
    }

    fn calculate_flat_cost(
        &self,
        pricing: &ModelPricing,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
    ) -> f64 {
        let mut total_cost = 0.0;

        if let Some(input_cost) = pricing.input_cost_per_token {
            total_cost += input_tokens as f64 * input_cost;
        }
        if let Some(output_cost) = pricing.output_cost_per_token {
            total_cost += output_tokens as f64 * output_cost;
        }
        if let Some(cache_creation_cost) = pricing.cache_creation_input_token_cost {
            total_cost += cache_creation_tokens as f64 * cache_creation_cost;
        }
        if let Some(cache_read_cost) = pricing.cache_read_input_token_cost {
            total_cost += cache_read_tokens as f64 * cache_read_cost;
        }

        total_cost
    }

    fn calculate_tiered_cost(
        &self,
        pricing: &ModelPricing,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
        total_input_context: u64,
    ) -> f64 {
        // Ratio of tokens below/above the threshold
        let below_ratio = TIERED_THRESHOLD as f64 / total_input_context as f64;
        let above_ratio = 1.0 - below_ratio;

        let mut total_cost = 0.0;

        // Input tokens: split proportionally
        if let (Some(base), Some(tiered)) = (
            pricing.input_cost_per_token,
            pricing.input_cost_per_token_above_200k,
        ) {
            let below = (input_tokens as f64 * below_ratio) * base;
            let above = (input_tokens as f64 * above_ratio) * tiered;
            total_cost += below + above;
        } else if let Some(base) = pricing.input_cost_per_token {
            total_cost += input_tokens as f64 * base;
        }

        // Output tokens: split proportionally like input tokens
        if let (Some(base), Some(tiered)) = (
            pricing.output_cost_per_token,
            pricing.output_cost_per_token_above_200k,
        ) {
            let below = (output_tokens as f64 * below_ratio) * base;
            let above = (output_tokens as f64 * above_ratio) * tiered;
            total_cost += below + above;
        } else if let Some(base) = pricing.output_cost_per_token {
            total_cost += output_tokens as f64 * base;
        }

        // Cache creation tokens
        if let (Some(base), Some(tiered)) = (
            pricing.cache_creation_input_token_cost,
            pricing.cache_creation_cost_above_200k,
        ) {
            let below = (cache_creation_tokens as f64 * below_ratio) * base;
            let above = (cache_creation_tokens as f64 * above_ratio) * tiered;
            total_cost += below + above;
        } else if let Some(base) = pricing.cache_creation_input_token_cost {
            total_cost += cache_creation_tokens as f64 * base;
        }

        // Cache read tokens
        if let (Some(base), Some(tiered)) = (
            pricing.cache_read_input_token_cost,
            pricing.cache_read_cost_above_200k,
        ) {
            let below = (cache_read_tokens as f64 * below_ratio) * base;
            let above = (cache_read_tokens as f64 * above_ratio) * tiered;
            total_cost += below + above;
        } else if let Some(base) = pricing.cache_read_input_token_cost {
            total_cost += cache_read_tokens as f64 * base;
        }

        total_cost
    }
}

// Offline fallback pricing aligned with current provider pricing and ccusage.
pub fn get_fallback_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();
    let claude_pricing = |input_per_million: f64, output_per_million: f64| ModelPricing {
        input_cost_per_token: Some(input_per_million / 1_000_000.0),
        output_cost_per_token: Some(output_per_million / 1_000_000.0),
        cache_creation_input_token_cost: Some(input_per_million * 1.25 / 1_000_000.0),
        cache_read_input_token_cost: Some(input_per_million * 0.1 / 1_000_000.0),
        input_cost_per_token_above_200k: None,
        output_cost_per_token_above_200k: None,
        cache_creation_cost_above_200k: None,
        cache_read_cost_above_200k: None,
        long_context_threshold: None,
    };

    // Claude Sonnet 4 - $3/$15 per MTok, above 200k: $6/$30
    pricing.insert(
        "claude-sonnet-4-20250514".to_string(),
        ModelPricing {
            input_cost_per_token: Some(3.0 / 1_000_000.0),
            output_cost_per_token: Some(15.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(3.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(0.3 / 1_000_000.0),
            input_cost_per_token_above_200k: Some(6.0 / 1_000_000.0),
            output_cost_per_token_above_200k: Some(30.0 / 1_000_000.0),
            cache_creation_cost_above_200k: Some(7.5 / 1_000_000.0),
            cache_read_cost_above_200k: Some(0.6 / 1_000_000.0),
            long_context_threshold: None,
        },
    );

    // Claude Opus 4 - $15/$75 per MTok, above 200k: $30/$150
    pricing.insert(
        "claude-opus-4-20250514".to_string(),
        ModelPricing {
            input_cost_per_token: Some(15.0 / 1_000_000.0),
            output_cost_per_token: Some(75.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(18.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(1.5 / 1_000_000.0),
            input_cost_per_token_above_200k: Some(30.0 / 1_000_000.0),
            output_cost_per_token_above_200k: Some(150.0 / 1_000_000.0),
            cache_creation_cost_above_200k: Some(37.5 / 1_000_000.0),
            cache_read_cost_above_200k: Some(3.0 / 1_000_000.0),
            long_context_threshold: None,
        },
    );

    pricing.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        claude_pricing(3.0, 15.0),
    );
    pricing.insert(
        "claude-3-5-haiku-20241022".to_string(),
        claude_pricing(0.8, 4.0),
    );
    pricing.insert(
        "claude-3-opus-20240229".to_string(),
        claude_pricing(15.0, 75.0),
    );

    // Current Claude generation.
    for (model, input, output) in [
        ("claude-fable-5", 10.0, 50.0),
        ("claude-opus-5", 5.0, 25.0),
        ("claude-opus-4-8", 5.0, 25.0),
        ("claude-opus-4-7", 5.0, 25.0),
        ("claude-opus-4-6", 5.0, 25.0),
        ("claude-sonnet-5", 3.0, 15.0),
        ("claude-sonnet-4-6", 3.0, 15.0),
        ("claude-haiku-4-5-20251001", 1.0, 5.0),
    ] {
        pricing.insert(model.to_string(), claude_pricing(input, output));
    }
    pricing.insert(
        SONNET_5_INTRO_PRICING_KEY.to_string(),
        claude_pricing(2.0, 10.0),
    );

    let openai_pricing = |input_per_million: f64, output_per_million: f64| ModelPricing {
        input_cost_per_token: Some(input_per_million / 1_000_000.0),
        output_cost_per_token: Some(output_per_million / 1_000_000.0),
        // Older model pages do not publish a distinct cache-write price.
        // If a newer wire record reports writes for one of these models,
        // bill them at the ordinary input rate rather than making them free.
        cache_creation_input_token_cost: Some(input_per_million / 1_000_000.0),
        cache_read_input_token_cost: Some(input_per_million * 0.1 / 1_000_000.0),
        input_cost_per_token_above_200k: None,
        output_cost_per_token_above_200k: None,
        cache_creation_cost_above_200k: None,
        cache_read_cost_above_200k: None,
        long_context_threshold: None,
    };

    for (model, input, output) in [
        ("gpt-5-codex", 1.25, 10.0),
        ("gpt-5.1-codex", 1.25, 10.0),
        ("gpt-5.1-codex-mini", 0.25, 2.0),
        ("gpt-5.2-codex", 1.75, 14.0),
        ("gpt-5.3-codex", 1.75, 14.0),
        ("gpt-5", 1.25, 10.0),
    ] {
        pricing.insert(model.to_string(), openai_pricing(input, output));
    }

    // OpenAI bills GPT-5.6 requests above 272K input tokens entirely at the
    // long-context tier rather than applying a marginal breakpoint.
    for (
        model,
        input,
        output,
        cache_create,
        cache_read,
        long_input,
        long_output,
        long_cache_create,
        long_cache_read,
    ) in [
        ("gpt-5.6-sol", 5.0, 30.0, 6.25, 0.5, 10.0, 45.0, 12.5, 1.0),
        (
            "gpt-5.6-terra",
            2.5,
            15.0,
            3.125,
            0.25,
            5.0,
            22.5,
            6.25,
            0.5,
        ),
        ("gpt-5.6-luna", 1.0, 6.0, 1.25, 0.1, 2.0, 9.0, 2.5, 0.2),
    ] {
        pricing.insert(
            model.to_string(),
            ModelPricing {
                input_cost_per_token: Some(input / 1_000_000.0),
                output_cost_per_token: Some(output / 1_000_000.0),
                cache_creation_input_token_cost: Some(cache_create / 1_000_000.0),
                cache_read_input_token_cost: Some(cache_read / 1_000_000.0),
                input_cost_per_token_above_200k: Some(long_input / 1_000_000.0),
                output_cost_per_token_above_200k: Some(long_output / 1_000_000.0),
                cache_creation_cost_above_200k: Some(long_cache_create / 1_000_000.0),
                cache_read_cost_above_200k: Some(long_cache_read / 1_000_000.0),
                long_context_threshold: Some(OPENAI_LONG_CONTEXT_THRESHOLD),
            },
        );
    }

    pricing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pricing() -> ModelPricing {
        ModelPricing {
            input_cost_per_token: Some(3.0 / 1_000_000.0),
            output_cost_per_token: Some(15.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(3.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(0.30 / 1_000_000.0),
            input_cost_per_token_above_200k: Some(6.0 / 1_000_000.0),
            output_cost_per_token_above_200k: Some(30.0 / 1_000_000.0),
            cache_creation_cost_above_200k: Some(7.50 / 1_000_000.0),
            cache_read_cost_above_200k: Some(0.60 / 1_000_000.0),
            long_context_threshold: None,
        }
    }

    #[test]
    fn test_flat_cost_calculation() {
        let fetcher = PricingFetcher::new();
        let pricing = test_pricing();

        // Small context: flat pricing
        let cost = fetcher.calculate_cost(&pricing, 1000, 500, 0, 0);
        let expected = 1000.0 * 3.0 / 1_000_000.0 + 500.0 * 15.0 / 1_000_000.0;
        assert!(
            (cost - expected).abs() < 1e-10,
            "flat cost: {cost} != {expected}"
        );
    }

    #[test]
    fn test_flat_cost_with_cache() {
        let fetcher = PricingFetcher::new();
        let pricing = test_pricing();

        let cost = fetcher.calculate_cost(&pricing, 1000, 500, 2000, 5000);
        // total_input_context = 1000 + 5000 + 2000 = 8000 < 200k, so flat
        let expected = 1000.0 * 3.0 / 1_000_000.0
            + 500.0 * 15.0 / 1_000_000.0
            + 2000.0 * 3.75 / 1_000_000.0
            + 5000.0 * 0.30 / 1_000_000.0;
        assert!(
            (cost - expected).abs() < 1e-10,
            "flat cache cost: {cost} != {expected}"
        );
    }

    #[test]
    fn test_tiered_cost_kicks_in_above_threshold() {
        let fetcher = PricingFetcher::new();
        let pricing = test_pricing();

        // Total context > 200k triggers tiered pricing
        let cost_flat = fetcher.calculate_cost(&pricing, 1000, 500, 0, 0);
        let cost_tiered = fetcher.calculate_cost(&pricing, 1000, 500, 0, 300_000);

        // Tiered should be more expensive due to higher rates above 200k
        assert!(
            cost_tiered > cost_flat,
            "tiered {cost_tiered} should > flat {cost_flat}"
        );
        assert!(cost_tiered > 0.0, "tiered cost should be positive");
    }

    #[test]
    fn test_zero_tokens_returns_zero() {
        let fetcher = PricingFetcher::new();
        let pricing = test_pricing();
        let cost = fetcher.calculate_cost(&pricing, 0, 0, 0, 0);
        assert!((cost - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_cost_components_are_additive() {
        let fetcher = PricingFetcher::new();
        let pricing = test_pricing();

        // Input only
        let input_cost = fetcher.calculate_cost(&pricing, 1000, 0, 0, 0);
        // Output only
        let output_cost = fetcher.calculate_cost(&pricing, 0, 1000, 0, 0);
        // Both
        let combined = fetcher.calculate_cost(&pricing, 1000, 1000, 0, 0);

        assert!(
            (combined - (input_cost + output_cost)).abs() < 1e-10,
            "costs should be additive: {combined} != {input_cost} + {output_cost}"
        );
    }

    #[test]
    fn test_get_model_pricing_direct_match() {
        let fetcher = PricingFetcher::new();
        let data = get_fallback_pricing();
        let result = fetcher.get_model_pricing(&data, "claude-sonnet-4-20250514");
        assert!(result.is_some());
    }

    #[test]
    fn test_get_model_pricing_alias_match() {
        let fetcher = PricingFetcher::new();
        let data = get_fallback_pricing();
        // "claude-sonnet-4-6" should resolve
        let result = fetcher.get_model_pricing(&data, "claude-sonnet-4-6");
        assert!(result.is_some());
    }

    #[test]
    fn test_get_model_pricing_unknown() {
        let fetcher = PricingFetcher::new();
        let data = get_fallback_pricing();
        let result = fetcher.get_model_pricing(&data, "nonexistent-model-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_fable_5_pricing_matches_current_rates() {
        let pricing = get_fallback_pricing()
            .remove("claude-fable-5")
            .expect("Fable 5 pricing should be registered");

        assert_eq!(pricing.input_cost_per_token, Some(10.0 / 1_000_000.0));
        assert_eq!(pricing.output_cost_per_token, Some(50.0 / 1_000_000.0));
        assert_eq!(
            pricing.cache_creation_input_token_cost,
            Some(12.5 / 1_000_000.0)
        );
        assert_eq!(pricing.cache_read_input_token_cost, Some(1.0 / 1_000_000.0));
    }

    #[test]
    fn test_sonnet_5_pricing_is_effective_dated() {
        let fetcher = PricingFetcher::new();
        let data = get_fallback_pricing();
        let introductory = fetcher
            .get_model_pricing_at(
                &data,
                "claude-sonnet-5",
                NaiveDate::from_ymd_opt(2026, 8, 31).unwrap(),
            )
            .unwrap();
        let standard = fetcher
            .get_model_pricing_at(
                &data,
                "claude-sonnet-5",
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            )
            .unwrap();

        assert_eq!(introductory.input_cost_per_token, Some(2.0 / 1_000_000.0));
        assert_eq!(introductory.output_cost_per_token, Some(10.0 / 1_000_000.0));
        assert_eq!(standard.input_cost_per_token, Some(3.0 / 1_000_000.0));
        assert_eq!(standard.output_cost_per_token, Some(15.0 / 1_000_000.0));
    }

    #[test]
    fn test_gpt_5_6_alias_resolves_to_sol() {
        let fetcher = PricingFetcher::new();
        let data = get_fallback_pricing();
        let alias = fetcher
            .get_model_pricing(&data, "gpt-5.6")
            .expect("GPT-5.6 alias should resolve");
        let sol = data
            .get("gpt-5.6-sol")
            .expect("GPT-5.6 Sol should be registered");

        assert_eq!(alias.input_cost_per_token, sol.input_cost_per_token);
        assert_eq!(alias.output_cost_per_token, sol.output_cost_per_token);
    }

    #[test]
    fn test_gpt_5_6_uses_whole_request_long_context_rates() {
        let fetcher = PricingFetcher::new();
        let pricing = get_fallback_pricing()
            .remove("gpt-5.6-sol")
            .expect("GPT-5.6 Sol pricing should be registered");

        let short = fetcher.calculate_cost(&pricing, 100_000, 1_000, 0, 100);
        let long = fetcher.calculate_cost(&pricing, 300_000, 1_000, 0, 100);

        assert!((short - 0.53005).abs() < 1e-9, "short cost was {short}");
        assert!((long - 3.0451).abs() < 1e-9, "long cost was {long}");
    }

    #[test]
    fn test_codex_fallback_rates_are_model_specific() {
        let pricing = get_fallback_pricing();
        for (model, input, cached, output) in [
            ("gpt-5-codex", 1.25, 0.125, 10.0),
            ("gpt-5.1-codex", 1.25, 0.125, 10.0),
            ("gpt-5.1-codex-mini", 0.25, 0.025, 2.0),
            ("gpt-5.2-codex", 1.75, 0.175, 14.0),
            ("gpt-5.3-codex", 1.75, 0.175, 14.0),
        ] {
            let model_pricing = pricing.get(model).unwrap();
            let input_rate = model_pricing.input_cost_per_token.unwrap();
            let cache_write_rate = model_pricing.cache_creation_input_token_cost.unwrap();
            let cached_rate = model_pricing.cache_read_input_token_cost.unwrap();
            let output_rate = model_pricing.output_cost_per_token.unwrap();
            assert!(
                (input_rate - input / 1_000_000.0).abs() < f64::EPSILON,
                "{model} input"
            );
            assert!(
                (cached_rate - cached / 1_000_000.0).abs() < f64::EPSILON,
                "{model} cached input"
            );
            assert!(
                (cache_write_rate - input / 1_000_000.0).abs() < f64::EPSILON,
                "{model} cache write"
            );
            assert!(
                (output_rate - output / 1_000_000.0).abs() < f64::EPSILON,
                "{model} output"
            );
        }
    }

    #[test]
    fn test_fast_mode_multiplier() {
        assert!((FAST_MODE_MULTIPLIER - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tiered_threshold() {
        assert_eq!(TIERED_THRESHOLD, 200_000);
    }
}
