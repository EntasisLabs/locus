//! HTTP client for a System 1 server that speaks `POST /v1/systemone`.
//!
//! That is the wire Laya's `laya-serve`, the Rust `sys1` server, and
//! Jev-compatible hosts share. This client does not load a checkpoint.

use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde_json::Value;

use crate::domain::system1::{System1Decider, System1Request, System1Response};

/// Posts [`System1Request`] values to a remote System 1 endpoint.
#[derive(Debug, Clone)]
pub struct HttpSystem1 {
    client: reqwest::Client,
    endpoint: String,
    model: Option<String>,
    api_key: Option<String>,
}

impl HttpSystem1 {
    /// `endpoint` may be a base URL or a full `/v1/systemone` or `/v1/decide` URL.
    pub fn new(endpoint: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            endpoint: join_system1_endpoint(&endpoint.into()),
            model: None,
            api_key: None,
        }
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// Pin a checkpoint such as `typed-decisions`. Unset lets the server route.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        self.api_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        };
        self
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// JSON body this client will POST. Useful when a host wants to log the forward pass.
    pub fn request_json(&self, request: &System1Request) -> Value {
        let mut body = serde_json::to_value(request).unwrap_or_else(|_| serde_json::json!({}));
        if request.model.is_none() {
            if let Some(model) = &self.model {
                body["model"] = Value::String(model.clone());
            }
        }
        body
    }
}

/// Append `/v1/systemone` unless the URL already names a System 1 route.
pub fn join_system1_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1/systemone") || trimmed.ends_with("/v1/decide") {
        trimmed.to_string()
    } else if trimmed.is_empty() {
        "/v1/systemone".to_string()
    } else {
        format!("{trimmed}/v1/systemone")
    }
}

#[async_trait]
impl System1Decider for HttpSystem1 {
    fn decider_id(&self) -> &str {
        "http-system1"
    }

    async fn predict(&self, request: &System1Request) -> Result<System1Response> {
        let mut call = self
            .client
            .post(&self.endpoint)
            .json(&self.request_json(request));
        if let Some(api_key) = &self.api_key {
            call = call.bearer_auth(api_key);
        }
        let response = call.send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!(
                "system 1 endpoint returned {status}: {}",
                truncate(&body, 300)
            );
        }
        let value: Value = serde_json::from_str(&body)?;
        let mut parsed = System1Response::parse_wire(&request.questions, &value)?;
        parsed.decider_id = self.decider_id().to_string();
        if parsed.checkpoint.is_none() {
            parsed.checkpoint = self.model.clone();
        }
        Ok(parsed)
    }
}

fn truncate(text: &str, max: usize) -> String {
    let mut out = text.chars().take(max).collect::<String>();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{HttpSystem1, join_system1_endpoint};
    use crate::domain::system1::{DecisionQuestion, System1Request};
    use std::collections::BTreeMap;

    #[test]
    fn endpoint_join_keeps_existing_system1_routes() {
        assert_eq!(
            join_system1_endpoint("http://127.0.0.1:8000"),
            "http://127.0.0.1:8000/v1/systemone"
        );
        assert_eq!(
            join_system1_endpoint("http://127.0.0.1:8000/"),
            "http://127.0.0.1:8000/v1/systemone"
        );
        assert_eq!(
            join_system1_endpoint("http://127.0.0.1:8000/v1/systemone"),
            "http://127.0.0.1:8000/v1/systemone"
        );
        assert_eq!(
            join_system1_endpoint("http://127.0.0.1:3000/v1/decide/"),
            "http://127.0.0.1:3000/v1/decide"
        );
    }

    #[test]
    fn request_json_pins_the_configured_model() {
        let decider = HttpSystem1::new("http://127.0.0.1:8000").with_model("typed-decisions");
        let request = System1Request {
            state: serde_json::json!({"text": "remember that I prefer tea"}),
            questions: BTreeMap::from([(
                "should_persist".to_string(),
                DecisionQuestion::Noul {
                    instructions: "store this?".to_string(),
                },
            )]),
            model: None,
        };
        let body = decider.request_json(&request);
        assert_eq!(body["model"], "typed-decisions");
        assert_eq!(body["questions"]["should_persist"]["type"], "noul");
        assert_eq!(decider.endpoint(), "http://127.0.0.1:8000/v1/systemone");
    }
}
