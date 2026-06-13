use crate::models::ModelCatalog;
use anyhow::{anyhow, Result};

pub fn sync_providers(catalog: &ModelCatalog) -> Result<()> {
    sync_openrouter(catalog)?;
    Ok(())
}

pub fn sync_openrouter(catalog: &ModelCatalog) -> Result<()> {
    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            return Err(anyhow!(
                "OpenRouter sync skipped: missing OPENROUTER_API_KEY"
            ));
        }
    };

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .map_err(|e| anyhow!("OpenRouter request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        return Err(anyhow!(
            "OpenRouter API error ({}): sync failed. Your API key may be invalid or expired.",
            status
        ));
    }

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| anyhow!("Failed to parse OpenRouter response: {}", e))?;

    let models_added = process_models(catalog, &data)?;
    println!("Synced {} models from OpenRouter.", models_added);

    Ok(())
}

fn process_models(catalog: &ModelCatalog, data: &serde_json::Value) -> Result<usize> {
    let mut count = 0;

    catalog.update_provider_sync_time("openrouter")?;

    if let Some(models) = data.get("data").and_then(|d| d.as_array()) {
        for model in models {
            let id = model.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = model.get("name").and_then(|v| v.as_str()).unwrap_or(id);

            let parts: Vec<&str> = id.splitn(2, '/').collect();
            let provider_id = if parts.len() == 2 {
                parts[0]
            } else {
                "openrouter"
            };
            let model_id = if parts.len() == 2 { parts[1] } else { id };

            let context_window = model
                .get("context_length_tokens")
                .or_else(|| model.get("context_window"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let max_output = model
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let input_price = model
                .get("pricing")
                .and_then(|p| p.get("prompt"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|p| p * 1_000_000.0);

            let output_price = model
                .get("pricing")
                .and_then(|p| p.get("completion"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|p| p * 1_000_000.0);

            let supports_vision = id.contains("vision")
                || id.contains("claude")
                || id.contains("gpt")
                || id.contains("gemini");

            let mut attrs: Vec<(&str, String)> = Vec::new();

            if let Some(v) = context_window {
                attrs.push(("context_window", v.to_string()));
            }
            if let Some(v) = max_output {
                attrs.push(("max_output", v.to_string()));
            }
            if let Some(v) = input_price {
                attrs.push(("input_price", v.to_string()));
            }
            if let Some(v) = output_price {
                attrs.push(("output_price", v.to_string()));
            }
            attrs.push(("supports_vision", supports_vision.to_string()));

            catalog.upsert_model(provider_id, model_id, name, &attrs)?;
            count += 1;
        }
    }

    Ok(count)
}
