// Configuration models: Provider, Model, API Key
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model_name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_model: String,
    pub models: Vec<ModelConfig>,
    pub working_directory: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_model: String::new(),
            models: Vec::new(),
            working_directory: None,
        }
    }
}
