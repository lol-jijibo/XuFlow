use crate::backends::{openai_compat_chat, ChatParams, LlmBackend, StreamEvent};
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct OpenAICompatBackend {
    model: String,
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAICompatBackend {
    pub fn new(model: String, api_key: String, base_url: String) -> Self {
        Self {
            model,
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmBackend for OpenAICompatBackend {
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
