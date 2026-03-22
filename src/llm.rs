//! LLM client for OpenAI-compatible APIs.

use serde::{Deserialize, Serialize};
use snafu::ResultExt as _;

use crate::error::{self, Result};

/// A resolved LLM client ready for API calls.
#[derive(Debug, Clone)]
pub struct LlmClient {
    api_key:  String,
    base_url: String,
    model:    String,
    client:   reqwest::Client,
}

/// A chat message in the `OpenAI` chat completion format.
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    /// Role of the message author (e.g. "user", "system").
    pub role:    String,
    /// Content of the message.
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model:           String,
    messages:        Vec<ChatMessage>,
    temperature:     f64,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

impl LlmClient {
    /// Try to create a client from config + env vars. Returns `None` if no API
    /// key is available.
    pub fn from_config(cfg: &crate::app_config::LlmConfig) -> Option<Self> {
        let resolved = cfg.resolve();
        let api_key = resolved.api_key.as_ref()?;
        Some(Self {
            api_key:  api_key.clone(),
            base_url: resolved.base_url.clone(),
            model:    resolved.model.clone(),
            client:   reqwest::Client::new(),
        })
    }

    /// Send a chat completion request and return the response text.
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.3,
            response_format: ResponseFormat {
                r#type: "json_object".to_string(),
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context(error::HttpSnafu)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(crate::error::TenkiError::LlmApi {
                message: format!("HTTP {status}: {body}"),
            });
        }

        let chat_response: ChatResponse = response.json().await.context(error::HttpSnafu)?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or(crate::error::TenkiError::LlmApi {
                message: "empty response from LLM".to_string(),
            })
    }
}
