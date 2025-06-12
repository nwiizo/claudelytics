use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub aliases: Vec<String>,
    pub family: String,
    pub version: Option<String>,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelsRegistry {
    models: HashMap<String, ModelInfo>,
    families: HashMap<String, Vec<String>>,
}

impl ModelsRegistry {
    pub fn new() -> Self {
        let mut registry = ModelsRegistry {
            models: HashMap::new(),
            families: HashMap::new(),
        };

        // Initialize with known models
        registry.register_default_models();
        registry
    }

    fn register_default_models(&mut self) {
        // Opus models
        self.register_model(ModelInfo {
            name: "claude-opus-4-20250514".to_string(),
            aliases: vec!["opus-4".to_string(), "opus4".to_string()],
            family: "opus".to_string(),
            version: Some("4.0".to_string()),
            release_date: Some("2025-05-14".to_string()),
        });

        self.register_model(ModelInfo {
            name: "claude-3-opus-20240229".to_string(),
            aliases: vec!["opus-3".to_string(), "opus3".to_string()],
            family: "opus".to_string(),
            version: Some("3.0".to_string()),
            release_date: Some("2024-02-29".to_string()),
        });

        // Sonnet models
        self.register_model(ModelInfo {
            name: "claude-sonnet-4-20250514".to_string(),
            aliases: vec!["sonnet-4".to_string(), "sonnet4".to_string()],
            family: "sonnet".to_string(),
            version: Some("4.0".to_string()),
            release_date: Some("2025-05-14".to_string()),
        });

        self.register_model(ModelInfo {
            name: "claude-3-5-sonnet-20241022".to_string(),
            aliases: vec!["sonnet-3.5".to_string(), "sonnet3.5".to_string()],
            family: "sonnet".to_string(),
            version: Some("3.5".to_string()),
            release_date: Some("2024-10-22".to_string()),
        });

        // Haiku models
        self.register_model(ModelInfo {
            name: "claude-3-5-haiku-20241022".to_string(),
            aliases: vec!["haiku-3.5".to_string(), "haiku3.5".to_string()],
            family: "haiku".to_string(),
            version: Some("3.5".to_string()),
            release_date: Some("2024-10-22".to_string()),
        });

        self.register_model(ModelInfo {
            name: "claude-3-haiku-20240307".to_string(),
            aliases: vec!["haiku-3".to_string(), "haiku3".to_string()],
            family: "haiku".to_string(),
            version: Some("3.0".to_string()),
            release_date: Some("2024-03-07".to_string()),
        });
    }

    pub fn register_model(&mut self, model: ModelInfo) {
        // Add to family mapping
        self.families
            .entry(model.family.clone())
            .or_default()
            .push(model.name.clone());

        // Add model
        self.models.insert(model.name.clone(), model);
    }

    pub fn matches_filter(&self, model_name: &str, filter: &str) -> bool {
        let filter_lower = filter.to_lowercase();
        let model_lower = model_name.to_lowercase();

        // Direct partial match
        if model_lower.contains(&filter_lower) {
            return true;
        }

        // Check if filter is a family name
        if self.families.contains_key(&filter_lower) {
            if let Some(model_info) = self.get_model_info(model_name) {
                return model_info.family.to_lowercase() == filter_lower;
            }
        }

        // Check aliases
        if let Some(model_info) = self.get_model_info(model_name) {
            for alias in &model_info.aliases {
                if alias.to_lowercase() == filter_lower {
                    return true;
                }
            }
        }

        // For unknown models, fall back to simple contains check
        // This ensures forward compatibility
        false
    }

    pub fn get_model_info(&self, model_name: &str) -> Option<&ModelInfo> {
        // Direct lookup
        if let Some(info) = self.models.get(model_name) {
            return Some(info);
        }

        // Try to find by partial match
        for (name, info) in &self.models {
            if model_name.contains(name) || name.contains(model_name) {
                return Some(info);
            }
        }

        None
    }

    #[allow(dead_code)]
    pub fn get_model_family(&self, model_name: &str) -> Option<String> {
        // First try exact lookup
        if let Some(info) = self.get_model_info(model_name) {
            return Some(info.family.clone());
        }

        // Heuristic detection for unknown models
        let model_lower = model_name.to_lowercase();
        if model_lower.contains("opus") {
            return Some("opus".to_string());
        } else if model_lower.contains("sonnet") {
            return Some("sonnet".to_string());
        } else if model_lower.contains("haiku") {
            return Some("haiku".to_string());
        }

        None
    }

    pub fn list_models(&self) -> Vec<&ModelInfo> {
        let mut models: Vec<_> = self.models.values().collect();
        models.sort_by_key(|m| (&m.family, &m.version));
        models
    }

    #[allow(dead_code)]
    pub fn list_families(&self) -> Vec<String> {
        let mut families: Vec<_> = self.families.keys().cloned().collect();
        families.sort();
        families
    }

    #[allow(dead_code)]
    pub fn get_models_by_family(&self, family: &str) -> Vec<&ModelInfo> {
        self.families
            .get(family)
            .map(|model_names| {
                model_names
                    .iter()
                    .filter_map(|name| self.models.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for ModelsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_matching() {
        let registry = ModelsRegistry::new();

        // Test family matching
        assert!(registry.matches_filter("claude-opus-4-20250514", "opus"));
        assert!(registry.matches_filter("claude-sonnet-4-20250514", "sonnet"));

        // Test exact model matching
        assert!(registry.matches_filter("claude-opus-4-20250514", "claude-opus-4"));

        // Test alias matching
        assert!(registry.matches_filter("claude-opus-4-20250514", "opus4"));

        // Test non-matching
        assert!(!registry.matches_filter("claude-opus-4-20250514", "sonnet"));
    }

    #[test]
    fn test_family_detection() {
        let registry = ModelsRegistry::new();

        assert_eq!(
            registry.get_model_family("claude-opus-4-20250514"),
            Some("opus".to_string())
        );

        // Test unknown model with heuristic
        assert_eq!(
            registry.get_model_family("claude-opus-5-20260101"),
            Some("opus".to_string())
        );
    }
}
