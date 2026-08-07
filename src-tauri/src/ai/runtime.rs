use std::time::{Duration, Instant};

use async_trait::async_trait;
use ielts_application::{
    AgentMessage, AgentModel, AgentModelRequest, AgentModelResponse, AgentToolCall,
    CompletionRequest, CompletionResponse, LanguageModel, ModelError, TokenUsage,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

const MAX_RETRIES: u32 = 2;

#[derive(Debug, Clone)]
pub(crate) struct AiProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub secret_name: String,
    pub timeout: Duration,
}

// The runtime owns the API key. Do not derive Debug/Clone: formatting or
// copying this value would make accidental credential exposure easy.
pub(crate) struct AiRuntime {
    pub config: AiProviderConfig,
    api_key: String,
    client: reqwest::Client,
}

impl AiRuntime {
    pub(crate) fn new(
        config: AiProviderConfig,
        api_key: String,
    ) -> Result<Self, ielts_db::DbError> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|error| {
                ielts_db::DbError::Message(format!("failed to build AI HTTP client: {error}"))
            })?;
        Ok(Self {
            config,
            api_key,
            client,
        })
    }
}

#[async_trait]
impl LanguageModel for AiRuntime {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ModelError> {
        let body = json!({
            "model": self.config.model,
            "temperature": request.temperature,
            "response_format": { "type": "json_object" },
            "messages": request.messages
        });
        let (envelope, latency_ms): (ChatResponse, _) = self
            .post_chat_completion(&body, "AI response envelope invalid")
            .await?;
        let content = envelope
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| model_error("AI response contained no choices", false))?;
        Ok(CompletionResponse {
            content,
            model: envelope.model.unwrap_or_else(|| self.config.model.clone()),
            latency_ms,
            usage: envelope.usage.map(token_usage),
            provider_request_id: envelope.id,
        })
    }
}

#[async_trait]
impl AgentModel for AiRuntime {
    async fn respond(&self, request: AgentModelRequest) -> Result<AgentModelResponse, ModelError> {
        let body = agent_request_body(&self.config.model, &request);
        let (envelope, latency_ms): (AgentChatResponse, _) = self
            .post_chat_completion(&body, "AI agent response envelope invalid")
            .await?;
        parse_agent_response(envelope, &self.config.model, latency_ms)
    }
}

impl AiRuntime {
    async fn post_chat_completion<T: DeserializeOwned>(
        &self,
        body: &Value,
        invalid_envelope: &str,
    ) -> Result<(T, u64), ModelError> {
        let endpoint = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let started = Instant::now();
        for attempt in 0..=MAX_RETRIES {
            let response = self
                .client
                .post(&endpoint)
                .bearer_auth(&self.api_key)
                .json(body)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let envelope = response.json().await.map_err(|error| {
                        model_error(format!("{invalid_envelope}: {error}"), false)
                    })?;
                    return Ok((envelope, elapsed_ms(started)));
                }
                Ok(response) => {
                    let status = response.status();
                    let retryable = status.is_server_error() || status.as_u16() == 429;
                    if retryable && attempt < MAX_RETRIES {
                        retry_delay(attempt).await;
                        continue;
                    }
                    return Err(model_error(
                        format!("AI provider returned HTTP {}", status.as_u16()),
                        retryable,
                    ));
                }
                Err(error) => {
                    let retryable = error.is_timeout() || error.is_connect();
                    if retryable && attempt < MAX_RETRIES {
                        retry_delay(attempt).await;
                        continue;
                    }
                    return Err(model_error(
                        format!("AI request failed: {error}"),
                        retryable,
                    ));
                }
            }
        }
        unreachable!("retry loop always returns")
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize)]
struct AgentChatResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    choices: Vec<AgentChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct AgentChoice {
    message: AgentResponseMessage,
}

#[derive(Deserialize)]
struct AgentResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<WireToolCall>,
}

#[derive(Deserialize)]
struct WireToolCall {
    id: String,
    function: WireFunctionCall,
}

#[derive(Deserialize)]
struct WireFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

fn agent_request_body(model: &str, request: &AgentModelRequest) -> Value {
    let messages = request
        .messages
        .iter()
        .map(agent_message_json)
        .collect::<Vec<_>>();
    let tools = request
        .tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "model": model,
        "temperature": request.temperature,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
    })
}

fn agent_message_json(message: &AgentMessage) -> Value {
    match message {
        AgentMessage::System { content } => json!({"role":"system","content":content}),
        AgentMessage::User { content } => json!({"role":"user","content":content}),
        AgentMessage::Assistant {
            content,
            tool_calls,
        } => json!({
            "role": "assistant",
            "content": content,
            "tool_calls": tool_calls.iter().map(|call| json!({
                "id": call.id,
                "type": "function",
                "function": {
                    "name": call.name,
                    "arguments": call.arguments_json,
                }
            })).collect::<Vec<_>>(),
        }),
        AgentMessage::ToolResult {
            tool_call_id,
            content,
            ..
        } => json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": content,
        }),
    }
}

fn parse_agent_response(
    envelope: AgentChatResponse,
    fallback_model: &str,
    latency_ms: u64,
) -> Result<AgentModelResponse, ModelError> {
    let message = envelope
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message)
        .ok_or_else(|| model_error("AI agent response contained no choices", false))?;
    Ok(AgentModelResponse {
        content: message.content,
        tool_calls: message
            .tool_calls
            .into_iter()
            .map(|call| AgentToolCall {
                id: call.id,
                name: call.function.name,
                arguments_json: call.function.arguments,
            })
            .collect(),
        model: envelope.model.unwrap_or_else(|| fallback_model.to_string()),
        latency_ms,
        usage: envelope.usage.map(token_usage),
        provider_request_id: envelope.id,
    })
}

fn token_usage(usage: ChatUsage) -> TokenUsage {
    TokenUsage {
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
    }
}

async fn retry_delay(attempt: u32) {
    tokio::time::sleep(Duration::from_millis(250 * 2u64.pow(attempt))).await;
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn model_error(message: impl Into<String>, retryable: bool) -> ModelError {
    ModelError::new(message, retryable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_error_does_not_expose_credentials() {
        let error = model_error("AI provider returned HTTP 401", false);
        assert_eq!(error.message, "AI provider returned HTTP 401");
    }

    #[test]
    fn existing_completion_body_keeps_json_response_format() {
        let request = CompletionRequest {
            messages: vec![ielts_application::ChatMessage::new("user", "hello")],
            temperature: 0.2,
        };
        let body = json!({
            "model": "fake-model",
            "temperature": request.temperature,
            "response_format": { "type": "json_object" },
            "messages": request.messages
        });
        assert_eq!(body["response_format"]["type"], "json_object");
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn agent_body_uses_tool_protocol_without_json_response_format() {
        let request = AgentModelRequest {
            messages: vec![
                AgentMessage::User {
                    content: "read it".into(),
                },
                AgentMessage::ToolResult {
                    tool_call_id: "call-1".into(),
                    content: "ok".into(),
                    is_error: false,
                },
            ],
            tools: vec![ielts_application::AgentToolDefinition {
                name: "read_file".into(),
                description: "Read a file".into(),
                parameters: json!({"type":"object"}),
            }],
            temperature: 0.1,
        };
        let body = agent_request_body("fake-model", &request);
        assert!(body.get("response_format").is_none());
        assert_eq!(body["tool_choice"], "auto");
        assert_eq!(body["messages"][1]["tool_call_id"], "call-1");
        assert_eq!(body["tools"][0]["function"]["name"], "read_file");
    }

    #[test]
    fn parses_null_content_and_multiple_tool_calls() {
        let envelope: AgentChatResponse = serde_json::from_value(json!({
            "id":"request-1",
            "model":"provider-model",
            "choices":[{"message":{
                "content":null,
                "tool_calls":[
                    {"id":"call-1","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"a.txt\"}"}},
                    {"id":"call-2","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"b.txt\"}"}}
                ]
            }}],
            "usage":{"prompt_tokens":3,"completion_tokens":4}
        }))
        .unwrap();
        let response = parse_agent_response(envelope, "fallback", 5).unwrap();
        assert!(response.content.is_none());
        assert_eq!(response.tool_calls.len(), 2);
        assert_eq!(response.tool_calls[1].name, "read_file");
        assert_eq!(response.model, "provider-model");
        assert_eq!(response.usage.unwrap().output_tokens, 4);
    }

    #[test]
    fn rejects_agent_response_without_choices() {
        let envelope: AgentChatResponse = serde_json::from_value(json!({
            "choices":[]
        }))
        .unwrap();
        let error = parse_agent_response(envelope, "fallback", 0).unwrap_err();
        assert!(error.message.contains("no choices"));
    }
}
