use std::time::Duration;
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use crate::config::ProviderConfig;
use crate::sanitizer::sanitize_commit_message;

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Option<Vec<ChatChoice>>,
    error: Option<ApiErrorResponse>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: Option<ChoiceMessage>,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    message: Option<String>,
}

pub struct LlmClient {
    client: Client,
    provider: ProviderConfig,
    completions_url: String,
}

impl LlmClient {
    pub fn new(provider: ProviderConfig) -> Result<Self> {
        let completions_url = Self::normalize_url(&provider.endpoint);
        let client = Client::builder()
            .timeout(Duration::from_secs(provider.timeout_seconds))
            .build()
            .context("Failed to initialize HTTP client")?;

        Ok(Self {
            client,
            provider,
            completions_url,
        })
    }

    pub fn generate_commit_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let payload = ChatCompletionRequest {
            model: &self.provider.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: user_prompt,
                },
            ],
            temperature: self.provider.temperature,
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if !self.provider.api_key.is_empty() && self.provider.api_key != "not-needed" {
            let auth_val = format!("Bearer {}", self.provider.api_key);
            if let Ok(hv) = HeaderValue::from_str(&auth_val) {
                headers.insert(AUTHORIZATION, hv);
            }
        }

        let resp = match self
            .client
            .post(&self.completions_url)
            .headers(headers)
            .json(&payload)
            .send()
        {
            Ok(r) => r,
            Err(err) => {
                if err.is_connect() || err.is_timeout() {
                    bail!(
                        "Failed to connect to LLM endpoint at '{}'.\n\
                         -> Is your local LLM service (e.g. LMStudio / Ollama) running?\n\
                         -> Use 'git msg config' to inspect or adjust your endpoint configuration.\n\
                         (Details: {})",
                        self.completions_url,
                        err
                    );
                }
                bail!("HTTP request to LLM failed: {}", err);
            }
        };

        let status = resp.status();
        let body_text = resp.text().unwrap_or_default();

        if !status.is_success() {
            if let Ok(api_err) = serde_json::from_str::<ChatCompletionResponse>(&body_text) {
                if let Some(err_obj) = api_err.error {
                    if let Some(msg) = err_obj.message {
                        bail!("LLM API error ({}): {}", status, msg);
                    }
                }
            }
            bail!("LLM API returned error status {}:\n{}", status, body_text);
        }

        let parsed: ChatCompletionResponse = serde_json::from_str(&body_text)
            .with_context(|| format!("Failed to parse response JSON from LLM: {}", body_text))?;

        let raw_content = parsed
            .choices
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.as_ref())
            .map(|s| s.as_str())
            .unwrap_or_default();

        if raw_content.trim().is_empty() {
            bail!("LLM returned an empty commit message.");
        }

        Ok(sanitize_commit_message(raw_content))
    }

    /// 规范化输入的端点 URL
    pub fn normalize_url(raw: &str) -> String {
        let trimmed = raw.trim().trim_end_matches('/');

        if trimmed.ends_with("/chat/completions") {
            trimmed.to_string()
        } else if trimmed.ends_with("/v1") {
            format!("{}/chat/completions", trimmed)
        } else {
            format!("{}/v1/chat/completions", trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_normalization() {
        assert_eq!(
            LlmClient::normalize_url("http://127.0.0.1:1234"),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
        assert_eq!(
            LlmClient::normalize_url("http://127.0.0.1:1234/"),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
        assert_eq!(
            LlmClient::normalize_url("http://localhost:1234/v1"),
            "http://localhost:1234/v1/chat/completions"
        );
        assert_eq!(
            LlmClient::normalize_url("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }
}
