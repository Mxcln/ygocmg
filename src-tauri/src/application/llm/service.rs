use serde_json::Value;

use crate::application::config::service::ConfigService;
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};

const DEEPSEEK_CHAT_URL: &str = "https://api.deepseek.com/chat/completions";

/// Thin transport for DeepSeek chat completions.
///
/// The frontend builds the full OpenAI-compatible request body (model, messages,
/// tools, thinking, stream:false). This service only injects the API key (read
/// from `GlobalConfig`) and forwards the request, returning the raw response JSON.
/// It does not parse or interpret any business fields.
pub struct LlmService<'a> {
    state: &'a AppState,
}

impl<'a> LlmService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn chat(&self, body: Value) -> AppResult<Value> {
        let api_key = self.api_key()?;

        let client = reqwest::Client::new();
        let response = client
            .post(DEEPSEEK_CHAT_URL)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                AppError::new("llm.http_error", "failed to reach DeepSeek API")
                    .with_detail("source", err.to_string())
            })?;

        let status = response.status();
        let payload: Value = response.json().await.map_err(|err| {
            AppError::new("llm.invalid_response", "failed to decode DeepSeek response")
                .with_detail("status", status.as_u16())
                .with_detail("source", err.to_string())
        })?;

        if !status.is_success() {
            return Err(
                AppError::new("llm.http_error", "DeepSeek API returned an error")
                    .with_detail("status", status.as_u16())
                    .with_detail("body", payload),
            );
        }

        Ok(payload)
    }

    fn api_key(&self) -> AppResult<String> {
        let config = ConfigService::new(self.state).load()?;
        config
            .deepseek_api_key
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                AppError::new(
                    "llm.no_api_key",
                    "DeepSeek API key is not configured; set it in settings",
                )
            })
    }
}
