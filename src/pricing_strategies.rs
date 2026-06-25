use crate::domain::{Cost, CostCalculator, ModelName, PricingModel, TokenUsage};
use crate::error::{ClaudelyticsError, Result};
use crate::pricing::{canonical_model_name, get_fallback_pricing};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 価格計算戦略の実装
/// デフォルトの価格情報を提供する戦略
#[derive(Debug)]
pub struct FallbackPricingStrategy {
    pricing_data: HashMap<String, PricingModel>,
}

impl FallbackPricingStrategy {
    pub fn new() -> Self {
        Self {
            pricing_data: Self::create_default_pricing(),
        }
    }

    fn create_default_pricing() -> HashMap<String, PricingModel> {
        get_fallback_pricing()
            .into_iter()
            .filter(|(name, _)| !name.starts_with("__"))
            .map(|(name, pricing)| {
                (
                    name,
                    PricingModel {
                        input_cost_per_token: pricing.input_cost_per_token.unwrap_or_default(),
                        output_cost_per_token: pricing.output_cost_per_token.unwrap_or_default(),
                        cache_creation_cost_per_token: pricing
                            .cache_creation_input_token_cost
                            .unwrap_or_default(),
                        cache_read_cost_per_token: pricing
                            .cache_read_input_token_cost
                            .unwrap_or_default(),
                    },
                )
            })
            .collect()
    }

    fn find_model_pricing(&self, model_name: &str) -> Option<&PricingModel> {
        // 直接マッチング
        if let Some(pricing) = self.pricing_data.get(model_name) {
            return Some(pricing);
        }

        if let Some(pricing) = canonical_model_name(model_name)
            .and_then(|canonical_name| self.pricing_data.get(canonical_name))
        {
            return Some(pricing);
        }

        self.pricing_data
            .iter()
            .filter(|(key, _)| key.contains(model_name) || model_name.contains(key.as_str()))
            .max_by(|(left, _), (right, _)| {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            })
            .map(|(_, pricing)| pricing)
    }
}

impl Default for FallbackPricingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl CostCalculator for FallbackPricingStrategy {
    fn calculate_cost(&self, model: &ModelName, usage: &TokenUsage) -> Option<Cost> {
        let pricing = self.find_model_pricing(&model.0)?;
        Some(pricing.calculate_cost(usage))
    }
}

/// 設定ファイルベースの価格計算戦略
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigurablePricingStrategy {
    pricing_data: HashMap<String, ConfigurablePricingModel>,
    #[serde(skip)]
    fallback: Option<FallbackPricingStrategy>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigurablePricingModel {
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub cache_creation_cost_per_token: f64,
    pub cache_read_cost_per_token: f64,
    pub aliases: Option<Vec<String>>,
}

#[allow(dead_code)]
impl ConfigurablePricingStrategy {
    pub fn new() -> Self {
        Self {
            pricing_data: HashMap::new(),
            fallback: Some(FallbackPricingStrategy::new()),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ClaudelyticsError::config_error(&format!("Failed to read config file: {}", e))
        })?;

        let mut strategy: Self = serde_yaml::from_str(&content)?;
        strategy.fallback = Some(FallbackPricingStrategy::new());
        Ok(strategy)
    }

    pub fn add_model(&mut self, model_name: String, pricing: ConfigurablePricingModel) {
        self.pricing_data.insert(model_name, pricing);
    }

    fn find_model_pricing(&self, model_name: &str) -> Option<&ConfigurablePricingModel> {
        // 直接マッチング
        if let Some(pricing) = self.pricing_data.get(model_name) {
            return Some(pricing);
        }

        // エイリアスマッチング
        for pricing in self.pricing_data.values() {
            if let Some(aliases) = &pricing.aliases
                && aliases.iter().any(|alias| alias == model_name)
            {
                return Some(pricing);
            }
        }

        None
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(&self.pricing_data).map_err(|e| {
            ClaudelyticsError::config_error(&format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, content).map_err(|e| {
            ClaudelyticsError::config_error(&format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }
}

impl Default for ConfigurablePricingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl CostCalculator for ConfigurablePricingStrategy {
    fn calculate_cost(&self, model: &ModelName, usage: &TokenUsage) -> Option<Cost> {
        // 設定された価格を試す
        if let Some(pricing) = self.find_model_pricing(&model.0) {
            let pricing_model = PricingModel {
                input_cost_per_token: pricing.input_cost_per_token,
                output_cost_per_token: pricing.output_cost_per_token,
                cache_creation_cost_per_token: pricing.cache_creation_cost_per_token,
                cache_read_cost_per_token: pricing.cache_read_cost_per_token,
            };
            return Some(pricing_model.calculate_cost(usage));
        }

        // フォールバックを使用
        if let Some(fallback) = &self.fallback {
            return fallback.calculate_cost(model, usage);
        }

        None
    }
}

/// 複数の戦略を組み合わせる戦略
pub struct CompositeCostCalculator {
    strategies: Vec<Box<dyn CostCalculator + Send + Sync>>,
}

impl CompositeCostCalculator {
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    pub fn add_strategy(mut self, strategy: Box<dyn CostCalculator + Send + Sync>) -> Self {
        self.strategies.push(strategy);
        self
    }
}

impl Default for CompositeCostCalculator {
    fn default() -> Self {
        Self::new().add_strategy(Box::new(FallbackPricingStrategy::new()))
    }
}

impl CostCalculator for CompositeCostCalculator {
    fn calculate_cost(&self, model: &ModelName, usage: &TokenUsage) -> Option<Cost> {
        for strategy in &self.strategies {
            if let Some(cost) = strategy.calculate_cost(model, usage) {
                return Some(cost);
            }
        }
        None
    }
}

/// 価格計算戦略のファクトリー
#[allow(dead_code)]
pub struct CostCalculatorFactory;

#[allow(dead_code)]
impl CostCalculatorFactory {
    /// デフォルトの価格計算戦略を作成
    pub fn create_default() -> Box<dyn CostCalculator + Send + Sync> {
        Box::new(FallbackPricingStrategy::new())
    }

    /// 設定ファイルベースの価格計算戦略を作成
    pub fn create_from_config<P: AsRef<Path>>(
        path: P,
    ) -> Result<Box<dyn CostCalculator + Send + Sync>> {
        let strategy = ConfigurablePricingStrategy::from_file(path)?;
        Ok(Box::new(strategy))
    }

    /// 複数の戦略を組み合わせた価格計算戦略を作成
    pub fn create_composite() -> Box<dyn CostCalculator + Send + Sync> {
        Box::new(CompositeCostCalculator::default())
    }

    /// 設定ファイルがあれば使用し、なければデフォルトを使用
    pub fn create_smart<P: AsRef<Path>>(config_path: P) -> Box<dyn CostCalculator + Send + Sync> {
        if config_path.as_ref().exists() {
            match Self::create_from_config(config_path) {
                Ok(calculator) => calculator,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load config, using default pricing: {}",
                        e
                    );
                    Self::create_default()
                }
            }
        } else {
            Self::create_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_pricing_strategy() {
        let strategy = FallbackPricingStrategy::new();
        let model = ModelName("claude-opus-4-20250514".to_string());
        let usage = TokenUsage::new(1000, 2000, 500, 100);

        let cost = strategy.calculate_cost(&model, &usage);
        assert!(cost.is_some());
        assert!(cost.unwrap().0 > 0.0);
    }

    #[test]
    fn test_model_name_variations() {
        let strategy = FallbackPricingStrategy::new();

        // Test direct match
        let model1 = ModelName("claude-opus-4-20250514".to_string());
        let usage = TokenUsage::new(1000, 1000, 0, 0);
        assert!(strategy.calculate_cost(&model1, &usage).is_some());
    }

    #[test]
    fn test_configurable_pricing_strategy() {
        let mut strategy = ConfigurablePricingStrategy::new();

        let pricing_model = ConfigurablePricingModel {
            input_cost_per_token: 0.001,
            output_cost_per_token: 0.002,
            cache_creation_cost_per_token: 0.0015,
            cache_read_cost_per_token: 0.0001,
            aliases: Some(vec!["test-model".to_string()]),
        };

        strategy.add_model("custom-model".to_string(), pricing_model);

        let model = ModelName("test-model".to_string());
        let usage = TokenUsage::new(1000, 1000, 0, 0);

        let cost = strategy.calculate_cost(&model, &usage);
        assert!(cost.is_some());
        assert_eq!(cost.unwrap().0, 3.0); // 1000 * 0.001 + 1000 * 0.002
    }

    #[test]
    fn test_composite_calculator() {
        let calculator = CompositeCostCalculator::default();
        let model = ModelName("claude-opus-4-20250514".to_string());
        let usage = TokenUsage::new(1000, 1000, 0, 0);

        let cost = calculator.calculate_cost(&model, &usage);
        assert!(cost.is_some());
    }
}
