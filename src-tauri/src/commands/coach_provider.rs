use ielts_db::{CoachMessage, ProviderError};
use serde_json::{json, Value};

use crate::commands::ai::{provider_error, AiRuntime};

pub async fn answer(
    runtime: &AiRuntime,
    history: &[CoachMessage],
    question_context: Option<&Value>,
) -> Result<(String, Value), ProviderError> {
    let mut messages = vec![json!({
        "role": "system",
        "content": "You are an IELTS reading coach. Explain reasoning from the supplied question context, do not invent passage facts, and never claim to change scores. Return JSON only as {\"answer\":\"...\"}."
    })];
    if let Some(context) = question_context {
        messages.push(json!({
            "role": "system",
            "content": format!("Current IELTS question context:\n{}", context)
        }));
    }
    for message in history.iter().filter(|message| {
        message.status == "completed" && matches!(message.role.as_str(), "user" | "assistant")
    }) {
        messages.push(json!({ "role": message.role, "content": message.content }));
    }

    let raw = runtime.chat_completion(Value::Array(messages), 0.2).await?;
    let payload: Value = serde_json::from_str(&raw)
        .map_err(|error| provider_error(format!("coach response JSON invalid: {error}"), false))?;
    let answer = payload
        .get("answer")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|answer| !answer.is_empty())
        .ok_or_else(|| provider_error("coach response missing non-empty answer".into(), false))?;
    Ok((answer.to_string(), payload))
}
