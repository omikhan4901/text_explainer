//! A streaming client for llama-server's OpenAI-compatible chat endpoint (also works with
//! other local OpenAI-compatible servers such as Ollama and LM Studio).

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::time::Duration;

use super::EngineError;
use crate::prompt::Message;

/// Sampling settings for one request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sampling {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

impl Default for Sampling {
    fn default() -> Self {
        // Low temperature: rewrites should be faithful, not creative.
        Sampling {
            temperature: 0.3,
            top_p: 0.9,
            max_tokens: 512,
        }
    }
}

/// Where requests go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub base_url: String,
    pub api_key: Option<String>,
    /// Sent as `model` (endpoints need it; llama-server ignores it).
    pub model: Option<String>,
    /// llama-server understands `grammar`, `cache_prompt` and template arguments.
    pub llama_server: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Timings {
    pub prompt_n: Option<u64>,
    pub prompt_ms: Option<f64>,
    pub predicted_n: Option<u64>,
    pub predicted_ms: Option<f64>,
    pub predicted_per_second: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Completion {
    pub text: String,
    pub finish_reason: Option<String>,
    pub timings: Option<Timings>,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        crate::tls_init();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            // Local only: never route through a system proxy.
            .no_proxy()
            .build()
            .expect("HTTP client");
        Client { http }
    }

    fn request(
        &self,
        method: reqwest::Method,
        target: &Target,
        path: &str,
    ) -> reqwest::RequestBuilder {
        let url = format!("{}{}", target.base_url.trim_end_matches('/'), path);
        let mut rb = self.http.request(method, url);
        if let Some(key) = &target.api_key {
            rb = rb.bearer_auth(key);
        }
        rb
    }

    /// `Ok(true)` when the model is loaded, `Ok(false)` while it is still loading.
    pub async fn health(&self, target: &Target) -> Result<bool, EngineError> {
        let resp = self
            .request(reqwest::Method::GET, target, "/health")
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .map_err(EngineError::from_reqwest)?;
        match resp.status().as_u16() {
            200 => Ok(true),
            503 => Ok(false),
            s => Err(EngineError::Http {
                status: s,
                body: resp.text().await.unwrap_or_default(),
            }),
        }
    }

    /// Streams a chat completion, calling `on_delta` with each piece of text.
    pub async fn chat(
        &self,
        target: &Target,
        messages: &[Message],
        sampling: &Sampling,
        grammar: Option<&str>,
        mut on_delta: impl FnMut(&str),
    ) -> Result<Completion, EngineError> {
        let mut body = Map::new();
        body.insert("messages".into(), json!(messages));
        body.insert("stream".into(), json!(true));
        body.insert("temperature".into(), json!(sampling.temperature));
        body.insert("top_p".into(), json!(sampling.top_p));
        body.insert("max_tokens".into(), json!(sampling.max_tokens));
        if let Some(model) = &target.model {
            body.insert("model".into(), json!(model));
        }
        if target.llama_server {
            body.insert("cache_prompt".into(), json!(true));
            // Thinking models answer directly: we want the rewrite, not a monologue.
            body.insert(
                "chat_template_kwargs".into(),
                json!({ "enable_thinking": false }),
            );
            if let Some(g) = grammar {
                body.insert("grammar".into(), json!(g));
            }
        }

        let resp = self
            .request(reqwest::Method::POST, target, "/v1/chat/completions")
            .json(&Value::Object(body))
            .send()
            .await
            .map_err(EngineError::from_reqwest)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(EngineError::Http {
                status: status.as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }

        let mut completion = Completion::default();
        let mut buf: Vec<u8> = Vec::new();
        let mut stream = resp.bytes_stream();
        'outer: while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(EngineError::from_reqwest)?;
            buf.extend_from_slice(&chunk);
            while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=nl).collect();
                let line = String::from_utf8_lossy(&line);
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    break 'outer;
                }
                let event: StreamEvent = match serde_json::from_str(data) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                if let Some(err) = event.error {
                    return Err(EngineError::Model(err.to_string()));
                }
                if let Some(choice) = event.choices.first() {
                    if let Some(text) = choice.delta.as_ref().and_then(|d| d.content.as_deref())
                        && !text.is_empty()
                    {
                        completion.text.push_str(text);
                        on_delta(text);
                    }
                    if choice.finish_reason.is_some() {
                        completion.finish_reason.clone_from(&choice.finish_reason);
                    }
                }
                if event.timings.is_some() {
                    completion.timings = event.timings;
                }
            }
        }
        Ok(completion)
    }
}

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    timings: Option<Timings>,
    error: Option<Value>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Option<Delta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}
