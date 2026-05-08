use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::ModelConfig;

#[derive(Clone)]
pub struct ModelClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
    done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ModelClient {
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Send a prompt and get a complete response (non-streaming).
    pub async fn generate(&self, system: &str, prompt: &str) -> Result<String> {
        match self.call_ollama_chat(system, prompt).await {
            Ok(response) => Ok(response),
            Err(e) => {
                eprintln!("Ollama chat API failed: {e}, trying generate API...");
                self.call_ollama_generate(system, prompt).await
            }
        }
    }

    /// Send a multi-turn conversation.
    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let ollama_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();

        let body = json!({
            "model": self.model,
            "messages": ollama_messages,
            "stream": false,
        });

        let url = format!("{}/api/chat", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Ollama at {}: {}", self.base_url, e))?;

        if !resp.status().is_success() {
            bail!(
                "Ollama API returned status {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            );
        }

        let chat_resp: OllamaChatResponse = resp.json().await?;
        Ok(chat_resp.message.content)
    }

    async fn call_ollama_chat(&self, system: &str, prompt: &str) -> Result<String> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": prompt }
            ],
            "stream": false,
        });

        let url = format!("{}/api/chat", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            bail!("Ollama chat API error: {}", resp.status());
        }

        let chat_resp: OllamaChatResponse = resp.json().await?;
        Ok(chat_resp.message.content)
    }

    async fn call_ollama_generate(&self, system: &str, prompt: &str) -> Result<String> {
        let full_prompt = format!("System: {system}\n\nUser: {prompt}");

        let body = json!({
            "model": self.model,
            "prompt": full_prompt,
            "stream": false,
        });

        let url = format!("{}/api/generate", self.base_url);
        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            bail!("Ollama generate API error: {}", resp.status());
        }

        let gen_resp: OllamaGenerateResponse = resp.json().await?;
        Ok(gen_resp.response)
    }
}
