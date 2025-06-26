use crate::domain::{Cost, CostCalculator, ModelName, PricingModel, TokenUsage};
use crate::error::{ClaudelyticsError, Result};
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
        let mut pricing = HashMap::new();

        // Claude 4 Opus (2025年の最新モデル)
        pricing.insert(
            "claude-opus-4-20250514".to_string(),
            PricingModel {
                input_cost_per_token: 15.0 / 1_000_000.0,
                output_cost_per_token: 75.0 / 1_000_000.0,
                cache_creation_cost_per_token: 18.75 / 1_000_000.0,
                cache_read_cost_per_token: 1.5 / 1_000_000.0,
            },
        );

        // Claude 4 Sonnet
        pricing.insert(
            "claude-sonnet-4-20250514".to_string(),
            PricingModel {
                input_cost_per_token: 3.0 / 1_000_000.0,
                output_cost_per_token: 15.0 / 1_000_000.0,
                cache_creation_cost_per_token: 3.75 / 1_000_000.0,
                cache_read_cost_per_token: 0.3 / 1_000_000.0,
            },
        );

        // Claude 3.5 Sonnet
        pricing.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            PricingModel {
                input_cost_per_token: 3.0 / 1_000_000.0,
                output_cost_per_token: 15.0 / 1_000_000.0,
                cache_creation_cost_per_token: 3.75 / 1_000_000.0,
                cache_read_cost_per_token: 0.3 / 1_000_000.0,
            },
        );

        // Claude 3.5 Haiku
        pricing.insert(
            "claude-3-5-haiku-20241022".to_string(),
            PricingModel {
                input_cost_per_token: 0.8 / 1_000_000.0,
                output_cost_per_token: 4.0 / 1_000_000.0,
                cache_creation_cost_per_token: 1.0 / 1_000_000.0,
                cache_read_cost_per_token: 0.08 / 1_000_000.0,
            },
        );

        // Claude 3 Opus
        pricing.insert(
            "claude-3-opus-20240229".to_string(),
            PricingModel {
                input_cost_per_token: 15.0 / 1_000_000.0,
                output_cost_per_token: 75.0 / 1_000_000.0,
                cache_creation_cost_per_token: 18.75 / 1_000_000.0,
                cache_read_cost_per_token: 1.5 / 1_000_000.0,
            },
        );

        pricing
    }

    fn find_model_pricing(&self, model_name: &str) -> Option<&PricingModel> {
        // 直接マッチング
        if let Some(pricing) = self.pricing_data.get(model_name) {
            return Some(pricing);
        }

        // Claude モデルのバリエーションを試す
        let variations = [
            format!("claude-3-5-{}", model_name.trim_start_matches("claude-")),
            format!("claude-3-{}", model_name.trim_start_matches("claude-")),
            format!("claude-{}", model_name.trim_start_matches("claude-")),
            model_name.replace("claude-", ""),
        ];

        for variation in &variations {
            if let Some(pricing) = self.pricing_data.get(variation) {
                return Some(pricing);
            }
        }

        // 部分マッチングフォールバック
        for (key, pricing) in &self.pricing_data {
            if key.contains(model_name) || model_name.contains(key) {
                return Some(pricing);
            }
        }

        None
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
            if let Some(aliases) = &pricing.aliases {
                if aliases.iter().any(|alias| alias == model_name) {
                    return Some(pricing);
                }
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
