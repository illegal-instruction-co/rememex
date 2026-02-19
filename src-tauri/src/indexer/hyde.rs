use anyhow::{anyhow, Result};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HydeConfig {
    #[serde(default)]
    pub enabled: bool,
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

const SYSTEM_PROMPT: &str = "\
You are a code search assistant. Given a search query, generate a hypothetical code snippet \
or document passage that would be a perfect search result for this query. \
Write ONLY the code/text, no explanations. Keep it under 200 words. \
Match the language if the query implies one.";

pub async fn generate_hypothetical_document(
    config: &HydeConfig,
    query: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let request = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: query.to_string(),
            },
        ],
        max_tokens: 300,
        temperature: 0.3,
    };

    let mut req = client.post(&config.endpoint).json(&request);

    if let Some(ref key) = config.api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }

    let response = req
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| anyhow!("HyDE LLM request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("HyDE LLM returned {}: {}", status, body));
    }

    let resp: ChatResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("HyDE: failed to parse LLM response: {}", e))?;

    let content = resp
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    if content.trim().is_empty() {
        return Err(anyhow!("HyDE: LLM returned empty response"));
    }

    debug!("HyDE generated {} chars for query: {}", content.len(), query);
    Ok(content)
}

pub async fn maybe_generate(
    config: Option<&HydeConfig>,
    query: &str,
    use_hyde: bool,
) -> Option<String> {
    let config = config?;
    if !config.enabled || !use_hyde {
        return None;
    }

    match generate_hypothetical_document(config, query).await {
        Ok(doc) => Some(doc),
        Err(e) => {
            warn!("HyDE fallback to normal query: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, header_exists};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn mock_chat_response(content: &str) -> serde_json::Value {
        serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": content }
            }]
        })
    }

    #[tokio::test]
    async fn test_generate_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_chat_response("fn search(query: &str) -> Vec<Result> { todo!() }"))
            )
            .expect(1)
            .mount(&server)
            .await;

        let config = HydeConfig {
            enabled: true,
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            model: "test-model".into(),
            api_key: None,
        };

        let result = generate_hypothetical_document(&config, "how does search work").await;
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.contains("search"), "generated doc should contain query-related content");
    }

    #[tokio::test]
    async fn test_generate_api_error_500() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .expect(1)
            .mount(&server)
            .await;

        let config = HydeConfig {
            enabled: true,
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            model: "test-model".into(),
            api_key: None,
        };

        let result = generate_hypothetical_document(&config, "test query").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("500"), "error should mention status code: {}", err);
    }

    #[tokio::test]
    async fn test_generate_empty_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_chat_response("   "))
            )
            .expect(1)
            .mount(&server)
            .await;

        let config = HydeConfig {
            enabled: true,
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            model: "test-model".into(),
            api_key: None,
        };

        let result = generate_hypothetical_document(&config, "test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_generate_malformed_json() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("{not valid json")
            )
            .expect(1)
            .mount(&server)
            .await;

        let config = HydeConfig {
            enabled: true,
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            model: "test-model".into(),
            api_key: None,
        };

        let result = generate_hypothetical_document(&config, "test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parse"));
    }

    #[tokio::test]
    async fn test_generate_sends_auth_header() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header_exists("Authorization"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_chat_response("authenticated response"))
            )
            .expect(1)
            .mount(&server)
            .await;

        let config = HydeConfig {
            enabled: true,
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            model: "test-model".into(),
            api_key: Some("sk-test-key-123".into()),
        };

        let result = generate_hypothetical_document(&config, "test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_maybe_generate_no_config() {
        let result = maybe_generate(None, "test query", true).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_maybe_generate_disabled() {
        let config = HydeConfig {
            enabled: false,
            endpoint: "http://localhost:1/nope".into(),
            model: "test".into(),
            api_key: None,
        };
        let result = maybe_generate(Some(&config), "test query", true).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_maybe_generate_use_hyde_false() {
        let config = HydeConfig {
            enabled: true,
            endpoint: "http://localhost:1/nope".into(),
            model: "test".into(),
            api_key: None,
        };
        let result = maybe_generate(Some(&config), "test query", false).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_maybe_generate_end_to_end() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_chat_response("pub fn indexer() { /* hypothetical */ }"))
            )
            .mount(&server)
            .await;

        let config = HydeConfig {
            enabled: true,
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            model: "gpt-4".into(),
            api_key: Some("sk-key".into()),
        };

        let result = maybe_generate(Some(&config), "how does indexing work", true).await;
        assert!(result.is_some());
        assert!(result.unwrap().contains("indexer"));
    }

    #[tokio::test]
    async fn test_maybe_generate_network_error_returns_none() {
        let config = HydeConfig {
            enabled: true,
            endpoint: "http://127.0.0.1:1/v1/chat/completions".into(),
            model: "test".into(),
            api_key: None,
        };
        let result = maybe_generate(Some(&config), "test", true).await;
        assert!(result.is_none(), "network error should gracefully return None");
    }
}


