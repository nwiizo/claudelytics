use serde::Deserialize;
use std::collections::HashMap;

/// Fast mode pricing multiplier (Claude Code /fast uses 6x pricing)
pub const FAST_MODE_MULTIPLIER: f64 = 6.0;

/// Token threshold for tiered pricing on 1M context models
pub const TIERED_THRESHOLD: u64 = 200_000;

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

    pub fn get_model_pricing(
        &self,
        pricing_data: &HashMap<String, ModelPricing>,
        model_name: &str,
    ) -> Option<ModelPricing> {
        // Direct match first
        if let Some(pricing) = pricing_data.get(model_name) {
            return Some(pricing.clone());
        }

        // Try variations for Claude models
        let claude_variations = [
            // Exact model name variations
            model_name.to_string(),
            format!("claude-3-5-{}", model_name.trim_start_matches("claude-")),
            format!("claude-3-{}", model_name.trim_start_matches("claude-")),
            format!("claude-{}", model_name.trim_start_matches("claude-")),
            model_name.replace("claude-", ""),
            // Handle specific model mappings
            if model_name.contains("sonnet-4-6") || model_name.contains("sonnet-4.6") {
                "claude-sonnet-4-6-20260310".to_string()
            } else if model_name.contains("sonnet-4") {
                "claude-sonnet-4-20250514".to_string()
            } else {
                String::new()
            },
            if model_name.contains("opus-4-6") || model_name.contains("opus-4.6") {
                "claude-opus-4-6-20260310".to_string()
            } else if model_name.contains("opus-4") {
                "claude-opus-4-20250514".to_string()
            } else {
                String::new()
            },
            if model_name.contains("haiku-4-5") || model_name.contains("haiku-4.5") {
                "claude-haiku-4-5-20251001".to_string()
            } else {
                String::new()
            },
        ];

        for variation in &claude_variations {
            if !variation.is_empty()
                && let Some(pricing) = pricing_data.get(variation)
            {
                return Some(pricing.clone());
            }
        }

        // Partial match as fallback
        for (key, pricing) in pricing_data {
            if key.contains(model_name) || model_name.contains(key) {
                return Some(pricing.clone());
            }
        }

        None
    }

    pub fn calculate_cost(
        &self,
        pricing: &ModelPricing,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
    ) -> f64 {
        let total_input_context = input_tokens + cache_read_tokens + cache_creation_tokens;
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

// Fallback pricing for common Claude models (as of 2025)
// Updated to match ccusage pricing methodology
pub fn get_fallback_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

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
        },
    );

    // Claude 3.5 Sonnet - $3/$15 per MTok (no tiered pricing, 200k context)
    pricing.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        ModelPricing {
            input_cost_per_token: Some(3.0 / 1_000_000.0),
            output_cost_per_token: Some(15.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(3.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(0.3 / 1_000_000.0),
            input_cost_per_token_above_200k: None,
            output_cost_per_token_above_200k: None,
            cache_creation_cost_above_200k: None,
            cache_read_cost_above_200k: None,
        },
    );

    // Claude 3.5 Haiku - $0.80/$4 per MTok (no tiered pricing)
    pricing.insert(
        "claude-3-5-haiku-20241022".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.8 / 1_000_000.0),
            output_cost_per_token: Some(4.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(1.0 / 1_000_000.0),
            cache_read_input_token_cost: Some(0.08 / 1_000_000.0),
            input_cost_per_token_above_200k: None,
            output_cost_per_token_above_200k: None,
            cache_creation_cost_above_200k: None,
            cache_read_cost_above_200k: None,
        },
    );

    // Claude 3 Opus - $15/$75 per MTok (no tiered pricing, 200k context)
    pricing.insert(
        "claude-3-opus-20240229".to_string(),
        ModelPricing {
            input_cost_per_token: Some(15.0 / 1_000_000.0),
            output_cost_per_token: Some(75.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(18.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(1.5 / 1_000_000.0),
            input_cost_per_token_above_200k: None,
            output_cost_per_token_above_200k: None,
            cache_creation_cost_above_200k: None,
            cache_read_cost_above_200k: None,
        },
    );

    // Claude Opus 4.6 - $15/$75 per MTok, above 200k: $30/$150
    pricing.insert(
        "claude-opus-4-6-20260310".to_string(),
        ModelPricing {
            input_cost_per_token: Some(15.0 / 1_000_000.0),
            output_cost_per_token: Some(75.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(18.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(1.5 / 1_000_000.0),
            input_cost_per_token_above_200k: Some(30.0 / 1_000_000.0),
            output_cost_per_token_above_200k: Some(150.0 / 1_000_000.0),
            cache_creation_cost_above_200k: Some(37.5 / 1_000_000.0),
            cache_read_cost_above_200k: Some(3.0 / 1_000_000.0),
        },
    );

    // Claude Sonnet 4.6 - $3/$15 per MTok, above 200k: $6/$30
    pricing.insert(
        "claude-sonnet-4-6-20260310".to_string(),
        ModelPricing {
            input_cost_per_token: Some(3.0 / 1_000_000.0),
            output_cost_per_token: Some(15.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(3.75 / 1_000_000.0),
            cache_read_input_token_cost: Some(0.3 / 1_000_000.0),
            input_cost_per_token_above_200k: Some(6.0 / 1_000_000.0),
            output_cost_per_token_above_200k: Some(30.0 / 1_000_000.0),
            cache_creation_cost_above_200k: Some(7.5 / 1_000_000.0),
            cache_read_cost_above_200k: Some(0.6 / 1_000_000.0),
        },
    );

    // Claude Haiku 4.5 - $0.80/$4 per MTok (no tiered pricing)
    pricing.insert(
        "claude-haiku-4-5-20251001".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.8 / 1_000_000.0),
            output_cost_per_token: Some(4.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(1.0 / 1_000_000.0),
            cache_read_input_token_cost: Some(0.08 / 1_000_000.0),
            input_cost_per_token_above_200k: None,
            output_cost_per_token_above_200k: None,
            cache_creation_cost_above_200k: None,
            cache_read_cost_above_200k: None,
        },
    );

    pricing
}
