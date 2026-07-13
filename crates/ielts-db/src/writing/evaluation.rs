//! Persisted writing evaluation state machine (Phase 5).
//!
//! Stages: Preparing → Scoring → Reviewing → Finalizing
//! Checkpoints after each stage. Cancel aborts executor only; inputs remain.
//! Retries create lineage via `retry_of` without losing prior evaluation rows.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use ielts_domain::domain::{AttemptStatus, EvaluationStage, EvaluationStatus, WritingTaskType};
use ielts_domain::dto::{
    EvaluationDegradation, WritingEvaluationV4, WritingFeedbackV4, WritingScoreV4,
};
use ielts_domain::ErrorEnvelope;

use crate::sqlite::{DbError, DbResult};
use crate::writing::draft::get_writing_draft;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationEvent {
    pub evaluation_id: String,
    pub sequence: u32,
    pub revision: u32,
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<EvaluationStage>,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSession {
    pub id: String,
    pub attempt_id: String,
    pub evaluation_id: String,
    pub status: EvaluationStatus,
    pub stage: EvaluationStage,
    pub revision: u32,
    pub sequence: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of: Option<String>,
    pub cancel_requested: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub started_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartEvaluationCommand {
    pub attempt_id: String,
    pub idempotency_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationRunResult {
    pub session: EvaluationSession,
    pub evaluation: WritingEvaluationV4,
    pub events: Vec<EvaluationEvent>,
}

#[derive(Debug, Clone)]
pub struct PreparedEvaluation {
    pub evaluation_id: String,
    pub session_id: String,
    pub essay: String,
    pub prompt: Option<String>,
    pub task_type: Option<WritingTaskType>,
    pub existing: Option<EvaluationRunResult>,
}

/// Provider abstraction: production plugs real HTTP/AI; tests use Fake/Deterministic.
pub trait WritingProvider: Send + Sync {
    fn id(&self) -> &str;
    fn model(&self) -> &str;
    fn score(
        &self,
        essay: &str,
        prompt: Option<&str>,
        task_type: Option<WritingTaskType>,
    ) -> Result<WritingScoreV4, ProviderError>;
    fn review(
        &self,
        essay: &str,
        score: &WritingScoreV4,
    ) -> Result<WritingFeedbackV4, ProviderError>;
}

#[derive(Debug, Clone)]
pub struct ProviderError {
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Deterministic offline provider for tests and degraded local runs.
#[derive(Debug, Default)]
pub struct DeterministicProvider;

impl WritingProvider for DeterministicProvider {
    fn id(&self) -> &str {
        "deterministic"
    }
    fn model(&self) -> &str {
        "local-v1"
    }
    fn score(
        &self,
        essay: &str,
        _prompt: Option<&str>,
        _task_type: Option<WritingTaskType>,
    ) -> Result<WritingScoreV4, ProviderError> {
        let words = essay.split_whitespace().count().max(1) as f64;
        // Stable pseudo-band from word count (5.0–8.0)
        let overall = ((words / 50.0).clamp(0.0, 1.0) * 3.0 + 5.0).min(8.0);
        let overall = (overall * 2.0).round() / 2.0;
        Ok(WritingScoreV4 {
            overall,
            task_response: overall,
            coherence: (overall - 0.5).max(5.0),
            lexical: overall,
            grammar: (overall - 0.5).max(5.0),
        })
    }
    fn review(
        &self,
        essay: &str,
        score: &WritingScoreV4,
    ) -> Result<WritingFeedbackV4, ProviderError> {
        let first = essay.lines().next().unwrap_or("").trim();
        Ok(WritingFeedbackV4 {
            overall: Some(format!(
                "Deterministic review: overall band {:.1}. Focus on development and precision.",
                score.overall
            )),
            plan: vec![
                "Strengthen topic sentences".into(),
                "Add one concrete example per body paragraph".into(),
            ],
            paragraphs: vec![],
            sentences: if first.is_empty() {
                vec![]
            } else {
                vec![ielts_domain::dto::SentenceFeedback {
                    sentence: first.to_string(),
                    correction: None,
                    kind: Some("observation".into()),
                }]
            },
            rewrites: vec![],
        })
    }
}

/// Orchestrator with priority, failure counts, cooldown (minimal Phase 5).
#[derive(Debug, Default)]
pub struct ProviderOrchestrator {
    pub failure_count: u32,
    pub cooldown_until: Option<i64>,
}

impl ProviderOrchestrator {
    pub fn select<'a>(
        &self,
        providers: &'a [&'a dyn WritingProvider],
    ) -> Option<&'a dyn WritingProvider> {
        if let Some(until) = self.cooldown_until {
            if chrono::Utc::now().timestamp() < until {
                return None;
            }
        }
        providers.first().copied()
    }

    pub fn record_failure(&mut self) {
        self.failure_count = self.failure_count.saturating_add(1);
        if self.failure_count >= 3 {
            self.cooldown_until = Some(chrono::Utc::now().timestamp() + 30);
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.cooldown_until = None;
    }
}

pub fn start_evaluation(
    conn: &Connection,
    cmd: &StartEvaluationCommand,
    provider: &dyn WritingProvider,
) -> DbResult<EvaluationRunResult> {
    let prepared = prepare_evaluation(conn, cmd, provider.id(), provider.model())?;
    if let Some(existing) = prepared.existing {
        return Ok(existing);
    }
    let score = provider.score(
        &prepared.essay,
        prepared.prompt.as_deref(),
        prepared.task_type,
    );
    let (feedback, review_error) = match score.as_ref() {
        Ok(score) => match provider.review(&prepared.essay, score) {
            Ok(feedback) => (Some(feedback), None),
            Err(error) => (None, Some(error)),
        },
        Err(_) => (None, None),
    };
    finish_evaluation(conn, &prepared, score, feedback, review_error)
}

/// Creates the persisted session and returns an owned provider request.
/// The caller must release its database lock before performing network I/O.
pub fn prepare_evaluation(
    conn: &Connection,
    cmd: &StartEvaluationCommand,
    provider_id: &str,
    model: &str,
) -> DbResult<PreparedEvaluation> {
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }

    // Idempotent start
    let existing_eval: Result<Option<String>, rusqlite::Error> = conn.query_row(
        "SELECT evaluation_id FROM attempt_idempotency
         WHERE scope = 'writing.evaluate' AND idempotency_key = ?1",
        params![cmd.idempotency_key],
        |r| r.get(0),
    );
    if let Ok(Some(eval_id)) = existing_eval {
        if let Some(session) = load_session_by_evaluation(conn, &eval_id)? {
            let evaluation = load_evaluation_result(conn, &eval_id)?.unwrap_or_else(|| {
                empty_eval(EvaluationStatus::Running, EvaluationStage::Preparing)
            });
            let events = list_events(conn, &eval_id, 0)?;
            return Ok(PreparedEvaluation {
                evaluation_id: eval_id,
                session_id: session.id.clone(),
                essay: String::new(),
                prompt: None,
                task_type: None,
                existing: Some(EvaluationRunResult {
                    session,
                    evaluation,
                    events,
                }),
            });
        }
    }

    let draft = get_writing_draft(conn, &cmd.attempt_id)?
        .ok_or_else(|| DbError::Validation("draft required before evaluation".into()))?;
    if draft.content_text.trim().is_empty() {
        return Err(DbError::Validation("empty essay".into()));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let evaluation_id = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();
    let revision = 1u32;

    // Create evaluation row first, then session — transactionally related.
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "INSERT INTO writing_evaluations (
            id, attempt_id, status, stage, provider_id, model, rubric_version, prompt_version,
            result_json, degradation_json, error_json, started_at, completed_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL, ?9, NULL, ?9)",
        params![
            evaluation_id,
            cmd.attempt_id,
            status_str(EvaluationStatus::Queued),
            stage_str(EvaluationStage::Preparing),
            provider_id,
            model,
            "rubric-v1",
            "prompt-v1",
            now,
        ],
    )?;

    if let Some(retry_of) = &cmd.retry_of {
        let root = resolve_root_evaluation(&tx, retry_of)?.unwrap_or_else(|| retry_of.clone());
        tx.execute(
            "INSERT INTO evaluation_lineage (evaluation_id, attempt_id, retry_of, root_evaluation_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![evaluation_id, cmd.attempt_id, retry_of, root, now],
        )?;
    }

    tx.execute(
        "INSERT INTO evaluation_sessions (
            id, attempt_id, evaluation_id, status, stage, revision, sequence, retry_of,
            cancel_requested, provider_id, model, started_at, updated_at, completed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 0, ?8, ?9, ?10, ?10, NULL)",
        params![
            session_id,
            cmd.attempt_id,
            evaluation_id,
            status_str(EvaluationStatus::Queued),
            stage_str(EvaluationStage::Preparing),
            revision as i64,
            cmd.retry_of,
            provider_id,
            model,
            now,
        ],
    )?;

    // Mark attempt reviewing
    tx.execute(
        "UPDATE attempts SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![
            status_attempt(AttemptStatus::Reviewing),
            now,
            cmd.attempt_id
        ],
    )?;

    tx.execute(
        "INSERT INTO attempt_idempotency (scope, idempotency_key, attempt_id, evaluation_id, response_json, created_at)
         VALUES ('writing.evaluate', ?1, ?2, ?3, ?4, ?5)",
        params![
            cmd.idempotency_key,
            cmd.attempt_id,
            evaluation_id,
            json!({ "evaluationId": evaluation_id, "sessionId": session_id }).to_string(),
            now
        ],
    )?;

    tx.commit()?;

    Ok(PreparedEvaluation {
        evaluation_id,
        session_id,
        essay: draft.content_text,
        prompt: draft.prompt_snapshot,
        task_type: parse_task(cmd.task_type.as_deref()),
        existing: None,
    })
}

pub fn finish_evaluation(
    conn: &Connection,
    prepared: &PreparedEvaluation,
    score: Result<WritingScoreV4, ProviderError>,
    feedback: Option<WritingFeedbackV4>,
    review_error: Option<ProviderError>,
) -> DbResult<EvaluationRunResult> {
    let provider = PreparedProvider {
        score,
        feedback,
        review_error,
    };
    run_state_machine(
        conn,
        &prepared.evaluation_id,
        &prepared.session_id,
        &provider,
        &prepared.essay,
        prepared.prompt.as_deref(),
        prepared.task_type,
    )
}

struct PreparedProvider {
    score: Result<WritingScoreV4, ProviderError>,
    feedback: Option<WritingFeedbackV4>,
    review_error: Option<ProviderError>,
}

impl WritingProvider for PreparedProvider {
    fn id(&self) -> &str {
        "prepared"
    }
    fn model(&self) -> &str {
        "prepared"
    }
    fn score(
        &self,
        _essay: &str,
        _prompt: Option<&str>,
        _task_type: Option<WritingTaskType>,
    ) -> Result<WritingScoreV4, ProviderError> {
        self.score.clone()
    }
    fn review(
        &self,
        _essay: &str,
        _score: &WritingScoreV4,
    ) -> Result<WritingFeedbackV4, ProviderError> {
        if let Some(error) = &self.review_error {
            return Err(error.clone());
        }
        self.feedback.clone().ok_or_else(|| ProviderError {
            message: "provider returned no feedback".into(),
            retryable: false,
        })
    }
}

fn run_state_machine(
    conn: &Connection,
    evaluation_id: &str,
    session_id: &str,
    provider: &dyn WritingProvider,
    essay: &str,
    prompt: Option<&str>,
    task_type: Option<WritingTaskType>,
) -> DbResult<EvaluationRunResult> {
    let mut events = Vec::new();
    let mut revision = 1u32;

    // Preparing
    if is_cancel_requested(conn, session_id)? {
        return finalize_cancelled(conn, evaluation_id, session_id, events);
    }
    let mut evaluation = empty_eval(EvaluationStatus::Running, EvaluationStage::Preparing);
    evaluation.task_type = task_type;
    persist_stage(
        conn,
        evaluation_id,
        session_id,
        EvaluationStatus::Running,
        EvaluationStage::Preparing,
        revision,
        &evaluation,
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        revision,
        "stage",
        Some(EvaluationStage::Preparing),
        json!({ "message": "preparing" }),
    )?);
    save_checkpoint(
        conn,
        evaluation_id,
        EvaluationStage::Preparing,
        revision,
        &evaluation,
    )?;

    // Scoring
    if is_cancel_requested(conn, session_id)? {
        return finalize_cancelled(conn, evaluation_id, session_id, events);
    }
    evaluation.stage = EvaluationStage::Scoring;
    persist_stage(
        conn,
        evaluation_id,
        session_id,
        EvaluationStatus::Running,
        EvaluationStage::Scoring,
        revision,
        &evaluation,
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        revision,
        "stage",
        Some(EvaluationStage::Scoring),
        json!({ "message": "scoring" }),
    )?);

    let score = match provider.score(essay, prompt, task_type) {
        Ok(s) => s,
        Err(err) => {
            return finalize_failed(conn, evaluation_id, session_id, evaluation, events, err);
        }
    };
    evaluation.score = Some(score.clone());
    revision += 1;
    persist_stage(
        conn,
        evaluation_id,
        session_id,
        EvaluationStatus::Running,
        EvaluationStage::Scoring,
        revision,
        &evaluation,
    )?;
    save_checkpoint(
        conn,
        evaluation_id,
        EvaluationStage::Scoring,
        revision,
        &evaluation,
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        revision,
        "score",
        Some(EvaluationStage::Scoring),
        json!({ "score": score }),
    )?);

    // Reviewing
    if is_cancel_requested(conn, session_id)? {
        // Keep score checkpoint; mark interrupted/cancelled without deleting inputs
        return finalize_cancelled_with_partial(
            conn,
            evaluation_id,
            session_id,
            evaluation,
            events,
        );
    }
    evaluation.stage = EvaluationStage::Reviewing;
    persist_stage(
        conn,
        evaluation_id,
        session_id,
        EvaluationStatus::Running,
        EvaluationStage::Reviewing,
        revision,
        &evaluation,
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        revision,
        "stage",
        Some(EvaluationStage::Reviewing),
        json!({ "message": "reviewing" }),
    )?);

    match provider.review(essay, &score) {
        Ok(feedback) => {
            evaluation.feedback = Some(feedback);
        }
        Err(err) => {
            // Degrade: keep score, mark degraded
            evaluation.status = EvaluationStatus::Degraded;
            evaluation.degradation = Some(EvaluationDegradation {
                stage: EvaluationStage::Reviewing,
                reason: err.message.clone(),
                missing: vec!["feedback".into(), "sentences".into()],
            });
            revision += 1;
            save_checkpoint(
                conn,
                evaluation_id,
                EvaluationStage::Reviewing,
                revision,
                &evaluation,
            )?;
            events.push(append_event(
                conn,
                evaluation_id,
                revision,
                "degraded",
                Some(EvaluationStage::Reviewing),
                json!({ "reason": err.message }),
            )?);
            return finalize_completed(conn, evaluation_id, session_id, evaluation, events, true);
        }
    }
    revision += 1;
    save_checkpoint(
        conn,
        evaluation_id,
        EvaluationStage::Reviewing,
        revision,
        &evaluation,
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        revision,
        "review",
        Some(EvaluationStage::Reviewing),
        json!({ "hasFeedback": true }),
    )?);

    // Finalizing
    evaluation.stage = EvaluationStage::Finalizing;
    evaluation.status = EvaluationStatus::Completed;
    revision += 1;
    save_checkpoint(
        conn,
        evaluation_id,
        EvaluationStage::Finalizing,
        revision,
        &evaluation,
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        revision,
        "stage",
        Some(EvaluationStage::Finalizing),
        json!({ "message": "finalizing" }),
    )?);

    finalize_completed(conn, evaluation_id, session_id, evaluation, events, false)
}

fn finalize_completed(
    conn: &Connection,
    evaluation_id: &str,
    session_id: &str,
    mut evaluation: WritingEvaluationV4,
    mut events: Vec<EvaluationEvent>,
    degraded: bool,
) -> DbResult<EvaluationRunResult> {
    let now = chrono::Utc::now().to_rfc3339();
    if degraded {
        evaluation.status = EvaluationStatus::Degraded;
    } else {
        evaluation.status = EvaluationStatus::Completed;
    }
    evaluation.stage = EvaluationStage::Finalizing;
    let result_json =
        serde_json::to_string(&evaluation).map_err(|e| DbError::Message(e.to_string()))?;
    let degradation_json = evaluation
        .degradation
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_else(|_| "null".into()));

    conn.execute(
        "UPDATE writing_evaluations SET status = ?1, stage = ?2, result_json = ?3, degradation_json = ?4,
            completed_at = ?5, updated_at = ?5 WHERE id = ?6",
        params![
            status_str(evaluation.status),
            stage_str(EvaluationStage::Finalizing),
            result_json,
            degradation_json,
            now,
            evaluation_id
        ],
    )?;

    conn.execute(
        "UPDATE evaluation_sessions SET status = ?1, stage = ?2, completed_at = ?3, updated_at = ?3 WHERE id = ?4",
        params![
            status_str(evaluation.status),
            stage_str(EvaluationStage::Finalizing),
            now,
            session_id
        ],
    )?;

    // Persist score onto attempt
    if let Some(score) = &evaluation.score {
        conn.execute(
            "UPDATE attempts SET status = ?1, score_value = ?2, score_scale = ?3, completed_at = ?4, updated_at = ?4 WHERE id = (
                SELECT attempt_id FROM writing_evaluations WHERE id = ?5
             )",
            params![
                status_attempt(AttemptStatus::Completed),
                score.overall,
                "band9",
                now,
                evaluation_id
            ],
        )?;
    }

    events.push(append_event(
        conn,
        evaluation_id,
        0,
        "completed",
        Some(EvaluationStage::Finalizing),
        json!({ "status": status_str(evaluation.status) }),
    )?);

    let session = load_session(conn, session_id)?.expect("session");
    Ok(EvaluationRunResult {
        session,
        evaluation,
        events,
    })
}

fn finalize_failed(
    conn: &Connection,
    evaluation_id: &str,
    session_id: &str,
    mut evaluation: WritingEvaluationV4,
    mut events: Vec<EvaluationEvent>,
    err: ProviderError,
) -> DbResult<EvaluationRunResult> {
    let now = chrono::Utc::now().to_rfc3339();
    evaluation.status = EvaluationStatus::Failed;
    evaluation.error = Some(ErrorEnvelope::new(
        "provider.failed",
        err.message.clone(),
        err.retryable,
    ));
    let result_json = serde_json::to_string(&evaluation).unwrap_or_else(|_| "{}".into());
    let error_json = serde_json::to_string(evaluation.error.as_ref().unwrap()).unwrap();
    conn.execute(
        "UPDATE writing_evaluations SET status = ?1, result_json = ?2, error_json = ?3, updated_at = ?4 WHERE id = ?5",
        params![status_str(EvaluationStatus::Failed), result_json, error_json, now, evaluation_id],
    )?;
    conn.execute(
        "UPDATE evaluation_sessions SET status = ?1, updated_at = ?2, completed_at = ?2 WHERE id = ?3",
        params![status_str(EvaluationStatus::Failed), now, session_id],
    )?;
    events.push(append_event(
        conn,
        evaluation_id,
        0,
        "failed",
        Some(evaluation.stage),
        json!({ "message": err.message }),
    )?);
    let session = load_session(conn, session_id)?.expect("session");
    Ok(EvaluationRunResult {
        session,
        evaluation,
        events,
    })
}

fn finalize_cancelled(
    conn: &Connection,
    evaluation_id: &str,
    session_id: &str,
    events: Vec<EvaluationEvent>,
) -> DbResult<EvaluationRunResult> {
    let evaluation = empty_eval(EvaluationStatus::Interrupted, EvaluationStage::Preparing);
    finalize_cancelled_with_partial(conn, evaluation_id, session_id, evaluation, events)
}

fn finalize_cancelled_with_partial(
    conn: &Connection,
    evaluation_id: &str,
    session_id: &str,
    mut evaluation: WritingEvaluationV4,
    mut events: Vec<EvaluationEvent>,
) -> DbResult<EvaluationRunResult> {
    let now = chrono::Utc::now().to_rfc3339();
    evaluation.status = EvaluationStatus::Interrupted;
    let result_json = serde_json::to_string(&evaluation).unwrap_or_else(|_| "{}".into());
    conn.execute(
        "UPDATE writing_evaluations SET status = ?1, result_json = ?2, updated_at = ?3 WHERE id = ?4",
        params![status_str(EvaluationStatus::Interrupted), result_json, now, evaluation_id],
    )?;
    conn.execute(
        "UPDATE evaluation_sessions SET status = ?1, cancel_requested = 1, updated_at = ?2, completed_at = ?2 WHERE id = ?3",
        params![status_str(EvaluationStatus::Interrupted), now, session_id],
    )?;
    // Do NOT wipe attempt content / draft
    events.push(append_event(
        conn,
        evaluation_id,
        0,
        "cancelled",
        Some(evaluation.stage),
        json!({ "keptInputs": true }),
    )?);
    let session = load_session(conn, session_id)?.expect("session");
    Ok(EvaluationRunResult {
        session,
        evaluation,
        events,
    })
}

pub fn request_cancel(conn: &Connection, evaluation_id: &str) -> DbResult<bool> {
    let n = conn.execute(
        "UPDATE evaluation_sessions SET cancel_requested = 1, updated_at = ?1 WHERE evaluation_id = ?2 AND completed_at IS NULL",
        params![chrono::Utc::now().to_rfc3339(), evaluation_id],
    )?;
    Ok(n > 0)
}

pub fn list_events(
    conn: &Connection,
    evaluation_id: &str,
    after_seq: u32,
) -> DbResult<Vec<EvaluationEvent>> {
    let mut stmt = conn.prepare(
        "SELECT evaluation_id, sequence, revision, event_type, stage, payload_json, created_at
         FROM evaluation_events
         WHERE evaluation_id = ?1 AND sequence > ?2
         ORDER BY sequence ASC",
    )?;
    let rows = stmt.query_map(params![evaluation_id, after_seq as i64], |row| {
        let stage_raw: Option<String> = row.get(4)?;
        let payload_json: String = row.get(5)?;
        Ok(EvaluationEvent {
            evaluation_id: row.get(0)?,
            sequence: row.get::<_, i64>(1)? as u32,
            revision: row.get::<_, i64>(2)? as u32,
            event_type: row.get(3)?,
            stage: stage_raw.as_deref().and_then(parse_stage),
            payload: serde_json::from_str(&payload_json).unwrap_or(Value::Null),
            created_at: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn load_evaluation_for_attempt(
    conn: &Connection,
    attempt_id: &str,
) -> DbResult<Option<WritingEvaluationV4>> {
    let result: Result<String, _> = conn.query_row(
        "SELECT result_json FROM writing_evaluations WHERE attempt_id = ?1 ORDER BY updated_at DESC LIMIT 1",
        params![attempt_id],
        |r| r.get(0),
    );
    match result {
        Ok(json) => {
            if json.is_empty() {
                return Ok(None);
            }
            let v: WritingEvaluationV4 = serde_json::from_str(&json)
                .map_err(|e| DbError::Message(format!("eval parse: {e}")))?;
            Ok(Some(v))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn recover_interrupted_sessions(conn: &Connection) -> DbResult<u32> {
    // On boot: mark running sessions without completed_at as interrupted (process died).
    let now = chrono::Utc::now().to_rfc3339();
    let n = conn.execute(
        "UPDATE evaluation_sessions
         SET status = ?1, updated_at = ?2
         WHERE completed_at IS NULL AND status IN ('queued', 'running')",
        params![status_str(EvaluationStatus::Interrupted), now],
    )?;
    conn.execute(
        "UPDATE writing_evaluations
         SET status = ?1, updated_at = ?2
         WHERE completed_at IS NULL AND status IN ('queued', 'running')
           AND id IN (SELECT evaluation_id FROM evaluation_sessions WHERE status = ?1)",
        params![status_str(EvaluationStatus::Interrupted), now],
    )?;
    Ok(n as u32)
}

fn append_event(
    conn: &Connection,
    evaluation_id: &str,
    revision: u32,
    event_type: &str,
    stage: Option<EvaluationStage>,
    payload: Value,
) -> DbResult<EvaluationEvent> {
    let now = chrono::Utc::now().to_rfc3339();
    // sequence from session
    let seq: i64 = conn.query_row(
        "SELECT sequence FROM evaluation_sessions WHERE evaluation_id = ?1",
        params![evaluation_id],
        |r| r.get(0),
    )?;
    let next = seq + 1;
    conn.execute(
        "UPDATE evaluation_sessions SET sequence = ?1, updated_at = ?2 WHERE evaluation_id = ?3",
        params![next, now, evaluation_id],
    )?;
    conn.execute(
        "UPDATE evaluation_sessions SET revision = ?1 WHERE evaluation_id = ?2 AND revision < ?1",
        params![revision as i64, evaluation_id],
    )?;

    let payload_json = payload.to_string();
    conn.execute(
        "INSERT INTO evaluation_events (evaluation_id, sequence, revision, event_type, stage, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            evaluation_id,
            next,
            revision as i64,
            event_type,
            stage.map(stage_str),
            payload_json,
            now
        ],
    )?;
    Ok(EvaluationEvent {
        evaluation_id: evaluation_id.to_string(),
        sequence: next as u32,
        revision,
        event_type: event_type.to_string(),
        stage,
        payload,
        created_at: now,
    })
}

fn save_checkpoint(
    conn: &Connection,
    evaluation_id: &str,
    stage: EvaluationStage,
    revision: u32,
    evaluation: &WritingEvaluationV4,
) -> DbResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::to_string(evaluation).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT OR REPLACE INTO evaluation_checkpoints (evaluation_id, stage, revision, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![evaluation_id, stage_str(stage), revision as i64, payload, now],
    )?;
    Ok(())
}

fn persist_stage(
    conn: &Connection,
    evaluation_id: &str,
    session_id: &str,
    status: EvaluationStatus,
    stage: EvaluationStage,
    revision: u32,
    evaluation: &WritingEvaluationV4,
) -> DbResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let result_json =
        serde_json::to_string(evaluation).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "UPDATE writing_evaluations SET status = ?1, stage = ?2, result_json = ?3, updated_at = ?4 WHERE id = ?5",
        params![status_str(status), stage_str(stage), result_json, now, evaluation_id],
    )?;
    conn.execute(
        "UPDATE evaluation_sessions SET status = ?1, stage = ?2, revision = ?3, updated_at = ?4 WHERE id = ?5",
        params![status_str(status), stage_str(stage), revision as i64, now, session_id],
    )?;
    Ok(())
}

fn is_cancel_requested(conn: &Connection, session_id: &str) -> DbResult<bool> {
    let v: i64 = conn.query_row(
        "SELECT cancel_requested FROM evaluation_sessions WHERE id = ?1",
        params![session_id],
        |r| r.get(0),
    )?;
    Ok(v != 0)
}

fn load_session(conn: &Connection, session_id: &str) -> DbResult<Option<EvaluationSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, attempt_id, evaluation_id, status, stage, revision, sequence, retry_of,
                cancel_requested, provider_id, model, started_at, updated_at, completed_at
         FROM evaluation_sessions WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![session_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(map_session(row)?))
    } else {
        Ok(None)
    }
}

fn load_session_by_evaluation(
    conn: &Connection,
    evaluation_id: &str,
) -> DbResult<Option<EvaluationSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, attempt_id, evaluation_id, status, stage, revision, sequence, retry_of,
                cancel_requested, provider_id, model, started_at, updated_at, completed_at
         FROM evaluation_sessions WHERE evaluation_id = ?1 ORDER BY started_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![evaluation_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(map_session(row)?))
    } else {
        Ok(None)
    }
}

fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvaluationSession> {
    Ok(EvaluationSession {
        id: row.get(0)?,
        attempt_id: row.get(1)?,
        evaluation_id: row.get(2)?,
        status: parse_status(&row.get::<_, String>(3)?).unwrap_or(EvaluationStatus::Queued),
        stage: parse_stage(&row.get::<_, String>(4)?).unwrap_or(EvaluationStage::Preparing),
        revision: row.get::<_, i64>(5)? as u32,
        sequence: row.get::<_, i64>(6)? as u32,
        retry_of: row.get(7)?,
        cancel_requested: row.get::<_, i64>(8)? != 0,
        provider_id: row.get(9)?,
        model: row.get(10)?,
        started_at: row.get(11)?,
        updated_at: row.get(12)?,
        completed_at: row.get(13)?,
    })
}

fn load_evaluation_result(
    conn: &Connection,
    evaluation_id: &str,
) -> DbResult<Option<WritingEvaluationV4>> {
    let json: Option<String> = conn.query_row(
        "SELECT result_json FROM writing_evaluations WHERE id = ?1",
        params![evaluation_id],
        |r| r.get(0),
    )?;
    match json {
        Some(j) if !j.is_empty() => {
            let v = serde_json::from_str(&j).map_err(|e| DbError::Message(e.to_string()))?;
            Ok(Some(v))
        }
        _ => Ok(None),
    }
}

fn resolve_root_evaluation(conn: &Connection, retry_of: &str) -> DbResult<Option<String>> {
    let result: Result<Option<String>, _> = conn.query_row(
        "SELECT root_evaluation_id FROM evaluation_lineage WHERE evaluation_id = ?1",
        params![retry_of],
        |r| r.get(0),
    );
    match result {
        Ok(v) => Ok(v.or_else(|| Some(retry_of.to_string()))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Some(retry_of.to_string())),
        Err(e) => Err(e.into()),
    }
}

fn empty_eval(status: EvaluationStatus, stage: EvaluationStage) -> WritingEvaluationV4 {
    WritingEvaluationV4 {
        schema_version: WritingEvaluationV4::SCHEMA_VERSION,
        status,
        stage,
        task_type: None,
        score: None,
        diagnosis: None,
        feedback: None,
        degradation: None,
        error: None,
    }
}

fn status_str(s: EvaluationStatus) -> &'static str {
    match s {
        EvaluationStatus::Queued => "queued",
        EvaluationStatus::Running => "running",
        EvaluationStatus::Completed => "completed",
        EvaluationStatus::Degraded => "degraded",
        EvaluationStatus::Failed => "failed",
        EvaluationStatus::Interrupted => "interrupted",
    }
}

fn stage_str(s: EvaluationStage) -> &'static str {
    match s {
        EvaluationStage::Preparing => "preparing",
        EvaluationStage::Scoring => "scoring",
        EvaluationStage::Reviewing => "reviewing",
        EvaluationStage::Finalizing => "finalizing",
    }
}

fn status_attempt(s: AttemptStatus) -> &'static str {
    match s {
        AttemptStatus::Draft => "draft",
        AttemptStatus::Active => "active",
        AttemptStatus::Submitted => "submitted",
        AttemptStatus::Reviewing => "reviewing",
        AttemptStatus::Completed => "completed",
        AttemptStatus::Cancelled => "cancelled",
        AttemptStatus::Failed => "failed",
        AttemptStatus::Interrupted => "interrupted",
    }
}

fn parse_status(raw: &str) -> Option<EvaluationStatus> {
    Some(match raw {
        "queued" => EvaluationStatus::Queued,
        "running" => EvaluationStatus::Running,
        "completed" => EvaluationStatus::Completed,
        "degraded" => EvaluationStatus::Degraded,
        "failed" => EvaluationStatus::Failed,
        "interrupted" => EvaluationStatus::Interrupted,
        _ => return None,
    })
}

fn parse_stage(raw: &str) -> Option<EvaluationStage> {
    Some(match raw {
        "preparing" => EvaluationStage::Preparing,
        "scoring" => EvaluationStage::Scoring,
        "reviewing" => EvaluationStage::Reviewing,
        "finalizing" => EvaluationStage::Finalizing,
        _ => return None,
    })
}

fn parse_task(raw: Option<&str>) -> Option<WritingTaskType> {
    raw.and_then(WritingTaskType::parse_loose)
}
