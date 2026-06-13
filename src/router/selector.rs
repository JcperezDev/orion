use crate::models::{ModelCatalog, ModelInfo};
use crate::providers::ProviderRegistry;
use std::sync::Arc;

pub struct ModelSelector {
    catalog: Arc<ModelCatalog>,
    registry: Arc<ProviderRegistry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskKind {
    Coding,
    Vision,
    Reasoning,
    Cheap,
    Fast,
    Local,
    Tools,
    General,
}

impl TaskKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "coding" | "code" | "program" | "debug" => Some(TaskKind::Coding),
            "vision" | "visual" | "image" => Some(TaskKind::Vision),
            "reasoning" | "think" | "complex" => Some(TaskKind::Reasoning),
            "cheap" | "cost" | "affordable" => Some(TaskKind::Cheap),
            "fast" | "quick" | "simple" => Some(TaskKind::Fast),
            "local" | "offline" | "free" => Some(TaskKind::Local),
            "tools" | "tool" | "function" => Some(TaskKind::Tools),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelConstraints {
    pub require_tools: bool,
    pub require_vision: bool,
    pub prefer_local: bool,
    pub max_input_price: Option<f64>,
    pub max_output_price: Option<f64>,
    pub min_context_window: Option<u32>,
    pub exclude_provider: Option<String>,
}

impl Default for ModelConstraints {
    fn default() -> Self {
        Self {
            require_tools: false,
            require_vision: false,
            prefer_local: false,
            max_input_price: None,
            max_output_price: None,
            min_context_window: None,
            exclude_provider: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelRecommendation {
    pub model: ModelInfo,
    pub score: f64,
    pub reason: String,
    pub fallback_count: usize,
    pub heuristic_score: bool,
}

impl ModelRecommendation {
    pub fn new(model: ModelInfo, reason: &str) -> Self {
        Self {
            score: 0.0,
            model,
            reason: reason.to_string(),
            fallback_count: 0,
            heuristic_score: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub recommendation: Option<ModelRecommendation>,
    pub fallbacks: Vec<ModelRecommendation>,
    pub missing_requirements: Vec<String>,
}

impl Default for SelectionResult {
    fn default() -> Self {
        Self {
            recommendation: None,
            fallbacks: Vec::new(),
            missing_requirements: Vec::new(),
        }
    }
}

impl ModelSelector {
    pub fn new(catalog: Arc<ModelCatalog>, registry: Arc<ProviderRegistry>) -> Self {
        Self { catalog, registry }
    }

    pub fn select_best_model(
        &self,
        task: TaskKind,
        constraints: ModelConstraints,
    ) -> SelectionResult {
        let mut models = self.catalog.list_models(None);
        let mut missing_requirements = Vec::new();

        if models.is_empty() {
            missing_requirements
                .push("No models in catalog. Run `/providers sync` first.".to_string());
            return SelectionResult {
                recommendation: None,
                fallbacks: Vec::new(),
                missing_requirements,
            };
        }

        match task {
            TaskKind::Vision => {
                models.retain(|m| m.supports_vision);
                if models.is_empty() {
                    missing_requirements.push("No vision-capable models found".to_string());
                }
            }
            TaskKind::Tools => {
                models.retain(|m| m.supports_tools);
                if models.is_empty() {
                    missing_requirements.push("No tool-calling models found".to_string());
                }
            }
            TaskKind::Local => {
                models.retain(|m| m.is_local || m.provider_id == "ollama");
                if models.is_empty() {
                    missing_requirements
                        .push("No local models found. Ollama may not be running.".to_string());
                }
            }
            TaskKind::Reasoning => {
                models.retain(|m| m.supports_reasoning);
                if models.is_empty() {
                    missing_requirements.push("No reasoning models found".to_string());
                }
            }
            TaskKind::Coding => {
                models.retain(|m| m.supports_tools || m.supports_reasoning);
            }
            _ => {}
        }

        if let Some(exclude) = &constraints.exclude_provider {
            models.retain(|m| &m.provider_id != exclude);
        }

        if constraints.require_vision && task != TaskKind::Vision {
            let before = models.len();
            models.retain(|m| m.supports_vision);
            if models.is_empty() && before > 0 {
                missing_requirements.push("No models with required vision capability".to_string());
            }
        }

        if constraints.require_tools && task != TaskKind::Tools {
            let before = models.len();
            models.retain(|m| m.supports_tools);
            if models.is_empty() && before > 0 {
                missing_requirements.push("No models with required tool calling".to_string());
            }
        }

        if constraints.prefer_local {
            let local: Vec<_> = models.iter().filter(|m| m.is_local).cloned().collect();
            if !local.is_empty() {
                models = local;
            }
        }

        if let Some(min_ctx) = constraints.min_context_window {
            models.retain(|m| m.context_window.unwrap_or(0) >= min_ctx);
        }

        if let Some(max_price) = constraints.max_input_price {
            models.retain(|m| m.input_price.map(|p| p <= max_price).unwrap_or(true));
        }

        let available: Vec<_> = models
            .into_iter()
            .filter(|m| self.registry.is_available(&m.provider_id) && m.is_available)
            .collect();

        if available.is_empty() {
            missing_requirements.push("No available providers for selected models".to_string());
        }

        let mut scored: Vec<(ModelInfo, f64, String)> = available
            .iter()
            .map(|m| {
                let (score, reason) = self.score_model(m, task);
                (m.clone(), score, reason)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let fallbacks: Vec<ModelRecommendation> = scored
            .iter()
            .skip(1)
            .take(3)
            .map(|(m, s, r)| {
                let mut rec = ModelRecommendation::new(m.clone(), r);
                rec.score = *s;
                rec.heuristic_score = m.rank_overall.is_none();
                rec
            })
            .collect();

        let recommendation = scored.first().map(|(m, s, r)| {
            let mut rec = ModelRecommendation::new(m.clone(), r);
            rec.score = *s;
            rec.heuristic_score = m.rank_overall.is_none();
            rec.fallback_count = fallbacks.len();
            rec
        });

        SelectionResult {
            recommendation,
            fallbacks,
            missing_requirements,
        }
    }

    fn score_model(&self, model: &ModelInfo, task: TaskKind) -> (f64, String) {
        let has_rank = model.rank_overall.is_some();

        let base_score = if has_rank {
            let rank = model.rank_overall.unwrap_or(999) as f64;
            100.0 - rank.min(100.0)
        } else {
            50.0
        };

        let mut bonus = 0.0;
        let mut reason_parts = Vec::new();

        match task {
            TaskKind::Coding => {
                if model.supports_tools {
                    bonus += 20.0;
                    reason_parts.push("tools".to_string());
                }
                if model.supports_reasoning {
                    bonus += 15.0;
                    reason_parts.push("reasoning".to_string());
                }
                if let Some(ctx) = model.context_window {
                    if ctx >= 100_000 {
                        bonus += 10.0;
                        reason_parts.push(format!("{}K ctx", ctx / 1000));
                    }
                }
            }
            TaskKind::Vision => {
                if model.supports_vision {
                    bonus += 30.0;
                }
            }
            TaskKind::Reasoning => {
                if model.supports_reasoning {
                    bonus += 25.0;
                }
            }
            TaskKind::Cheap => {
                if let Some(price) = model.input_price {
                    bonus += (10.0 - price.min(10.0)).max(0.0);
                    reason_parts.push(format!("${:.2}/M", price));
                } else {
                    reason_parts.push("price unknown".to_string());
                }
            }
            TaskKind::Fast => {
                if let Some(price) = model.input_price {
                    bonus += (10.0 - price.min(10.0)).max(0.0);
                    reason_parts.push(format!("${:.2}/M", price));
                }
                if let Some(ctx) = model.context_window {
                    if ctx >= 32_000 {
                        bonus += 5.0;
                    }
                }
            }
            TaskKind::Local => {
                if model.is_local || model.provider_id == "ollama" {
                    bonus += 50.0;
                    reason_parts.push("local".to_string());
                }
            }
            TaskKind::Tools => {
                if model.supports_tools {
                    bonus += 30.0;
                    reason_parts.push("tools".to_string());
                }
            }
            TaskKind::General => {
                if let Some(rank) = model.rank_overall {
                    let rank_f = rank as f64;
                    bonus += (50.0 - rank_f.min(50.0)) * 0.5;
                }
            }
        }

        if let Some(price) = model.input_price {
            if price < 1.0 {
                bonus += 5.0;
            }
        }

        let total_score = base_score + bonus;
        let reason = if reason_parts.is_empty() {
            if has_rank {
                format!("rank #{}", model.rank_overall.unwrap_or(0))
            } else {
                "heuristic score".to_string()
            }
        } else {
            reason_parts.join(", ")
        };

        let score_type = if has_rank { "rank" } else { "heuristic" };
        let full_reason = format!("{} ({}: {:.1})", reason, score_type, total_score);

        (total_score, full_reason)
    }

    pub fn best_for_task(&self, task: &str) -> Option<ModelInfo> {
        let task_kind = TaskKind::from_str(task)?;
        let result = self.select_best_model(task_kind, ModelConstraints::default());
        result.recommendation.map(|r| r.model)
    }
}
