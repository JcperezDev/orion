use std::sync::Arc;
use crate::models::{ModelCatalog, ModelInfo};
use crate::providers::ProviderRegistry;

pub struct ModelSelector {
    catalog: Arc<ModelCatalog>,
    registry: Arc<ProviderRegistry>,
}

#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    pub task: Option<String>,
    pub coding: bool,
    pub vision: bool,
    pub reasoning: bool,
    pub fast: bool,
    pub max_cost: Option<f64>,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            task: None,
            coding: false,
            vision: false,
            reasoning: false,
            fast: false,
            max_cost: None,
        }
    }
}

impl ModelSelector {
    pub fn new(catalog: Arc<ModelCatalog>, registry: Arc<ProviderRegistry>) -> Self {
        Self { catalog, registry }
    }

    pub fn select(&self, criteria: SelectionCriteria) -> Option<ModelInfo> {
        let mut models = self.catalog.list_models(None);

        if criteria.coding {
            models.retain(|m| m.rank_coding.unwrap_or(999) > 0);
            models.sort_by_key(|m| m.rank_coding.unwrap_or(999));
        } else if criteria.vision {
            models.retain(|m| m.rank_vision.unwrap_or(999) > 0);
            models.sort_by_key(|m| m.rank_vision.unwrap_or(999));
        } else if criteria.reasoning {
            models.retain(|m| m.supports_reasoning);
            models.sort_by_key(|m| m.rank_overall.unwrap_or(999));
        } else if criteria.fast {
            models.sort_by(|a, b| {
                let a_price = a.input_price.unwrap_or(f64::MAX);
                let b_price = b.input_price.unwrap_or(f64::MAX);
                a_price.partial_cmp(&b_price).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            models.retain(|m| m.rank_overall.unwrap_or(999) > 0);
            models.sort_by_key(|m| m.rank_overall.unwrap_or(999));
        }

        for model in models {
            if !self.registry.is_available(&model.provider_id) {
                continue;
            }

            if let Some(max_cost) = criteria.max_cost {
                if model.input_price.map(|p| p > max_cost).unwrap_or(false) {
                    continue;
                }
            }

            return Some(model);
        }

        None
    }

    pub fn best_for_task(&self, task: &str) -> Option<ModelInfo> {
        let criteria = if task.contains("code") || task.contains("programming") || task.contains("debug") {
            SelectionCriteria { coding: true, ..Default::default() }
        } else if task.contains("image") || task.contains("vision") || task.contains("visual") {
            SelectionCriteria { vision: true, ..Default::default() }
        } else if task.contains("think") || task.contains("reason") || task.contains("complex") {
            SelectionCriteria { reasoning: true, ..Default::default() }
        } else if task.contains("fast") || task.contains("quick") || task.contains("simple") || task.contains("cheap") {
            SelectionCriteria { fast: true, ..Default::default() }
        } else {
            SelectionCriteria::default()
        };

        self.select(criteria)
    }
}
