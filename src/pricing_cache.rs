use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::pricing::{ModelPricing, get_fallback_pricing};

/// Cache duration for pricing data (7 days)
const CACHE_DURATION_DAYS: i64 = 7;

/// Offline pricing cache for storing model pricing data
#[derive(Debug, Serialize, Deserialize)]
pub struct PricingCache {
    /// Cached pricing data
    pub pricing_data: HashMap<String, ModelPricing>,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
    /// Version for cache compatibility
    pub version: String,
}

impl PricingCache {
    /// Create a new pricing cache with current data
    pub fn new() -> Self {
        Self {
            pricing_data: get_fallback_pricing(),
            last_updated: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Check if the cache is still valid
    pub fn is_valid(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age < Duration::days(CACHE_DURATION_DAYS)
    }

    /// Update the cache with new pricing data
    #[allow(dead_code)]
    pub fn update(&mut self, pricing_data: HashMap<String, ModelPricing>) {
        self.pricing_data = pricing_data;
        self.last_updated = Utc::now();
    }

    /// Get cache file path
    fn get_cache_path() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;

        let claudelytics_cache = cache_dir.join("claudelytics");
        fs::create_dir_all(&claudelytics_cache)?;

        Ok(claudelytics_cache.join("pricing_cache.json"))
    }

    /// Load cache from disk
    pub fn load() -> Result<Option<Self>> {
        let cache_path = Self::get_cache_path()?;

        if !cache_path.exists() {
            return Ok(None);
        }

        let cache_data = fs::read_to_string(&cache_path)
            .with_context(|| format!("Failed to read cache file: {}", cache_path.display()))?;

        let cache: PricingCache =
            serde_json::from_str(&cache_data).with_context(|| "Failed to parse pricing cache")?;

        // Check version compatibility
        if cache.version != env!("CARGO_PKG_VERSION") {
            return Ok(None);
        }

        Ok(Some(cache))
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let cache_path = Self::get_cache_path()?;

        let cache_data =
            serde_json::to_string_pretty(self).context("Failed to serialize pricing cache")?;

        fs::write(&cache_path, cache_data)
            .with_context(|| format!("Failed to write cache file: {}", cache_path.display()))?;

        Ok(())
    }

    /// Clear the cache
    pub fn clear() -> Result<()> {
        let cache_path = Self::get_cache_path()?;

        if cache_path.exists() {
            fs::remove_file(&cache_path).with_context(|| {
                format!("Failed to remove cache file: {}", cache_path.display())
            })?;
        }

        Ok(())
    }
}

/// Enhanced pricing fetcher with offline cache support
#[allow(dead_code)]
pub struct CachedPricingFetcher {
    cache: Option<PricingCache>,
    fallback_pricing: HashMap<String, ModelPricing>,
}

#[allow(dead_code)]
impl CachedPricingFetcher {
    /// Create a new cached pricing fetcher
    pub fn new() -> Self {
        // Try to load cache from disk
        let cache = PricingCache::load().unwrap_or(None);

        Self {
            cache,
            fallback_pricing: get_fallback_pricing(),
        }
    }

    /// Get pricing data with cache support
    pub fn get_pricing_data(&self) -> &HashMap<String, ModelPricing> {
        // If we have a valid cache, use it
        if let Some(ref cache) = self.cache {
            if cache.is_valid() {
                return &cache.pricing_data;
            }
        }

        // Otherwise use fallback pricing
        &self.fallback_pricing
    }

    /// Try to fetch pricing from online source (placeholder for future implementation)
    pub async fn fetch_online_pricing(&mut self) -> Result<()> {
        // In the future, this would fetch from an API
        // For now, we'll just update the cache with fallback data

        let mut new_cache = PricingCache::new();
        new_cache.update(self.fallback_pricing.clone());
        new_cache.save()?;

        self.cache = Some(new_cache);
        Ok(())
    }

    /// Get model pricing with caching
    pub fn get_model_pricing(&self, model_name: &str) -> Option<&ModelPricing> {
        let pricing_data = self.get_pricing_data();

        // Direct match first
        if let Some(pricing) = pricing_data.get(model_name) {
            return Some(pricing);
        }

        // Try variations for Claude models
        let variations = Self::get_model_variations(model_name);

        for variation in &variations {
            if !variation.is_empty() {
                if let Some(pricing) = pricing_data.get(variation) {
                    return Some(pricing);
                }
            }
        }

        // Partial match as fallback
        for (key, pricing) in pricing_data {
            if key.contains(model_name) || model_name.contains(key) {
                return Some(pricing);
            }
        }

        None
    }

    /// Get model name variations for matching
    fn get_model_variations(model_name: &str) -> Vec<String> {
        vec![
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
        ]
    }

    /// Calculate cost using cached pricing
    pub fn calculate_cost(
        &self,
        model_name: &str,
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
    ) -> f64 {
        if let Some(pricing) = self.get_model_pricing(model_name) {
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
        } else {
            0.0
        }
    }
}

impl Default for CachedPricingFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_cache_validity() {
        let mut cache = PricingCache::new();
        assert!(cache.is_valid());

        // Simulate old cache
        cache.last_updated = Utc::now() - Duration::days(8);
        assert!(!cache.is_valid());
    }

    #[test]
    fn test_model_variations() {
        let variations = CachedPricingFetcher::get_model_variations("claude-sonnet-4");
        assert!(variations.contains(&"claude-sonnet-4-20250514".to_string()));

        let variations = CachedPricingFetcher::get_model_variations("opus-4");
        assert!(variations.contains(&"claude-opus-4-20250514".to_string()));
    }

    #[test]
    fn test_cached_pricing_fetcher() {
        let fetcher = CachedPricingFetcher::new();
        let pricing_data = fetcher.get_pricing_data();
        assert!(!pricing_data.is_empty());

        // Test model pricing lookup
        let sonnet_pricing = fetcher.get_model_pricing("claude-sonnet-4-20250514");
        assert!(sonnet_pricing.is_some());

        // Test cost calculation
        let cost = fetcher.calculate_cost("claude-sonnet-4-20250514", 1000, 500, 0, 0);
        assert!(cost > 0.0);
    }
}
