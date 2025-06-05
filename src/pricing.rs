use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
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
            format!("claude-3-5-{}", model_name.trim_start_matches("claude-")),
            format!("claude-3-{}", model_name.trim_start_matches("claude-")),
            format!("claude-{}", model_name.trim_start_matches("claude-")),
            model_name.replace("claude-", ""),
        ];

        for variation in &claude_variations {
            if let Some(pricing) = pricing_data.get(variation) {
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

// Fallback pricing for common Claude models (as of 2024)
pub fn get_fallback_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

    // Claude 4 Opus (New model)
    pricing.insert(
        "claude-opus-4-20250514".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.015 / 1000.0),
            output_cost_per_token: Some(0.075 / 1000.0),
            cache_creation_input_token_cost: Some(0.01875 / 1000.0),
            cache_read_input_token_cost: Some(0.0015 / 1000.0),
        },
    );

    // Claude 3.5 Sonnet
    pricing.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.003 / 1000.0),
            output_cost_per_token: Some(0.015 / 1000.0),
            cache_creation_input_token_cost: Some(0.00375 / 1000.0),
            cache_read_input_token_cost: Some(0.0003 / 1000.0),
        },
    );

    // Claude 3.5 Haiku
    pricing.insert(
        "claude-3-5-haiku-20241022".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.0008 / 1000.0),
            output_cost_per_token: Some(0.004 / 1000.0),
            cache_creation_input_token_cost: Some(0.001 / 1000.0),
            cache_read_input_token_cost: Some(0.00008 / 1000.0),
        },
    );

    // Claude 3 Opus
    pricing.insert(
        "claude-3-opus-20240229".to_string(),
        ModelPricing {
            input_cost_per_token: Some(0.015 / 1000.0),
            output_cost_per_token: Some(0.075 / 1000.0),
            cache_creation_input_token_cost: Some(0.01875 / 1000.0),
            cache_read_input_token_cost: Some(0.0015 / 1000.0),
        },
    );

    pricing
}
