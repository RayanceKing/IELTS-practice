//! Endless mode session (Phase 7).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::reading::attempt::{
    submit_reading_attempt, ReadingQuestionProgress, ReadingSubmitCommand, ReadingSubmitResult,
};
use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndlessStatus {
    Active,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EndlessPoolPolicy {
    /// Category filter e.g. P1/P2/P3; empty = any.
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub frequency_scope: Option<String>,
    /// When true, skip assets already completed in this session.
    #[serde(default = "default_true")]
    pub exclude_completed: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EndlessSession {
    pub id: String,
    pub status: EndlessStatus,
    pub pool_policy: EndlessPoolPolicy,
    pub pool: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_attempt_id: Option<String>,
    pub completed_asset_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEndlessCommand {
    pub pool: Vec<String>,
    #[serde(default)]
    pub pool_policy: Option<EndlessPoolPolicy>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceEndlessCommand {
    pub session_id: String,
    /// Optional forced next asset; otherwise first remaining in pool.
    #[serde(default)]
    pub next_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SubmitEndlessCommand {
    pub session_id: String,
    pub asset_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_revision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_fingerprint: Option<String>,
    #[serde(default)]
    pub answers: Value,
    #[serde(default)]
    pub marked_questions: Vec<String>,
    #[serde(default)]
    pub question_timeline: Vec<ReadingQuestionProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_snapshot: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitEndlessResult {
    pub session: EndlessSession,
    pub submission: ReadingSubmitResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_asset_id: Option<String>,
}

fn status_str(s: EndlessStatus) -> &'static str {
    match s {
        EndlessStatus::Active => "active",
        EndlessStatus::Completed => "completed",
        EndlessStatus::Cancelled => "cancelled",
    }
}

fn parse_status(raw: &str) -> EndlessStatus {
    match raw {
        "completed" => EndlessStatus::Completed,
        "cancelled" => EndlessStatus::Cancelled,
        _ => EndlessStatus::Active,
    }
}

pub fn create_endless_session(
    conn: &Connection,
    cmd: &CreateEndlessCommand,
) -> DbResult<EndlessSession> {
    if cmd.pool.is_empty() {
        return Err(DbError::Validation("endless pool required".into()));
    }
    if let Some(key) = cmd.idempotency_key.as_deref() {
        if !key.trim().is_empty() {
            if let Some(prev) = load_idempotent(conn, key)? {
                return Ok(prev);
            }
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    let id = format!("endless-{}", Uuid::new_v4());
    let policy = cmd.pool_policy.clone().unwrap_or(EndlessPoolPolicy {
        categories: vec![],
        frequency_scope: Some("all".into()),
        exclude_completed: true,
    });
    let first = cmd.pool.first().cloned();
    let session = EndlessSession {
        id: id.clone(),
        status: EndlessStatus::Active,
        pool_policy: policy,
        pool: cmd.pool.clone(),
        current_asset_id: first,
        current_attempt_id: None,
        completed_asset_ids: vec![],
        created_at: now.clone(),
        updated_at: now,
    };
    persist(conn, &session)?;
    if let Some(key) = cmd.idempotency_key.as_deref() {
        if !key.trim().is_empty() {
            store_idempotent(conn, key, &session)?;
        }
    }
    Ok(session)
}

pub fn get_endless_session(conn: &Connection, id: &str) -> DbResult<EndlessSession> {
    load(conn, id)
}

/// Dynamic filter: remaining pool entries not yet completed (when policy says so).
pub fn remaining_pool(session: &EndlessSession) -> Vec<String> {
    if !session.pool_policy.exclude_completed {
        return session.pool.clone();
    }
    session
        .pool
        .iter()
        .filter(|id| !session.completed_asset_ids.iter().any(|c| c == *id))
        .cloned()
        .collect()
}

pub fn advance_endless(conn: &Connection, cmd: &AdvanceEndlessCommand) -> DbResult<EndlessSession> {
    let mut session = load(conn, &cmd.session_id)?;
    if session.status != EndlessStatus::Active {
        return Err(DbError::Validation("endless session not active".into()));
    }
    let remaining = remaining_pool(&session);
    let next = cmd
        .next_asset_id
        .clone()
        .or_else(|| remaining.first().cloned());
    match next {
        Some(asset_id) => {
            session.current_asset_id = Some(asset_id);
            session.current_attempt_id = None;
            session.updated_at = chrono::Utc::now().to_rfc3339();
            persist(conn, &session)?;
            Ok(session)
        }
        None => {
            session.status = EndlessStatus::Completed;
            session.current_asset_id = None;
            session.current_attempt_id = None;
            session.updated_at = chrono::Utc::now().to_rfc3339();
            persist(conn, &session)?;
            Ok(session)
        }
    }
}

pub fn submit_endless_passage(
    conn: &Connection,
    cmd: &SubmitEndlessCommand,
) -> DbResult<SubmitEndlessResult> {
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }
    if let Some(mut prev) = load_submit_idempotent(conn, &cmd.idempotency_key)? {
        if prev.session.id != cmd.session_id
            || prev.submission.attempt.asset_id.as_deref() != Some(cmd.asset_id.as_str())
        {
            return Err(DbError::Validation(
                "idempotency key belongs to another endless submission".into(),
            ));
        }
        prev.submission.idempotent_replay = true;
        return Ok(prev);
    }
    let mut session = load(conn, &cmd.session_id)?;
    if session.status != EndlessStatus::Active {
        return Err(DbError::Validation("endless session not active".into()));
    }
    if session.current_asset_id.as_deref() != Some(cmd.asset_id.as_str()) {
        // still allow if asset is in pool and not completed
        if !session.pool.iter().any(|p| p == &cmd.asset_id) {
            return Err(DbError::Validation("asset not in endless pool".into()));
        }
    }

    // Stable across retries so a crash after the inner reading submit cannot
    // strand the mode transition behind a different generated attempt id.
    let attempt_id = format!(
        "reading-{}-{:016x}",
        session.id,
        stable_key_hash(&cmd.idempotency_key)
    );
    let submission = submit_reading_attempt(
        conn,
        &ReadingSubmitCommand {
            attempt_id: attempt_id.clone(),
            asset_id: cmd.asset_id.clone(),
            asset_revision: cmd.asset_revision,
            asset_fingerprint: cmd.asset_fingerprint.clone(),
            answers: cmd.answers.clone(),
            marked_questions: cmd.marked_questions.clone(),
            question_timeline: cmd.question_timeline.clone(),
            duration_ms: cmd.duration_ms,
            title_snapshot: cmd.title_snapshot.clone(),
            idempotency_key: format!("endless-{}", cmd.idempotency_key),
        },
    )?;

    let persisted_attempt_id = submission.attempt.id.clone();
    conn.execute(
        "UPDATE attempts SET mode = 'endless', suite_id = ?1 WHERE id = ?2",
        params![session.id, persisted_attempt_id],
    )?;

    if !session
        .completed_asset_ids
        .iter()
        .any(|c| c == &cmd.asset_id)
    {
        session.completed_asset_ids.push(cmd.asset_id.clone());
    }
    session.current_attempt_id = Some(persisted_attempt_id);
    let remaining = remaining_pool(&session);
    let next_asset_id = remaining.first().cloned();
    session.current_asset_id = next_asset_id.clone();
    if next_asset_id.is_none() {
        session.status = EndlessStatus::Completed;
    }
    session.updated_at = chrono::Utc::now().to_rfc3339();
    persist(conn, &session)?;

    let result = SubmitEndlessResult {
        session: session.clone(),
        submission,
        next_asset_id,
    };
    store_submit_idempotent(conn, &cmd.idempotency_key, &result)?;
    Ok(result)
}

fn stable_key_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn persist(conn: &Connection, session: &EndlessSession) -> DbResult<()> {
    let policy =
        serde_json::to_string(&session.pool_policy).map_err(|e| DbError::Message(e.to_string()))?;
    let pool = serde_json::to_string(&session.pool).map_err(|e| DbError::Message(e.to_string()))?;
    let completed = serde_json::to_string(&session.completed_asset_ids)
        .map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO endless_sessions (
            id, status, pool_policy_json, pool_json, current_asset_id, current_attempt_id,
            completed_asset_ids_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            pool_policy_json = excluded.pool_policy_json,
            pool_json = excluded.pool_json,
            current_asset_id = excluded.current_asset_id,
            current_attempt_id = excluded.current_attempt_id,
            completed_asset_ids_json = excluded.completed_asset_ids_json,
            updated_at = excluded.updated_at",
        params![
            session.id,
            status_str(session.status),
            policy,
            pool,
            session.current_asset_id,
            session.current_attempt_id,
            completed,
            session.created_at,
            session.updated_at,
        ],
    )?;
    Ok(())
}

fn load(conn: &Connection, id: &str) -> DbResult<EndlessSession> {
    conn.query_row(
        "SELECT id, status, pool_policy_json, pool_json, current_asset_id, current_attempt_id,
                completed_asset_ids_json, created_at, updated_at
         FROM endless_sessions WHERE id = ?1",
        params![id],
        |r| {
            let policy_json: String = r.get(2)?;
            let pool_json: String = r.get(3)?;
            let completed_json: String = r.get(6)?;
            Ok(EndlessSession {
                id: r.get(0)?,
                status: parse_status(&r.get::<_, String>(1)?),
                pool_policy: serde_json::from_str(&policy_json).unwrap_or(EndlessPoolPolicy {
                    categories: vec![],
                    frequency_scope: None,
                    exclude_completed: true,
                }),
                pool: serde_json::from_str(&pool_json).unwrap_or_default(),
                current_asset_id: r.get(4)?,
                current_attempt_id: r.get(5)?,
                completed_asset_ids: serde_json::from_str(&completed_json).unwrap_or_default(),
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
            })
        },
    )
    .map_err(|_| DbError::Message(format!("endless not found: {id}")))
}

fn store_idempotent(conn: &Connection, key: &str, session: &EndlessSession) -> DbResult<()> {
    let json = serde_json::to_string(session).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO mode_idempotency (scope, idempotency_key, entity_id, response_json, created_at)
         VALUES ('endless.create', ?1, ?2, ?3, ?4)
         ON CONFLICT(scope, idempotency_key) DO UPDATE SET response_json = excluded.response_json",
        params![key, session.id, json, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn load_idempotent(conn: &Connection, key: &str) -> DbResult<Option<EndlessSession>> {
    let mut stmt = conn.prepare(
        "SELECT response_json FROM mode_idempotency WHERE scope = 'endless.create' AND idempotency_key = ?1",
    )?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        Ok(Some(
            serde_json::from_str(&json).map_err(|e| DbError::Message(e.to_string()))?,
        ))
    } else {
        Ok(None)
    }
}

fn store_submit_idempotent(
    conn: &Connection,
    key: &str,
    result: &SubmitEndlessResult,
) -> DbResult<()> {
    let json = serde_json::to_string(result).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO mode_idempotency (scope, idempotency_key, entity_id, response_json, created_at)
         VALUES ('endless.submit', ?1, ?2, ?3, ?4)
         ON CONFLICT(scope, idempotency_key) DO UPDATE SET response_json = excluded.response_json",
        params![
            key,
            result.session.id,
            json,
            chrono::Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

fn load_submit_idempotent(conn: &Connection, key: &str) -> DbResult<Option<SubmitEndlessResult>> {
    let mut stmt = conn.prepare(
        "SELECT response_json FROM mode_idempotency WHERE scope = 'endless.submit' AND idempotency_key = ?1",
    )?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        Ok(Some(
            serde_json::from_str(&json).map_err(|e| DbError::Message(e.to_string()))?,
        ))
    } else {
        Ok(None)
    }
}

/// Helper for UI pool filtering without session.
pub fn filter_pool_ids(
    pool: &[String],
    completed: &[String],
    exclude_completed: bool,
) -> Vec<String> {
    if !exclude_completed {
        return pool.to_vec();
    }
    pool.iter()
        .filter(|id| !completed.iter().any(|c| c == *id))
        .cloned()
        .collect()
}

#[allow(dead_code)]
fn _json_link() -> Value {
    json!({})
}
