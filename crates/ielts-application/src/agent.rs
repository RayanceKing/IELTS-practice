use async_trait::async_trait;
use ielts_db::{
    BeginAgentRunCommand, BeginAgentToolCallCommand, FinishAgentRunCommand,
    FinishAgentToolCallCommand,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

use crate::{ApplicationError, ModelError, TokenUsage};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum AgentMessage {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<AgentToolCall>,
    },
    ToolResult {
        tool_call_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelRequest {
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<AgentToolDefinition>,
    pub temperature: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<AgentToolCall>,
    pub model: String,
    pub latency_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
}

#[async_trait]
pub trait AgentModel: Send + Sync {
    async fn respond(&self, request: AgentModelRequest) -> Result<AgentModelResponse, ModelError>;
}

pub use ielts_db::{
    StoredAgentRunStatus as AgentRunStatus, StoredAgentToolStatus as AgentToolStatus,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolExecution {
    pub status: AgentToolStatus,
    pub model_content: String,
    pub audit_result: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApplicationError>,
}

impl AgentToolExecution {
    pub fn succeeded(model_content: impl Into<String>, audit_result: Value) -> Self {
        Self {
            status: AgentToolStatus::Succeeded,
            model_content: model_content.into(),
            audit_result,
            error: None,
        }
    }

    pub fn rejected(
        code: impl Into<String>,
        message: impl Into<String>,
        audit_result: Value,
    ) -> Self {
        Self::error(
            AgentToolStatus::Rejected,
            code,
            message,
            false,
            audit_result,
        )
    }

    pub fn failed(
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
        audit_result: Value,
    ) -> Self {
        Self::error(
            AgentToolStatus::Failed,
            code,
            message,
            retryable,
            audit_result,
        )
    }

    fn error(
        status: AgentToolStatus,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
        audit_result: Value,
    ) -> Self {
        let error = ApplicationError::new(code, message, retryable);
        let model_content = serde_json::to_string(&error).unwrap_or_else(|_| {
            "{\"code\":\"agent.tool_failed\",\"message\":\"tool failed\",\"retryable\":false}"
                .into()
        });
        Self {
            status,
            model_content,
            audit_result,
            error: Some(error),
        }
    }
}

#[async_trait]
pub trait AgentToolExecutor: Send + Sync {
    fn definitions(&self) -> Vec<AgentToolDefinition>;

    fn audit_arguments(&self, call: &AgentToolCall) -> Value;

    async fn execute(&self, call: &AgentToolCall) -> AgentToolExecution;
}

pub trait AgentStore: Send + Sync {
    fn begin_run(&self, run: &BeginAgentRunCommand) -> Result<(), ApplicationError>;

    fn begin_tool_call(&self, call: &BeginAgentToolCallCommand) -> Result<(), ApplicationError>;

    fn finish_tool_call(&self, call: &FinishAgentToolCallCommand) -> Result<(), ApplicationError>;

    fn finish_run(&self, run: &FinishAgentRunCommand) -> Result<(), ApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentLimits {
    pub max_rounds: u32,
    pub max_tool_calls: u32,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_rounds: 8,
            max_tool_calls: 24,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunAgentCommand {
    pub run_id: String,
    pub provider_id: String,
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub temperature: f32,
    pub limits: AgentLimits,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunOutcome {
    pub run_id: String,
    pub content: String,
    pub model: String,
    pub rounds: u32,
    pub tool_calls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
}

pub struct AgentService;

impl AgentService {
    pub async fn run<S, M, T>(
        store: &S,
        model: &M,
        tools: &T,
        command: RunAgentCommand,
    ) -> Result<AgentRunOutcome, ApplicationError>
    where
        S: AgentStore,
        M: AgentModel,
        T: AgentToolExecutor,
    {
        validate_command(&command)?;
        store.begin_run(&BeginAgentRunCommand {
            id: command.run_id.clone(),
            provider_id: command.provider_id.clone(),
            model: command.model.clone(),
        })?;

        let mut messages = vec![
            AgentMessage::System {
                content: command.system_prompt,
            },
            AgentMessage::User {
                content: command.user_prompt,
            },
        ];
        let definitions = tools.definitions();
        let mut tool_call_count = 0_u32;
        let mut usage = None;
        let mut seen_tool_call_ids = HashSet::new();

        for round in 1..=command.limits.max_rounds {
            let response = match model
                .respond(AgentModelRequest {
                    messages: messages.clone(),
                    tools: definitions.clone(),
                    temperature: command.temperature,
                })
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let error = ApplicationError::new(
                        "agent.provider_failed",
                        error.message,
                        error.retryable,
                    );
                    finish_failed_run_best_effort(
                        store,
                        &command.run_id,
                        AgentRunStatus::Failed,
                        round,
                        tool_call_count,
                        error.clone(),
                    );
                    return Err(error);
                }
            };
            usage = merge_usage(usage, response.usage.clone());

            if response.tool_calls.is_empty() {
                let content = response
                    .content
                    .as_deref()
                    .map(str::trim)
                    .filter(|content| !content.is_empty())
                    .ok_or_else(|| {
                        ApplicationError::new(
                            "agent.invalid_response",
                            "agent response has neither content nor tool calls",
                            false,
                        )
                    });
                let content = match content {
                    Ok(content) => content.to_string(),
                    Err(error) => {
                        finish_failed_run_best_effort(
                            store,
                            &command.run_id,
                            AgentRunStatus::Failed,
                            round,
                            tool_call_count,
                            error.clone(),
                        );
                        return Err(error);
                    }
                };
                store.finish_run(&FinishAgentRunCommand {
                    id: command.run_id.clone(),
                    status: AgentRunStatus::Completed,
                    rounds: round,
                    tool_call_count,
                    result: Some(json!({
                        "model": response.model,
                        "hasContent": true,
                    })),
                    error: None,
                })?;
                return Ok(AgentRunOutcome {
                    run_id: command.run_id,
                    content,
                    model: response.model,
                    rounds: round,
                    tool_calls: tool_call_count,
                    usage,
                    provider_request_id: response.provider_request_id,
                });
            }

            messages.push(AgentMessage::Assistant {
                content: response.content,
                tool_calls: response.tool_calls.clone(),
            });
            if let Err((status, error)) = validate_tool_batch(
                &response.tool_calls,
                &seen_tool_call_ids,
                command.limits.max_tool_calls - tool_call_count,
                command.limits.max_tool_calls,
            ) {
                finish_failed_run_best_effort(
                    store,
                    &command.run_id,
                    status,
                    round,
                    tool_call_count,
                    error.clone(),
                );
                return Err(error);
            }

            seen_tool_call_ids.extend(response.tool_calls.iter().map(|call| call.id.clone()));
            for call in response.tool_calls {
                tool_call_count += 1;

                store.begin_tool_call(&BeginAgentToolCallCommand {
                    run_id: command.run_id.clone(),
                    call_id: call.id.clone(),
                    sequence: tool_call_count,
                    round,
                    tool_name: call.name.clone(),
                    arguments: tools.audit_arguments(&call),
                })?;
                let execution = tools.execute(&call).await;
                store.finish_tool_call(&FinishAgentToolCallCommand {
                    run_id: command.run_id.clone(),
                    call_id: call.id.clone(),
                    sequence: tool_call_count,
                    status: execution.status,
                    result: execution.audit_result,
                    error: execution.error.as_ref().map(error_json),
                })?;
                messages.push(AgentMessage::ToolResult {
                    tool_call_id: call.id,
                    content: execution.model_content,
                    is_error: execution.status != AgentToolStatus::Succeeded,
                });
            }
        }

        let error = ApplicationError::new(
            "agent.max_rounds_exceeded",
            format!(
                "agent exceeded the round limit of {}",
                command.limits.max_rounds
            ),
            false,
        );
        finish_failed_run_best_effort(
            store,
            &command.run_id,
            AgentRunStatus::LimitExceeded,
            command.limits.max_rounds,
            tool_call_count,
            error.clone(),
        );
        Err(error)
    }
}

fn validate_tool_batch(
    calls: &[AgentToolCall],
    seen_ids: &HashSet<String>,
    remaining_limit: u32,
    max_tool_calls: u32,
) -> Result<(), (AgentRunStatus, ApplicationError)> {
    if calls.len() > remaining_limit as usize {
        return Err((
            AgentRunStatus::LimitExceeded,
            ApplicationError::new(
                "agent.max_tool_calls_exceeded",
                format!("agent exceeded the tool call limit of {}", max_tool_calls),
                false,
            ),
        ));
    }

    let mut ids = seen_ids.clone();
    if calls.iter().any(|call| {
        call.id.trim().is_empty() || call.name.trim().is_empty() || !ids.insert(call.id.clone())
    }) {
        return Err((
            AgentRunStatus::Failed,
            ApplicationError::new(
                "agent.invalid_response",
                "agent tool calls require non-empty, unique ids and non-empty names",
                false,
            ),
        ));
    }

    Ok(())
}

fn validate_command(command: &RunAgentCommand) -> Result<(), ApplicationError> {
    if command.run_id.trim().is_empty()
        || command.provider_id.trim().is_empty()
        || command.model.trim().is_empty()
        || command.system_prompt.trim().is_empty()
        || command.user_prompt.trim().is_empty()
    {
        return Err(ApplicationError::new(
            "agent.invalid_request",
            "run id, provider, model, system prompt, and user prompt are required",
            false,
        ));
    }
    if command.limits.max_rounds == 0 || command.limits.max_tool_calls == 0 {
        return Err(ApplicationError::new(
            "agent.invalid_request",
            "agent limits must be greater than zero",
            false,
        ));
    }
    Ok(())
}

fn finish_run_best_effort<S: AgentStore>(store: &S, run: &FinishAgentRunCommand) {
    if let Err(error) = store.finish_run(run) {
        tracing::warn!(
            run_id = %run.id,
            status = ?run.status,
            error = %error,
            "agent run audit finish failed"
        );
    }
}

fn finish_failed_run_best_effort<S: AgentStore>(
    store: &S,
    run_id: &str,
    status: AgentRunStatus,
    rounds: u32,
    tool_calls: u32,
    error: ApplicationError,
) {
    finish_run_best_effort(
        store,
        &FinishAgentRunCommand {
            id: run_id.to_string(),
            status,
            rounds,
            tool_call_count: tool_calls,
            result: None,
            error: Some(error_json(&error)),
        },
    );
}

fn error_json(error: &ApplicationError) -> Value {
    serde_json::to_value(error).unwrap_or_else(|_| {
        json!({
            "code": error.code,
            "message": error.message,
            "retryable": error.retryable,
        })
    })
}

fn merge_usage(current: Option<TokenUsage>, next: Option<TokenUsage>) -> Option<TokenUsage> {
    match (current, next) {
        (None, None) => None,
        (Some(usage), None) | (None, Some(usage)) => Some(usage),
        (Some(current), Some(next)) => Some(TokenUsage {
            input_tokens: current.input_tokens.saturating_add(next.input_tokens),
            output_tokens: current.output_tokens.saturating_add(next.output_tokens),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Default)]
    struct StoreState {
        run_started: bool,
        calls_started: Vec<BeginAgentToolCallCommand>,
        calls_finished: Vec<FinishAgentToolCallCommand>,
        run_finished: Option<FinishAgentRunCommand>,
        fail_finish_tool_call_once: bool,
        fail_finish_run_once: bool,
    }

    #[derive(Clone, Default)]
    struct FakeStore(Arc<Mutex<StoreState>>);

    impl AgentStore for FakeStore {
        fn begin_run(&self, _run: &BeginAgentRunCommand) -> Result<(), ApplicationError> {
            self.0.lock().unwrap().run_started = true;
            Ok(())
        }

        fn begin_tool_call(
            &self,
            call: &BeginAgentToolCallCommand,
        ) -> Result<(), ApplicationError> {
            self.0.lock().unwrap().calls_started.push(call.clone());
            Ok(())
        }

        fn finish_tool_call(
            &self,
            call: &FinishAgentToolCallCommand,
        ) -> Result<(), ApplicationError> {
            let mut state = self.0.lock().unwrap();
            if state.fail_finish_tool_call_once {
                state.fail_finish_tool_call_once = false;
                return Err(ApplicationError::new(
                    "agent.persistence_failed",
                    "injected tool audit failure",
                    true,
                ));
            }
            state.calls_finished.push(call.clone());
            Ok(())
        }

        fn finish_run(&self, run: &FinishAgentRunCommand) -> Result<(), ApplicationError> {
            let mut state = self.0.lock().unwrap();
            if state.fail_finish_run_once {
                state.fail_finish_run_once = false;
                return Err(ApplicationError::new(
                    "agent.persistence_failed",
                    "injected run audit failure",
                    true,
                ));
            }
            state.run_finished = Some(run.clone());
            Ok(())
        }
    }

    struct ScriptedModel {
        responses: Mutex<VecDeque<Result<AgentModelResponse, ModelError>>>,
        requests: Mutex<Vec<AgentModelRequest>>,
        store: FakeStore,
    }

    #[async_trait]
    impl AgentModel for ScriptedModel {
        async fn respond(
            &self,
            request: AgentModelRequest,
        ) -> Result<AgentModelResponse, ModelError> {
            assert!(
                self.store.0.try_lock().is_ok(),
                "store lock was held during model I/O"
            );
            self.requests.lock().unwrap().push(request);
            self.responses.lock().unwrap().pop_front().unwrap()
        }
    }

    #[derive(Default)]
    struct FakeTools(Mutex<Vec<String>>);

    #[async_trait]
    impl AgentToolExecutor for FakeTools {
        fn definitions(&self) -> Vec<AgentToolDefinition> {
            vec![AgentToolDefinition {
                name: "read_file".into(),
                description: "Read a UTF-8 file".into(),
                parameters: json!({"type":"object"}),
            }]
        }

        fn audit_arguments(&self, call: &AgentToolCall) -> Value {
            json!({"tool": call.name, "hasArguments": !call.arguments_json.is_empty()})
        }

        async fn execute(&self, call: &AgentToolCall) -> AgentToolExecution {
            self.0.lock().unwrap().push(call.name.clone());
            if call.name == "read_file" {
                AgentToolExecution::succeeded(
                    r#"{"content":"hello","sha256":"abc"}"#,
                    json!({"path":"note.txt","bytes":5,"sha256":"abc"}),
                )
            } else {
                AgentToolExecution::rejected(
                    "agent.unknown_tool",
                    "unknown tool",
                    json!({"known":false}),
                )
            }
        }
    }

    #[tokio::test]
    async fn completes_without_calling_tools() {
        let store = FakeStore::default();
        let model = model(&store, vec![Ok(response(Some("done"), vec![]))]);
        let outcome = AgentService::run(&store, &model, &FakeTools::default(), command())
            .await
            .unwrap();

        assert_eq!(outcome.content, "done");
        assert_eq!(outcome.rounds, 1);
        assert_eq!(outcome.tool_calls, 0);
        assert_eq!(
            store
                .0
                .lock()
                .unwrap()
                .run_finished
                .as_ref()
                .unwrap()
                .status,
            AgentRunStatus::Completed
        );
    }

    #[tokio::test]
    async fn executes_tool_calls_sequentially_and_returns_results_to_model() {
        let store = FakeStore::default();
        let model = model(
            &store,
            vec![
                Ok(response(
                    None,
                    vec![call("call-1", "read_file"), call("call-2", "unknown")],
                )),
                Ok(response(Some("finished"), vec![])),
            ],
        );
        let tools = FakeTools::default();
        let outcome = AgentService::run(&store, &model, &tools, command())
            .await
            .unwrap();

        assert_eq!(outcome.tool_calls, 2);
        assert_eq!(*tools.0.lock().unwrap(), vec!["read_file", "unknown"]);
        let requests = model.requests.lock().unwrap();
        let second = &requests[1].messages;
        assert!(matches!(
            second[3],
            AgentMessage::ToolResult {
                is_error: false,
                ..
            }
        ));
        assert!(matches!(
            second[4],
            AgentMessage::ToolResult { is_error: true, .. }
        ));
        let state = store.0.lock().unwrap();
        assert_eq!(state.calls_started.len(), 2);
        assert_eq!(state.calls_finished[1].status, AgentToolStatus::Rejected);
    }

    #[tokio::test]
    async fn persists_provider_failure() {
        let store = FakeStore::default();
        let model = model(&store, vec![Err(ModelError::new("provider timeout", true))]);
        let error = AgentService::run(&store, &model, &FakeTools::default(), command())
            .await
            .unwrap_err();

        assert_eq!(error.code, "agent.provider_failed");
        let state = store.0.lock().unwrap();
        let finish = state.run_finished.as_ref().unwrap();
        assert_eq!(finish.status, AgentRunStatus::Failed);
        assert_eq!(
            finish.error.as_ref().unwrap()["retryable"].as_bool(),
            Some(true)
        );
    }

    #[tokio::test]
    async fn tool_audit_finish_failure_stops_before_next_model_round() {
        let store = FakeStore::default();
        store.0.lock().unwrap().fail_finish_tool_call_once = true;
        let model = model(
            &store,
            vec![
                Ok(response(None, vec![call("call-1", "read_file")])),
                Ok(response(Some("finished"), vec![])),
            ],
        );
        let tools = FakeTools::default();
        let error = AgentService::run(&store, &model, &tools, command())
            .await
            .unwrap_err();

        assert_eq!(error.code, "agent.persistence_failed");
        assert_eq!(model.requests.lock().unwrap().len(), 1);
        assert_eq!(*tools.0.lock().unwrap(), vec!["read_file"]);
        let state = store.0.lock().unwrap();
        assert!(state.calls_finished.is_empty());
        assert!(state.run_finished.is_none());
    }

    #[tokio::test]
    async fn successful_run_audit_finish_failure_is_returned() {
        let store = FakeStore::default();
        store.0.lock().unwrap().fail_finish_run_once = true;
        let model = model(&store, vec![Ok(response(Some("finished"), vec![]))]);
        let error = AgentService::run(&store, &model, &FakeTools::default(), command())
            .await
            .unwrap_err();

        assert_eq!(error.code, "agent.persistence_failed");
        assert!(store.0.lock().unwrap().run_finished.is_none());
    }

    #[tokio::test]
    async fn failed_run_audit_finish_failure_does_not_mask_provider_error() {
        let store = FakeStore::default();
        store.0.lock().unwrap().fail_finish_run_once = true;
        let model = model(&store, vec![Err(ModelError::new("provider timeout", true))]);
        let error = AgentService::run(&store, &model, &FakeTools::default(), command())
            .await
            .unwrap_err();

        assert_eq!(error.code, "agent.provider_failed");
        assert!(store.0.lock().unwrap().run_finished.is_none());
    }

    #[tokio::test]
    async fn enforces_tool_call_limit() {
        let store = FakeStore::default();
        let model = model(
            &store,
            vec![Ok(response(
                None,
                vec![call("call-1", "read_file"), call("call-2", "read_file")],
            ))],
        );
        let mut command = command();
        command.limits.max_tool_calls = 1;
        let error = AgentService::run(&store, &model, &FakeTools::default(), command)
            .await
            .unwrap_err();

        assert_eq!(error.code, "agent.max_tool_calls_exceeded");
        let state = store.0.lock().unwrap();
        assert!(state.calls_started.is_empty());
        assert!(state.calls_finished.is_empty());
        assert_eq!(
            state.run_finished.as_ref().unwrap().status,
            AgentRunStatus::LimitExceeded
        );
    }

    #[tokio::test]
    async fn enforces_round_limit_and_rejects_empty_final_response() {
        let store = FakeStore::default();
        let round_limited_model = model(
            &store,
            vec![Ok(response(None, vec![call("call-1", "read_file")]))],
        );
        let mut round_limited_command = command();
        round_limited_command.limits.max_rounds = 1;
        let error = AgentService::run(
            &store,
            &round_limited_model,
            &FakeTools::default(),
            round_limited_command,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "agent.max_rounds_exceeded");

        let store = FakeStore::default();
        let model = model(&store, vec![Ok(response(Some("  "), vec![]))]);
        let error = AgentService::run(&store, &model, &FakeTools::default(), command())
            .await
            .unwrap_err();
        assert_eq!(error.code, "agent.invalid_response");
    }

    #[tokio::test]
    async fn rejects_duplicate_tool_call_ids() {
        let store = FakeStore::default();
        let model = model(
            &store,
            vec![Ok(response(
                None,
                vec![call("call-1", "read_file"), call("call-1", "read_file")],
            ))],
        );
        let error = AgentService::run(&store, &model, &FakeTools::default(), command())
            .await
            .unwrap_err();
        assert_eq!(error.code, "agent.invalid_response");
        let state = store.0.lock().unwrap();
        assert!(state.calls_started.is_empty());
        assert!(state.calls_finished.is_empty());
    }

    fn model(
        store: &FakeStore,
        responses: Vec<Result<AgentModelResponse, ModelError>>,
    ) -> ScriptedModel {
        ScriptedModel {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
            store: store.clone(),
        }
    }

    fn response(content: Option<&str>, tool_calls: Vec<AgentToolCall>) -> AgentModelResponse {
        AgentModelResponse {
            content: content.map(str::to_string),
            tool_calls,
            model: "fake-model".into(),
            latency_ms: 1,
            usage: Some(TokenUsage {
                input_tokens: 2,
                output_tokens: 1,
            }),
            provider_request_id: Some("request-1".into()),
        }
    }

    fn call(id: &str, name: &str) -> AgentToolCall {
        AgentToolCall {
            id: id.into(),
            name: name.into(),
            arguments_json: r#"{"path":"note.txt"}"#.into(),
        }
    }

    fn command() -> RunAgentCommand {
        RunAgentCommand {
            run_id: "run-1".into(),
            provider_id: "openai-compatible".into(),
            model: "fake-model".into(),
            system_prompt: "Use tools when needed.".into(),
            user_prompt: "Read note.txt".into(),
            temperature: 0.1,
            limits: AgentLimits::default(),
        }
    }
}
