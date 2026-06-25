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

        // Current generally available Claude models use canonical dateless IDs
        // from the 4.6 generation onward.
        self.register_model(ModelInfo {
            name: "claude-fable-5".to_string(),
            aliases: vec!["fable-5".to_string(), "fable5".to_string()],
            family: "fable".to_string(),
            version: Some("5.0".to_string()),
            release_date: Some("2026-06-09".to_string()),
        });

        self.register_model(ModelInfo {
            name: "claude-opus-5".to_string(),
            aliases: vec!["opus-5".to_string(), "opus5".to_string()],
            family: "opus".to_string(),
            version: Some("5.0".to_string()),
            release_date: None,
        });

        self.register_model(ModelInfo {
            name: "claude-opus-4-8".to_string(),
            aliases: vec![
                "opus-4.8".to_string(),
                "opus-4-8".to_string(),
                "opus48".to_string(),
            ],
            family: "opus".to_string(),
            version: Some("4.8".to_string()),
            release_date: None,
        });

        self.register_model(ModelInfo {
            name: "claude-opus-4-7".to_string(),
            aliases: vec![
                "opus-4.7".to_string(),
                "opus-4-7".to_string(),
                "opus47".to_string(),
            ],
            family: "opus".to_string(),
            version: Some("4.7".to_string()),
            release_date: None,
        });

        self.register_model(ModelInfo {
            name: "claude-opus-4-6".to_string(),
            aliases: vec![
                "opus-4.6".to_string(),
                "opus-4-6".to_string(),
                "opus46".to_string(),
            ],
            family: "opus".to_string(),
            version: Some("4.6".to_string()),
            release_date: None,
        });

        self.register_model(ModelInfo {
            name: "claude-sonnet-5".to_string(),
            aliases: vec!["sonnet-5".to_string(), "sonnet5".to_string()],
            family: "sonnet".to_string(),
            version: Some("5.0".to_string()),
            release_date: None,
        });

        self.register_model(ModelInfo {
            name: "claude-sonnet-4-6".to_string(),
            aliases: vec![
                "sonnet-4.6".to_string(),
                "sonnet-4-6".to_string(),
                "sonnet46".to_string(),
            ],
            family: "sonnet".to_string(),
            version: Some("4.6".to_string()),
            release_date: None,
        });

        // Claude 4.5 Haiku
        self.register_model(ModelInfo {
            name: "claude-haiku-4-5-20251001".to_string(),
            aliases: vec![
                "haiku-4.5".to_string(),
                "haiku-4-5".to_string(),
                "haiku45".to_string(),
            ],
            family: "haiku".to_string(),
            version: Some("4.5".to_string()),
            release_date: Some("2025-10-01".to_string()),
        });

        for (name, aliases, version) in [
            ("gpt-5-codex", Vec::new(), "5-codex"),
            ("gpt-5.1-codex", Vec::new(), "5.1-codex"),
            ("gpt-5.1-codex-mini", Vec::new(), "5.1-codex-mini"),
            ("gpt-5.2-codex", Vec::new(), "5.2-codex"),
            ("gpt-5.3-codex", Vec::new(), "5.3-codex"),
            ("gpt-5", Vec::new(), "5"),
            (
                "gpt-5.6-sol",
                vec!["gpt-5.6".to_string(), "sol".to_string()],
                "5.6-sol",
            ),
            ("gpt-5.6-terra", vec!["terra".to_string()], "5.6-terra"),
            ("gpt-5.6-luna", vec!["luna".to_string()], "5.6-luna"),
        ] {
            self.register_model(ModelInfo {
                name: name.to_string(),
                aliases,
                family: "openai".to_string(),
                version: Some(version.to_string()),
                release_date: None,
            });
        }
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

        if model_lower == filter_lower {
            return true;
        }

        if self.families.contains_key(&filter_lower)
            && self
                .get_model_family(model_name)
                .is_some_and(|family| family.eq_ignore_ascii_case(filter))
        {
            return true;
        }

        let filter_model = self.models.values().find(|model| {
            model.name.eq_ignore_ascii_case(filter)
                || model
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(filter))
        });
        if let Some(filter_model) = filter_model {
            return self
                .get_model_info(model_name)
                .is_some_and(|model| model.name == filter_model.name);
        }

        // Unknown filters retain forward-compatible partial matching.
        model_lower.contains(&filter_lower)
    }

    pub fn get_model_info(&self, model_name: &str) -> Option<&ModelInfo> {
        // Direct lookup
        if let Some(info) = self.models.get(model_name) {
            return Some(info);
        }

        // Prefer exact aliases before substring fallback. This makes the
        // provider default (`gpt-5.6` -> Sol) explicit.
        if let Some(info) = self.models.values().find(|info| {
            info.aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(model_name))
        }) {
            return Some(info);
        }

        // Prefer the most specific model name so overlapping generations do
        // not depend on HashMap iteration order.
        self.models
            .iter()
            .filter(|(name, _)| model_name.contains(name.as_str()) || name.contains(model_name))
            .max_by(|(left, _), (right, _)| {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            })
            .map(|(_, info)| info)
    }

    pub fn get_model_family(&self, model_name: &str) -> Option<String> {
        // First try exact lookup
        if let Some(info) = self.get_model_info(model_name) {
            return Some(info.family.clone());
        }

        // Heuristic detection for unknown models
        let model_lower = model_name.to_lowercase();
        if model_lower.contains("fable") {
            return Some("fable".to_string());
        } else if model_lower.contains("opus") {
            return Some("opus".to_string());
        } else if model_lower.contains("sonnet") {
            return Some("sonnet".to_string());
        } else if model_lower.contains("haiku") {
            return Some("haiku".to_string());
        } else if model_lower.contains("gpt") || model_lower.contains("codex") {
            return Some("openai".to_string());
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

    #[test]
    fn test_current_model_families_and_aliases() {
        let registry = ModelsRegistry::new();

        assert_eq!(
            registry.get_model_family("claude-fable-5"),
            Some("fable".to_string())
        );
        assert!(registry.matches_filter("claude-fable-5", "fable"));
        assert!(registry.matches_filter("claude-fable-5", "fable5"));
        assert_eq!(
            registry.get_model_family("gpt-5.6-sol"),
            Some("openai".to_string())
        );
        assert_eq!(
            registry
                .get_model_info("gpt-5.6")
                .map(|model| model.name.as_str()),
            Some("gpt-5.6-sol")
        );
        assert!(registry.matches_filter("gpt-5.6-sol", "gpt-5.6"));
        assert!(!registry.matches_filter("gpt-5.6-terra", "gpt-5.6"));
        assert!(registry.matches_filter("gpt-5.2-codex", "openai"));
    }
}
