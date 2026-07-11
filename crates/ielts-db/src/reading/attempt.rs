//! Reading attempt drafts + idempotent submit with scoring (Phase 6).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use ielts_domain::domain::{Activity, AttemptMode, AttemptStatus, ScoreScale};
use ielts_domain::dto::{AttemptAnswer, AttemptRecord};

use crate::import::upsert_attempt;
use crate::reading::assets::{load_answer_key, load_controls, load_kinds};
use crate::reading::scoring::{score_attempt, AnswerComparison, ScoreSummary};
use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingDraftCommand {
    pub attempt_id: String,
    pub asset_id: String,
    #[serde(default)]
    pub answers: Value,
    #[serde(default)]
    pub marked_questions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_snapshot: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSubmitCommand {
    pub attempt_id: String,
    pub asset_id: String,
    /// Full reading payload (answerKey, interactionModel, questionGroups).
    pub payload: Value,
    #[serde(default)]
    pub answers: Value,
    #[serde(default)]
    pub marked_questions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_snapshot: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSubmitResult {
    pub attempt: AttemptRecord,
    pub score: ScoreSummary,
    pub comparisons: Vec<AnswerComparison>,
    pub idempotent_replay: bool,
}

pub fn save_reading_draft(conn: &Connection, cmd: &ReadingDraftCommand) -> DbResult<AttemptRecord> {
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let answers = answers_to_vec(&cmd.answers, None);
    let attempt = AttemptRecord {
        schema_version: AttemptRecord::SCHEMA_VERSION,
        id: cmd.attempt_id.clone(),
        activity: Activity::Reading,
        asset_id: Some(cmd.asset_id.clone()),
        mode: AttemptMode::Single,
        suite_id: None,
        status: AttemptStatus::Draft,
        started_at: now.clone(),
        submitted_at: None,
        completed_at: None,
        duration_ms: 0,
        score_value: None,
        score_scale: None,
        correct_count: None,
        question_count: None,
        title_snapshot: cmd.title_snapshot.clone(),
        prompt_snapshot: None,
        content_text: None,
        answers,
        annotations: vec![],
    };
    // ensure asset stub
    crate::import::ensure_asset_stub(
        conn,
        &cmd.asset_id,
        Activity::Reading,
        cmd.title_snapshot.as_deref().unwrap_or(&cmd.asset_id),
        Some(&cmd.asset_id),
    )?;
    upsert_attempt(conn, &attempt)?;
    // store marked as JSON in settings-like side table via attempt answers marked flag already in answers
    conn.execute(
        "INSERT INTO attempt_idempotency (scope, idempotency_key, attempt_id, evaluation_id, response_json, created_at)
         VALUES ('reading.draft', ?1, ?2, NULL, NULL, ?3)
         ON CONFLICT(scope, idempotency_key) DO UPDATE SET attempt_id = excluded.attempt_id, created_at = excluded.created_at",
        params![cmd.idempotency_key, cmd.attempt_id, now],
    )?;
    // persist marked list
    let marked_json = serde_json::to_string(&cmd.marked_questions).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT INTO settings (namespace, key, value_json, updated_at) VALUES ('reading_draft', ?1, ?2, ?3)
         ON CONFLICT(namespace, key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        params![cmd.attempt_id, marked_json, now],
    )?;
    Ok(attempt)
}

pub fn submit_reading_attempt(
    conn: &Connection,
    cmd: &ReadingSubmitCommand,
) -> DbResult<ReadingSubmitResult> {
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }

    // Idempotent replay
    if let Ok(Some(prev)) = lookup_submit_response(conn, &cmd.idempotency_key) {
        return Ok(prev);
    }

    let answer_key = load_answer_key(&cmd.payload);
    if answer_key.is_empty() {
        return Err(DbError::Validation("payload.answerKey required".into()));
    }
    let controls = load_controls(&cmd.payload);
    let kinds = load_kinds(&cmd.payload);
    let user_map = value_to_map(&cmd.answers);

    let (summary, comparisons) = score_attempt(&answer_key, &user_map, &controls, &kinds);
    let now = chrono::Utc::now().to_rfc3339();
    let answers = comparisons
        .iter()
        .map(|c| AttemptAnswer {
            question_id: c.question_id.clone(),
            answer: c.user_answer.clone(),
            is_correct: c.is_correct,
            weight: c.weight,
            question_kind: c.question_kind.clone(),
            change_count: 0,
            visit_count: 0,
            elapsed_ms: 0,
            marked: cmd.marked_questions.iter().any(|m| {
                crate::reading::scoring::normalize_question_id(m) == c.question_id
            }),
            answered_at: Some(now.clone()),
        })
        .collect();

    crate::import::ensure_asset_stub(
        conn,
        &cmd.asset_id,
        Activity::Reading,
        cmd.title_snapshot.as_deref().unwrap_or(&cmd.asset_id),
        Some(&cmd.asset_id),
    )?;

    let attempt = AttemptRecord {
        schema_version: AttemptRecord::SCHEMA_VERSION,
        id: cmd.attempt_id.clone(),
        activity: Activity::Reading,
        asset_id: Some(cmd.asset_id.clone()),
        mode: AttemptMode::Single,
        suite_id: None,
        status: AttemptStatus::Completed,
        started_at: now.clone(),
        submitted_at: Some(now.clone()),
        completed_at: Some(now.clone()),
        duration_ms: cmd.duration_ms.unwrap_or(0),
        score_value: Some(summary.accuracy),
        score_scale: Some(ScoreScale::Ratio),
        correct_count: Some(summary.correct),
        question_count: Some(summary.total as u32),
        title_snapshot: cmd.title_snapshot.clone(),
        prompt_snapshot: None,
        content_text: None,
        answers,
        annotations: vec![],
    };

    // Transaction: score + status together
    let tx = conn.unchecked_transaction()?;
    upsert_attempt(&tx, &attempt)?;
    let result = ReadingSubmitResult {
        attempt: attempt.clone(),
        score: summary,
        comparisons: comparisons.clone(),
        idempotent_replay: false,
    };
    let response_json =
        serde_json::to_string(&result).map_err(|e| DbError::Message(e.to_string()))?;
    tx.execute(
        "INSERT INTO attempt_idempotency (scope, idempotency_key, attempt_id, evaluation_id, response_json, created_at)
         VALUES ('reading.submit', ?1, ?2, NULL, ?3, ?4)",
        params![cmd.idempotency_key, cmd.attempt_id, response_json, now],
    )?;
    tx.commit()?;

    Ok(result)
}

fn lookup_submit_response(
    conn: &Connection,
    key: &str,
) -> DbResult<Option<ReadingSubmitResult>> {
    let mut stmt = conn.prepare(
        "SELECT response_json FROM attempt_idempotency WHERE scope = 'reading.submit' AND idempotency_key = ?1",
    )?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        let mut result: ReadingSubmitResult = serde_json::from_str(&json)
            .map_err(|e| DbError::Message(format!("idempotency parse: {e}")))?;
        result.idempotent_replay = true;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn value_to_map(v: &Value) -> serde_json::Map<String, Value> {
    match v {
        Value::Object(m) => m
            .iter()
            .map(|(k, val)| {
                (
                    crate::reading::scoring::normalize_question_id(k),
                    val.clone(),
                )
            })
            .collect(),
        _ => serde_json::Map::new(),
    }
}

fn answers_to_vec(answers: &Value, marked: Option<&[String]>) -> Vec<AttemptAnswer> {
    let map = value_to_map(answers);
    let mut out = Vec::new();
    for (qid, ans) in map {
        let marked_flag = marked
            .map(|m| {
                m.iter()
                    .any(|x| crate::reading::scoring::normalize_question_id(x) == qid)
            })
            .unwrap_or(false);
        out.push(AttemptAnswer {
            question_id: qid,
            answer: ans,
            is_correct: None,
            weight: 1.0,
            question_kind: None,
            change_count: 0,
            visit_count: 0,
            elapsed_ms: 0,
            marked: marked_flag,
            answered_at: None,
        });
    }
    out
}

/// Incremental answer save without full resubmit.
pub fn patch_reading_answer(
    conn: &Connection,
    attempt_id: &str,
    question_id: &str,
    answer: &Value,
    marked: bool,
) -> DbResult<()> {
    let qid = crate::reading::scoring::normalize_question_id(question_id);
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO attempt_answers (
            attempt_id, question_id, answer_json, is_correct, weight, question_kind,
            change_count, visit_count, elapsed_ms, marked, answered_at
         ) VALUES (?1, ?2, ?3, NULL, 1, NULL, 1, 1, 0, ?4, ?5)
         ON CONFLICT(attempt_id, question_id) DO UPDATE SET
            answer_json = excluded.answer_json,
            change_count = attempt_answers.change_count + 1,
            marked = excluded.marked,
            answered_at = excluded.answered_at",
        params![attempt_id, qid, answer.to_string(), if marked { 1 } else { 0 }, now],
    )?;
    // touch attempt
    conn.execute(
        "UPDATE attempts SET updated_at = ?1, status = CASE WHEN status = 'completed' THEN status ELSE 'active' END WHERE id = ?2",
        params![now, attempt_id],
    )?;
    Ok(())
}

pub fn new_attempt_id() -> String {
    format!("reading-{}", Uuid::new_v4())
}

#[allow(dead_code)]
fn _json_touch() -> Value {
    json!({})
}
