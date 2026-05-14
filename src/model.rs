use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::ModelConfig;

#[derive(Debug, Clone)]
enum Provider {
    Ollama,
    OpenAi,
    OpenAiCompatible,
}

#[derive(Clone)]
pub struct ModelClient {
    provider: Provider,
    base_url: String,
    model: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

// --- Ollama response types ---

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
}

// --- OpenAI-compatible request/response types ---

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageContent,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageContent {
    content: String,
}

// --- Public ChatMessage (used by TUI) ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ModelClient {
    pub fn new(config: &ModelConfig) -> Result<Self> {
        config.validate()?;

        let provider = match config.provider.as_str() {
            "openai" => Provider::OpenAi,
            "openai_compatible" => Provider::OpenAiCompatible,
            _ => Provider::Ollama,
        };

        let timeout = config.timeout_seconds.unwrap_or(120);

        Ok(Self {
            provider,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout))
                .build()
                .expect("Failed to create HTTP client"),
        })
    }

    pub async fn generate(&self, system: &str, prompt: &str) -> Result<String> {
        match self.provider {
            Provider::Ollama => self.generate_ollama(system, prompt).await,
            Provider::OpenAi | Provider::OpenAiCompatible => {
                self.generate_openai(system, prompt).await
            }
        }
    }

    // --- Ollama ---

    async fn generate_ollama(&self, system: &str, prompt: &str) -> Result<String> {
        match self.call_ollama_chat(system, prompt).await {
            Ok(response) => Ok(response),
            Err(e) => {
                eprintln!("Ollama chat API failed: {e}, trying generate API...");
                self.call_ollama_generate(system, prompt).await
            }
        }
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
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", self.base_url, e))?;

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

    // --- OpenAI-compatible ---

    async fn generate_openai(&self, system: &str, prompt: &str) -> Result<String> {
        let body = OpenAiRequest {
            model: self.model.clone(),
            messages: vec![
                OpenAiMessage {
                    role: "system".into(),
                    content: system.into(),
                },
                OpenAiMessage {
                    role: "user".into(),
                    content: prompt.into(),
                },
            ],
            stream: false,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", self.base_url, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("OpenAI-compatible API error {status}: {body}");
        }

        let openai_resp: OpenAiResponse = resp.json().await?;
        openai_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No choices in API response"))
    }
}
