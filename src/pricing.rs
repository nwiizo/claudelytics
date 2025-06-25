use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct ModelPricing {
    #[serde(rename = "input_cost_per_token")]
    pub input_cost_per_token: Option<f64>,
    #[serde(rename = "output_cost_per_token")]
    pub output_cost_per_token: Option<f64>,
    #[serde(rename = "cache_creation_input_token_cost")]
    pub cache_creation_input_token_cost: Option<f64>,
    #[serde(rename = "cache_read_input_token_cost")]
    pub cache_read_input_token_cost: Option<f64>,
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
            if model_name.contains("sonnet-4") {
                "claude-sonnet-4-20250514".to_string()
            } else {
                String::new()
            },
            if model_name.contains("opus-4") {
                "claude-opus-4-20250514".to_string()
            } else {
                String::new()
            },
        ];

        for variation in &claude_variations {
            if !variation.is_empty() {
                if let Some(pricing) = pricing_data.get(variation) {
                    return Some(pricing.clone());
                }
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
}

// Fallback pricing for common Claude models (as of 2025)
// Updated to match ccusage pricing methodology
pub fn get_fallback_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

    // Claude Sonnet 4 (Latest model) - Official pricing: $3.00/$15.00 per million tokens
    pricing.insert(
        "claude-sonnet-4-20250514".to_string(),
        ModelPricing {
            input_cost_per_token: Some(3.0 / 1_000_000.0),
            output_cost_per_token: Some(15.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(3.75 / 1_000_000.0), // 25% markup for cache creation
            cache_read_input_token_cost: Some(0.3 / 1_000_000.0), // 90% discount for cache reads
        },
    );

    // Claude 4 Opus (New model) - Official pricing: $15.00/$75.00 per million tokens
    pricing.insert(
        "claude-opus-4-20250514".to_string(),
        ModelPricing {
            input_cost_per_token: Some(15.0 / 1_000_000.0),
            output_cost_per_token: Some(75.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(18.75 / 1_000_000.0), // 25% markup for cache creation
            cache_read_input_token_cost: Some(1.5 / 1_000_000.0), // 90% discount for cache reads
        },
    );

    // Claude 3.5 Sonnet - Official pricing: $3.00/$15.00 per million tokens
    pricing.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        ModelPricing {
            input_cost_per_token: Some(3.0 / 1_000_000.0),
            output_cost_per_token: Some(15.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(3.75 / 1_000_000.0), // 25% markup for cache creation
            cache_read_input_token_cost: Some(0.3 / 1_000_000.0), // 90% discount for cache reads
        },
    );

    // Claude 3.5 Haiku - Official pricing: $0.80/$4.00 per million tokens
    pricing.insert(
        "claude-3-5-haiku-20241022".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.8 / 1_000_000.0),
            output_cost_per_token: Some(4.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(1.0 / 1_000_000.0), // 25% markup for cache creation
            cache_read_input_token_cost: Some(0.08 / 1_000_000.0), // 90% discount for cache reads
        },
    );

    // Claude 3 Opus - Official pricing: $15.00/$75.00 per million tokens
    pricing.insert(
        "claude-3-opus-20240229".to_string(),
        ModelPricing {
            input_cost_per_token: Some(15.0 / 1_000_000.0),
            output_cost_per_token: Some(75.0 / 1_000_000.0),
            cache_creation_input_token_cost: Some(18.75 / 1_000_000.0), // 25% markup for cache creation
            cache_read_input_token_cost: Some(1.5 / 1_000_000.0), // 90% discount for cache reads
        },
    );

    pricing
}
