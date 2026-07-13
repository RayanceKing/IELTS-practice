use ielts_domain::dto::{WritingFeedbackV4, WritingScoreV4};
use serde_json::json;

use crate::commands::ai::{provider_error, AiRuntime};
use ielts_db::{PreparedEvaluation, ProviderError};

#[derive(Debug)]
pub struct ProviderOutput {
    pub score: WritingScoreV4,
    /// Review parsing is independent from scoring so a bad review cannot erase
    /// a completed score checkpoint.
    pub feedback: Result<WritingFeedbackV4, ProviderError>,
}

pub async fn evaluate(
    runtime: &AiRuntime,
    prepared: &PreparedEvaluation,
) -> Result<ProviderOutput, ProviderError> {
    let task_type = prepared
        .task_type
        .map(|value| format!("{value:?}"))
        .unwrap_or_default();
    let prompt = format!(
        "Assess this IELTS writing response. Task type: {task_type}. Task prompt: {}\n\nEssay:\n{}",
        prepared.prompt.as_deref().unwrap_or("not provided"),
        prepared.essay
    );
    // Product path: Settings prompt bank + model temperature, resolved in prepare_evaluation.
    let system = prepared.system_prompt.as_str();
    let temperature = prepared.temperature;
    let content = runtime
        .chat_completion(
            json!([
                { "role": "system", "content": system },
                { "role": "user", "content": prompt }
            ]),
            temperature,
        )
        .await?;
    parse_output(&content)
}

fn parse_output(content: &str) -> Result<ProviderOutput, ProviderError> {
    let trimmed = content.trim();
    let json = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .strip_suffix("```")
        .unwrap_or(trimmed)
        .trim();
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| provider_error(format!("AI evaluation JSON invalid: {error}"), false))?;
    let score: WritingScoreV4 = serde_json::from_value(
        value
            .get("score")
            .cloned()
            .ok_or_else(|| provider_error("AI evaluation score is missing".into(), false))?,
    )
    .map_err(|error| provider_error(format!("AI evaluation score invalid: {error}"), false))?;
    validate_score(&score)?;
    let feedback = value
        .get("feedback")
        .cloned()
        .ok_or_else(|| provider_error("AI evaluation feedback is missing".into(), true))
        .and_then(|feedback| {
            serde_json::from_value(feedback).map_err(|error| {
                provider_error(format!("AI evaluation feedback invalid: {error}"), true)
            })
        });
    Ok(ProviderOutput { score, feedback })
}

fn validate_score(score: &WritingScoreV4) -> Result<(), ProviderError> {
    let values = [
        score.overall,
        score.task_response,
        score.coherence,
        score.lexical,
        score.grammar,
    ];
    if values
        .iter()
        .all(|value| value.is_finite() && (0.0..=9.0).contains(value))
    {
        Ok(())
    } else {
        Err(provider_error(
            "AI evaluation contains an invalid band score".into(),
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_json() {
        let output = parse_output("```json\n{\"score\":{\"overall\":7.0,\"taskResponse\":7.0,\"coherence\":6.5,\"lexical\":7.0,\"grammar\":6.5},\"feedback\":{\"overall\":\"Clear\",\"plan\":[],\"paragraphs\":[],\"sentences\":[],\"rewrites\":[]}}\n```").unwrap();
        assert_eq!(output.score.overall, 7.0);
        assert!(output.feedback.is_ok());
    }

    #[test]
    fn rejects_invalid_scores_and_malformed_json() {
        assert!(parse_output("not-json").is_err());
        assert!(parse_output("{\"score\":{\"overall\":10.0,\"taskResponse\":7.0,\"coherence\":7.0,\"lexical\":7.0,\"grammar\":7.0},\"feedback\":{\"plan\":[],\"paragraphs\":[],\"sentences\":[],\"rewrites\":[]}}").is_err());
    }

    #[test]
    fn keeps_valid_score_when_review_payload_is_invalid() {
        let output = parse_output("{\"score\":{\"overall\":7.0,\"taskResponse\":7.0,\"coherence\":6.5,\"lexical\":7.0,\"grammar\":6.5},\"feedback\":false}").unwrap();
        assert_eq!(output.score.overall, 7.0);
        assert!(output.feedback.is_err());
    }
}
