use ielts_db::{
    AppendCoachMessageCommand, CoachRunResult, RecordCoachFailureCommand, RunCoachCommand,
};
use serde_json::{json, Value};

use crate::{
    ApplicationError, ChatMessage, CoachStore, CompletionRequest, LanguageModel, ModelError,
};

pub struct CoachService;

impl CoachService {
    pub async fn run<S, M, F>(
        store: &S,
        command: RunCoachCommand,
        load_model: F,
    ) -> Result<CoachRunResult, ApplicationError>
    where
        S: CoachStore,
        M: LanguageModel,
        F: FnOnce() -> Result<M, ApplicationError>,
    {
        if command.content.trim().is_empty() {
            return Err(ApplicationError::new(
                "enrichment.error",
                "content required",
                false,
            ));
        }
        let user_message = store.append_message(&AppendCoachMessageCommand {
            thread_id: command.thread_id.clone(),
            role: "user".into(),
            content: command.content.clone(),
            structured_payload: command.question_context.clone(),
            status: "completed".into(),
        })?;
        let history = store.load_history(&command.thread_id, 100)?;
        let model = match load_model() {
            Ok(model) => model,
            Err(error) => {
                record_failure(store, &command.thread_id, &error);
                return Err(error);
            }
        };

        let request = build_request(&history, command.question_context.as_ref());
        let provider_result = model
            .complete(request)
            .await
            .map_err(provider_failure)
            .and_then(|response| parse_answer(&response.content));

        match provider_result {
            Ok((answer, payload)) => {
                let assistant_message =
                    store.complete_run(&command.thread_id, &answer, Some(payload))?;
                Ok(CoachRunResult {
                    user_message,
                    assistant_message,
                })
            }
            Err(error) => {
                record_failure(store, &command.thread_id, &error);
                Err(error)
            }
        }
    }
}

fn build_request(
    history: &[ielts_db::CoachMessage],
    question_context: Option<&Value>,
) -> CompletionRequest {
    let mut messages = vec![ChatMessage::new(
        "system",
        "You are an IELTS reading coach. Explain reasoning from the supplied question context, do not invent passage facts, and never claim to change scores. Return JSON only as {\"answer\":\"...\"}.",
    )];
    if let Some(context) = question_context {
        messages.push(ChatMessage::new(
            "system",
            format!("Current IELTS question context:\n{context}"),
        ));
    }
    for message in history.iter().filter(|message| {
        message.status == "completed" && matches!(message.role.as_str(), "user" | "assistant")
    }) {
        messages.push(ChatMessage::new(
            message.role.clone(),
            message.content.clone(),
        ));
    }
    CompletionRequest {
        messages,
        temperature: 0.2,
    }
}

fn parse_answer(raw: &str) -> Result<(String, Value), ApplicationError> {
    let payload: Value = serde_json::from_str(raw).map_err(|error| {
        ApplicationError::new(
            "coach.provider_failed",
            format!("coach response JSON invalid: {error}"),
            false,
        )
    })?;
    let answer = payload
        .get("answer")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|answer| !answer.is_empty())
        .ok_or_else(|| {
            ApplicationError::new(
                "coach.provider_failed",
                "coach response missing non-empty answer",
                false,
            )
        })?;
    Ok((answer.to_string(), payload))
}

fn provider_failure(error: ModelError) -> ApplicationError {
    ApplicationError::new("coach.provider_failed", error.message, error.retryable)
}

fn record_failure<S: CoachStore>(store: &S, thread_id: &str, error: &ApplicationError) {
    let _ = store.record_failure(&RecordCoachFailureCommand {
        thread_id: thread_id.to_string(),
        error: serde_json::to_value(error).unwrap_or_else(|_| {
            json!({
                "code": error.code,
                "message": error.message,
                "retryable": error.retryable,
            })
        }),
        preserve_scores: true,
    });
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use ielts_db::{AppendCoachMessageCommand, CoachMessage, RecordCoachFailureCommand};

    use super::*;

    #[derive(Default)]
    struct CoachState {
        messages: Vec<CoachMessage>,
        failures: Vec<RecordCoachFailureCommand>,
    }

    #[derive(Clone, Default)]
    struct FakeCoachStore {
        state: Arc<Mutex<CoachState>>,
    }

    impl CoachStore for FakeCoachStore {
        fn append_message(
            &self,
            command: &AppendCoachMessageCommand,
        ) -> Result<CoachMessage, ApplicationError> {
            let mut state = self.state.lock().unwrap();
            let message = message(
                &command.thread_id,
                &command.role,
                &command.content,
                state.messages.len() as u32 + 1,
                &command.status,
                command.structured_payload.clone(),
            );
            state.messages.push(message.clone());
            Ok(message)
        }

        fn load_history(
            &self,
            _thread_id: &str,
            _limit: u32,
        ) -> Result<Vec<CoachMessage>, ApplicationError> {
            Ok(self.state.lock().unwrap().messages.clone())
        }

        fn complete_run(
            &self,
            thread_id: &str,
            content: &str,
            payload: Option<Value>,
        ) -> Result<CoachMessage, ApplicationError> {
            let mut state = self.state.lock().unwrap();
            let message = message(
                thread_id,
                "assistant",
                content,
                state.messages.len() as u32 + 1,
                "completed",
                payload,
            );
            state.messages.push(message.clone());
            Ok(message)
        }

        fn record_failure(
            &self,
            command: &RecordCoachFailureCommand,
        ) -> Result<(), ApplicationError> {
            self.state.lock().unwrap().failures.push(command.clone());
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FakeModel {
        response: Result<String, ModelError>,
        store_state: Arc<Mutex<CoachState>>,
        request: Arc<Mutex<Option<CompletionRequest>>>,
    }

    #[async_trait]
    impl LanguageModel for FakeModel {
        async fn complete(
            &self,
            request: CompletionRequest,
        ) -> Result<crate::CompletionResponse, ModelError> {
            assert!(
                self.store_state.try_lock().is_ok(),
                "store lock was held during model I/O"
            );
            *self.request.lock().unwrap() = Some(request);
            self.response
                .clone()
                .map(|content| crate::CompletionResponse {
                    content,
                    model: "fake-model".into(),
                    latency_ms: 1,
                    usage: None,
                    provider_request_id: None,
                })
        }
    }

    #[tokio::test]
    async fn persists_user_and_assistant_messages_and_builds_context() {
        let store = FakeCoachStore::default();
        let request = Arc::new(Mutex::new(None));
        let model = FakeModel {
            response: Ok(r#"{"answer":"Check the qualifier.","kind":"hint"}"#.into()),
            store_state: store.state.clone(),
            request: request.clone(),
        };

        let result = CoachService::run(&store, command("Why?"), || Ok(model))
            .await
            .unwrap();

        assert_eq!(result.user_message.role, "user");
        assert_eq!(result.assistant_message.content, "Check the qualifier.");
        let request = request.lock().unwrap().clone().unwrap();
        assert!(request
            .messages
            .iter()
            .any(|message| message.content.contains("question-1")));
        assert_eq!(request.messages.last().unwrap().content, "Why?");
        assert!(store.state.lock().unwrap().failures.is_empty());
    }

    #[tokio::test]
    async fn rejects_empty_question_before_loading_model() {
        let store = FakeCoachStore::default();
        let error = CoachService::run::<_, FakeModel, _>(&store, command("  "), || {
            panic!("model loader must not run")
        })
        .await
        .unwrap_err();

        assert_eq!(error.code, "enrichment.error");
        assert!(store.state.lock().unwrap().messages.is_empty());
    }

    #[tokio::test]
    async fn records_invalid_and_provider_failures_without_score_mutation_permission() {
        for response in [
            Ok("not-json".to_string()),
            Ok(r#"{"answer":""}"#.to_string()),
            Err(ModelError::new("provider timeout", true)),
        ] {
            let store = FakeCoachStore::default();
            let model = FakeModel {
                response,
                store_state: store.state.clone(),
                request: Arc::new(Mutex::new(None)),
            };
            let error = CoachService::run(&store, command("Why?"), || Ok(model))
                .await
                .unwrap_err();
            assert_eq!(error.code, "coach.provider_failed");
            let state = store.state.lock().unwrap();
            assert_eq!(state.failures.len(), 1);
            assert!(state.failures[0].preserve_scores);
            assert_eq!(state.messages.len(), 1);
        }
    }

    #[tokio::test]
    async fn records_runtime_load_failure_after_persisting_user_message() {
        let store = FakeCoachStore::default();
        let error = CoachService::run::<_, FakeModel, _>(&store, command("Why?"), || {
            Err(ApplicationError::new(
                "enrichment.error",
                "AI is not configured",
                false,
            ))
        })
        .await
        .unwrap_err();

        assert_eq!(error.code, "enrichment.error");
        let state = store.state.lock().unwrap();
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.failures.len(), 1);
        assert!(state.failures[0].preserve_scores);
    }

    #[test]
    fn rejects_empty_and_non_json_answers() {
        assert!(parse_answer("not-json").is_err());
        assert!(parse_answer(r#"{"answer":""}"#).is_err());
    }

    #[test]
    fn parses_valid_answer_without_losing_payload() {
        let (answer, payload) =
            parse_answer(r#"{"answer":"Check the qualifier.","kind":"hint"}"#).unwrap();
        assert_eq!(answer, "Check the qualifier.");
        assert_eq!(payload["kind"], "hint");
    }

    fn command(content: &str) -> RunCoachCommand {
        RunCoachCommand {
            thread_id: "thread-1".into(),
            content: content.into(),
            question_context: Some(json!({ "questionId": "question-1" })),
        }
    }

    fn message(
        thread_id: &str,
        role: &str,
        content: &str,
        sequence: u32,
        status: &str,
        structured_payload: Option<Value>,
    ) -> CoachMessage {
        CoachMessage {
            id: format!("message-{sequence}"),
            thread_id: thread_id.into(),
            role: role.into(),
            content: content.trim().into(),
            structured_payload,
            status: status.into(),
            sequence,
            created_at: "2026-08-07T00:00:00Z".into(),
        }
    }
}
