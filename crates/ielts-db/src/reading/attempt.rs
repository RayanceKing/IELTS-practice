//! Reading attempt drafts + idempotent submit with scoring (Phase 6).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use ielts_domain::domain::{Activity, AttemptMode, AttemptStatus, ScoreScale};
use ielts_domain::dto::{AttemptAnswer, AttemptRecord};

use crate::attempts::upsert_attempt;
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
    crate::attempts::ensure_asset_stub(
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
            marked: cmd
                .marked_questions
                .iter()
                .any(|m| crate::reading::scoring::normalize_question_id(m) == c.question_id),
            answered_at: Some(now.clone()),
        })
        .collect();

    crate::attempts::ensure_asset_stub(
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

/// Latest draft attempt for an asset, with answers hydrated.
pub fn get_open_reading_draft(
    conn: &Connection,
    asset_id: &str,
) -> DbResult<Option<AttemptRecord>> {
    let asset_id = asset_id.trim();
    if asset_id.is_empty() {
        return Err(DbError::Validation("asset_id required".into()));
    }
    let attempt_id: Option<String> = conn
        .query_row(
            "SELECT id FROM attempts
             WHERE activity = 'reading' AND asset_id = ?1 AND lower(status) = 'draft'
             ORDER BY started_at DESC
             LIMIT 1",
            params![asset_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(attempt_id) = attempt_id else {
        return Ok(None);
    };

    let mut attempt = conn.query_row(
        "SELECT id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
                duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
                prompt_snapshot, content_text, schema_version
         FROM attempts WHERE id = ?1",
        params![attempt_id],
        |row| {
            let activity = match row.get::<_, String>(1)?.as_str() {
                "writing" => Activity::Writing,
                _ => Activity::Reading,
            };
            let mode = match row.get::<_, String>(3)?.as_str() {
                "suite" => AttemptMode::Suite,
                "endless" => AttemptMode::Endless,
                "memorize" => AttemptMode::Memorize,
                "freeform" => AttemptMode::Freeform,
                "bank" => AttemptMode::Bank,
                _ => AttemptMode::Single,
            };
            let status = match row.get::<_, String>(5)?.as_str() {
                "submitted" => AttemptStatus::Submitted,
                "completed" => AttemptStatus::Completed,
                "cancelled" => AttemptStatus::Cancelled,
                "failed" => AttemptStatus::Failed,
                _ => AttemptStatus::Draft,
            };
            let score_scale = match row.get::<_, Option<String>>(11)? {
                Some(ref s) if s == "band9" => Some(ScoreScale::Band9),
                Some(ref s) if s == "ratio" => Some(ScoreScale::Ratio),
                _ => None,
            };
            Ok(AttemptRecord {
                schema_version: row.get::<_, i64>(17)? as u32,
                id: row.get(0)?,
                activity,
                asset_id: row.get(2)?,
                mode,
                suite_id: row.get(4)?,
                status,
                started_at: row.get(6)?,
                submitted_at: row.get(7)?,
                completed_at: row.get(8)?,
                duration_ms: row.get::<_, i64>(9)? as u64,
                score_value: row.get(10)?,
                score_scale,
                correct_count: row.get(12)?,
                question_count: row.get::<_, Option<i64>>(13)?.map(|v| v as u32),
                title_snapshot: row.get(14)?,
                prompt_snapshot: row.get(15)?,
                content_text: row.get(16)?,
                answers: vec![],
                annotations: vec![],
            })
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT question_id, answer_json, is_correct, weight, question_kind, change_count, visit_count,
                elapsed_ms, marked, answered_at
         FROM attempt_answers WHERE attempt_id = ?1 ORDER BY question_id",
    )?;
    let rows = stmt.query_map(params![attempt.id], |row| {
        let answer_json: String = row.get(1)?;
        let answer = serde_json::from_str(&answer_json).unwrap_or(Value::Null);
        Ok(AttemptAnswer {
            question_id: row.get(0)?,
            answer,
            is_correct: row.get::<_, Option<i64>>(2)?.map(|v| v != 0),
            weight: row.get(3)?,
            question_kind: row.get(4)?,
            change_count: row.get::<_, i64>(5)? as u32,
            visit_count: row.get::<_, i64>(6)? as u32,
            elapsed_ms: row.get::<_, i64>(7)? as u64,
            marked: row.get::<_, i64>(8)? != 0,
            answered_at: row.get(9)?,
        })
    })?;
    for row in rows {
        attempt.answers.push(row?);
    }
    Ok(Some(attempt))
}

fn lookup_submit_response(conn: &Connection, key: &str) -> DbResult<Option<ReadingSubmitResult>> {
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
        params![
            attempt_id,
            qid,
            answer.to_string(),
            if marked { 1 } else { 0 },
            now
        ],
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
