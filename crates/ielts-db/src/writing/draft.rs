//! Writing draft repository + idempotent submit tokens (Phase 5).

use rusqlite::{params, Connection};

use ielts_domain::domain::{Activity, AttemptMode, AttemptStatus};
use ielts_domain::dto::{AttemptRecord, SaveDraftCommand, SubmitAttemptCommand};

use crate::attempts::upsert_attempt;
use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingDraft {
    pub attempt_id: String,
    pub content_text: String,
    pub prompt_snapshot: Option<String>,
    pub task_type: Option<String>,
    pub word_count: u32,
    pub idempotency_key: Option<String>,
    pub updated_at: String,
}

pub fn save_writing_draft(conn: &Connection, cmd: &SaveDraftCommand) -> DbResult<WritingDraft> {
    if cmd.activity != Activity::Writing {
        return Err(DbError::Validation(
            "save_writing_draft requires activity=writing".into(),
        ));
    }
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let content = cmd.content_text.clone().unwrap_or_default();
    let word_count = count_words(&content);

    // Ensure attempt row exists (draft status).
    let attempt = AttemptRecord {
        schema_version: AttemptRecord::SCHEMA_VERSION,
        id: cmd.attempt_id.clone(),
        activity: Activity::Writing,
        asset_id: cmd.asset_id.clone(),
        mode: cmd.mode,
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
        title_snapshot: None,
        prompt_snapshot: cmd.prompt_snapshot.clone(),
        content_text: Some(content.clone()),
        answers: vec![],
        annotations: vec![],
    };
    upsert_attempt(conn, &attempt)?;

    conn.execute(
        "INSERT INTO writing_drafts (
            attempt_id, content_text, prompt_snapshot, task_type, word_count, idempotency_key, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(attempt_id) DO UPDATE SET
            content_text = excluded.content_text,
            prompt_snapshot = excluded.prompt_snapshot,
            task_type = excluded.task_type,
            word_count = excluded.word_count,
            idempotency_key = excluded.idempotency_key,
            updated_at = excluded.updated_at",
        params![
            cmd.attempt_id,
            content,
            cmd.prompt_snapshot,
            mode_task_hint(cmd.mode),
            word_count as i64,
            cmd.idempotency_key,
            now,
        ],
    )?;

    // Record draft idempotency (latest wins for same key scope).
    conn.execute(
        "INSERT INTO attempt_idempotency (scope, idempotency_key, attempt_id, evaluation_id, response_json, created_at)
         VALUES ('writing.draft', ?1, ?2, NULL, NULL, ?3)
         ON CONFLICT(scope, idempotency_key) DO UPDATE SET
            attempt_id = excluded.attempt_id,
            created_at = excluded.created_at",
        params![cmd.idempotency_key, cmd.attempt_id, now],
    )?;

    Ok(WritingDraft {
        attempt_id: cmd.attempt_id.clone(),
        content_text: content,
        prompt_snapshot: cmd.prompt_snapshot.clone(),
        task_type: mode_task_hint(cmd.mode).map(str::to_string),
        word_count,
        idempotency_key: Some(cmd.idempotency_key.clone()),
        updated_at: now,
    })
}

pub fn get_writing_draft(conn: &Connection, attempt_id: &str) -> DbResult<Option<WritingDraft>> {
    let mut stmt = conn.prepare(
        "SELECT attempt_id, content_text, prompt_snapshot, task_type, word_count, idempotency_key, updated_at
         FROM writing_drafts WHERE attempt_id = ?1",
    )?;
    let mut rows = stmt.query(params![attempt_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(WritingDraft {
            attempt_id: row.get(0)?,
            content_text: row.get(1)?,
            prompt_snapshot: row.get(2)?,
            task_type: row.get(3)?,
            word_count: row.get::<_, i64>(4)? as u32,
            idempotency_key: row.get(5)?,
            updated_at: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

/// Mark attempt submitted. Idempotent: same key returns prior attempt_id without re-mutating.
pub fn submit_writing_attempt(
    conn: &Connection,
    cmd: &SubmitAttemptCommand,
) -> DbResult<AttemptRecord> {
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }

    if let Some(existing) =
        lookup_idempotency(conn, "writing.submit", &cmd.idempotency_key)?
    {
        return load_attempt_minimal(conn, &existing.attempt_id);
    }

    let now = chrono::Utc::now().to_rfc3339();
    let draft = get_writing_draft(conn, &cmd.attempt_id)?
        .ok_or_else(|| DbError::Validation(format!("no draft for {}", cmd.attempt_id)))?;

    if draft.content_text.trim().is_empty() {
        return Err(DbError::Validation("cannot submit empty essay".into()));
    }

    let mut attempt = load_attempt_minimal(conn, &cmd.attempt_id)?;
    attempt.status = AttemptStatus::Submitted;
    attempt.submitted_at = Some(now.clone());
    attempt.content_text = Some(draft.content_text.clone());
    attempt.prompt_snapshot = draft.prompt_snapshot.clone();
    upsert_attempt(conn, &attempt)?;

    let response = serde_json::json!({ "attemptId": attempt.id, "status": "submitted" });
    conn.execute(
        "INSERT INTO attempt_idempotency (scope, idempotency_key, attempt_id, evaluation_id, response_json, created_at)
         VALUES ('writing.submit', ?1, ?2, NULL, ?3, ?4)",
        params![
            cmd.idempotency_key,
            attempt.id,
            response.to_string(),
            now
        ],
    )?;

    Ok(attempt)
}

#[derive(Debug)]
struct IdemRow {
    attempt_id: String,
}

fn lookup_idempotency(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> DbResult<Option<IdemRow>> {
    let mut stmt = conn.prepare(
        "SELECT attempt_id FROM attempt_idempotency WHERE scope = ?1 AND idempotency_key = ?2",
    )?;
    let mut rows = stmt.query(params![scope, key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(IdemRow {
            attempt_id: row.get(0)?,
        }))
    } else {
        Ok(None)
    }
}

fn load_attempt_minimal(conn: &Connection, id: &str) -> DbResult<AttemptRecord> {
    conn.query_row(
        "SELECT id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
                duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
                prompt_snapshot, content_text, schema_version
         FROM attempts WHERE id = ?1",
        params![id],
        |row| {
            use ielts_domain::domain::ScoreScale;
            Ok(AttemptRecord {
                schema_version: row.get::<_, i64>(17)? as u32,
                id: row.get(0)?,
                activity: match row.get::<_, String>(1)?.as_str() {
                    "writing" => Activity::Writing,
                    _ => Activity::Reading,
                },
                asset_id: row.get(2)?,
                mode: match row.get::<_, String>(3)?.as_str() {
                    "suite" => AttemptMode::Suite,
                    "endless" => AttemptMode::Endless,
                    "memorize" => AttemptMode::Memorize,
                    "freeform" => AttemptMode::Freeform,
                    "bank" => AttemptMode::Bank,
                    _ => AttemptMode::Single,
                },
                suite_id: row.get(4)?,
                status: match row.get::<_, String>(5)?.as_str() {
                    "draft" => AttemptStatus::Draft,
                    "active" => AttemptStatus::Active,
                    "submitted" => AttemptStatus::Submitted,
                    "reviewing" => AttemptStatus::Reviewing,
                    "cancelled" => AttemptStatus::Cancelled,
                    "failed" => AttemptStatus::Failed,
                    "interrupted" => AttemptStatus::Interrupted,
                    _ => AttemptStatus::Completed,
                },
                started_at: row.get(6)?,
                submitted_at: row.get(7)?,
                completed_at: row.get(8)?,
                duration_ms: row.get::<_, i64>(9)? as u64,
                score_value: row.get(10)?,
                score_scale: row.get::<_, Option<String>>(11)?.and_then(|s| match s.as_str() {
                    "ratio" => Some(ScoreScale::Ratio),
                    "band9" => Some(ScoreScale::Band9),
                    _ => None,
                }),
                correct_count: row.get(12)?,
                question_count: row.get::<_, Option<i64>>(13)?.map(|v| v as u32),
                title_snapshot: row.get(14)?,
                prompt_snapshot: row.get(15)?,
                content_text: row.get(16)?,
                answers: vec![],
                annotations: vec![],
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            DbError::Validation(format!("attempt not found: {id}"))
        }
        other => other.into(),
    })
}

fn count_words(text: &str) -> u32 {
    text.split_whitespace().filter(|w| !w.is_empty()).count() as u32
}

fn mode_task_hint(mode: AttemptMode) -> Option<&'static str> {
    match mode {
        AttemptMode::Freeform | AttemptMode::Bank | AttemptMode::Single => None,
        _ => None,
    }
}
