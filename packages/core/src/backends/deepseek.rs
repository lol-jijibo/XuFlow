use crate::backends::{openai_compat_chat, ChatParams, LlmBackend, StreamEvent};
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct DeepSeekBackend {
    model: String,
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl DeepSeekBackend {
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            model,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.deepseek.com".into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmBackend for DeepSeekBackend {
    fn model(&self) -> &str {
        &self.model
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    async fn chat(&self, params: ChatParams, tx: mpsc::Sender<StreamEvent>) -> Result<crate::backends::Usage, anyhow::Error> {
        openai_compat_chat(&self.client, &self.base_url, &self.api_key, &self.model, params, &tx).await
    }
}
