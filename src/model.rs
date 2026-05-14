use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::ModelConfig;

const MAX_RETRIES: u32 = 3;
const BASE_BACKOFF_MS: u64 = 2000;

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

    /// Check if an HTTP status is retryable (rate limit or server error).
    fn is_retryable_status(status: reqwest::StatusCode) -> bool {
        status == 429 || status.is_server_error()
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

        for attempt in 0..MAX_RETRIES {
            let resp = self.client.post(&url).json(&body).send().await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let chat_resp: OllamaChatResponse = r.json().await?;
                    return Ok(chat_resp.message.content);
                }
                Ok(r) if Self::is_retryable_status(r.status()) => {
                    let status = r.status();
                    let wait = BASE_BACKOFF_MS * 2u64.pow(attempt);
                    eprintln!("  Retry {}/{}: HTTP {status}, waiting {wait}ms...", attempt + 1, MAX_RETRIES);
                    tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                    continue;
                }
                Ok(r) => {
                    bail!("Ollama chat API error: {}", r.status());
                }
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        let wait = BASE_BACKOFF_MS * 2u64.pow(attempt);
                        eprintln!("  Retry {}/{}: connection error, waiting {wait}ms...", attempt + 1, MAX_RETRIES);
                        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!("Failed to connect to {}: {}", self.base_url, e));
                }
            }
        }
        bail!("Max retries exceeded for Ollama chat API")
    }

    async fn call_ollama_generate(&self, system: &str, prompt: &str) -> Result<String> {
        let full_prompt = format!("System: {system}\n\nUser: {prompt}");

        let body = json!({
            "model": self.model,
            "prompt": full_prompt,
            "stream": false,
        });

        let url = format!("{}/api/generate", self.base_url);

        for attempt in 0..MAX_RETRIES {
            let resp = self.client.post(&url).json(&body).send().await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let gen_resp: OllamaGenerateResponse = r.json().await?;
                    return Ok(gen_resp.response);
                }
                Ok(r) if Self::is_retryable_status(r.status()) => {
                    let status = r.status();
                    let wait = BASE_BACKOFF_MS * 2u64.pow(attempt);
                    eprintln!("  Retry {}/{}: HTTP {status}, waiting {wait}ms...", attempt + 1, MAX_RETRIES);
                    tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                    continue;
                }
                Ok(r) => {
                    bail!("Ollama generate API error: {}", r.status());
                }
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        let wait = BASE_BACKOFF_MS * 2u64.pow(attempt);
                        eprintln!("  Retry {}/{}: connection error, waiting {wait}ms...", attempt + 1, MAX_RETRIES);
                        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                        continue;
                    }
                    return Err(e).map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", self.base_url, e));
                }
            }
        }
        bail!("Max retries exceeded for Ollama generate API")
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

        for attempt in 0..MAX_RETRIES {
            let mut req = self.client.post(&url).json(&body);
            if let Some(key) = &self.api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }

            let resp = req.send().await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let openai_resp: OpenAiResponse = r.json().await?;
                    return openai_resp
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .ok_or_else(|| anyhow::anyhow!("No choices in API response"));
                }
                Ok(r) if Self::is_retryable_status(r.status()) => {
                    let status = r.status();
                    let wait = BASE_BACKOFF_MS * 2u64.pow(attempt);
                    eprintln!("  Retry {}/{}: HTTP {status}, waiting {wait}ms...", attempt + 1, MAX_RETRIES);
                    tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                    continue;
                }
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    bail!("OpenAI-compatible API error {status}: {body}");
                }
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        let wait = BASE_BACKOFF_MS * 2u64.pow(attempt);
                        eprintln!("  Retry {}/{}: connection error, waiting {wait}ms...", attempt + 1, MAX_RETRIES);
                        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!("Failed to connect to {}: {}", self.base_url, e));
                }
            }
        }
        bail!("Max retries exceeded for OpenAI-compatible API")
    }
}
