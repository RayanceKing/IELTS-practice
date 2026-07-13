//! Reading suite state machine (Phase 7).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use ielts_domain::domain::{AttemptMode, SuiteFlowMode, SuiteStatus};

use crate::modes::timer::{TimerMode, TimerState};
use crate::reading::attempt::{submit_reading_attempt, ReadingSubmitCommand, ReadingSubmitResult};
use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassageStatus {
    Pending,
    Active,
    Submitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrequencyScope {
    High,
    HighMedium,
    All,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SuitePassageEntry {
    pub index: u32,
    pub asset_id: String,
    pub exam_id: String,
    pub title: String,
    pub category: String,
    pub status: PassageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// Review routes historically used sessionId; keep both equal to attempt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SuiteAggregate {
    pub submitted_passages: u32,
    pub total_passages: u32,
    pub correct: f64,
    pub total_questions: f64,
    pub accuracy: f64,
    pub percentage: f64,
    pub duration: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSuiteSession {
    pub session_id: String,
    pub activity: String,
    pub practice_mode: String,
    pub status: SuiteStatus,
    pub flow_mode: SuiteFlowMode,
    pub frequency_scope: FrequencyScope,
    pub timer: TimerState,
    pub current_index: u32,
    pub total_passages: u32,
    pub sequence: Vec<SuitePassageEntry>,
    pub aggregate: SuiteAggregate,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSuiteCommand {
    #[serde(default)]
    pub flow_mode: Option<String>,
    #[serde(default)]
    pub frequency_scope: Option<String>,
    #[serde(default)]
    pub seed: Option<String>,
    /// Ordered asset ids for custom sequence (must be length 3: P1/P2/P3).
    #[serde(default)]
    pub sequence: Vec<SuiteAssetSeed>,
    #[serde(default)]
    pub timer: Option<TimerState>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuiteAssetSeed {
    pub asset_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSuitePassageCommand {
    pub suite_id: String,
    pub asset_id: String,
    pub payload: Value,
    #[serde(default)]
    pub answers: Value,
    #[serde(default)]
    pub marked_questions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_snapshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timer_snapshot: Option<TimerState>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSuitePassageResult {
    pub suite_session: ReadingSuiteSession,
    pub submission: ReadingSubmitResult,
}

fn normalize_flow(raw: Option<&str>) -> SuiteFlowMode {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("classic") => SuiteFlowMode::Classic,
        Some("stationary") => SuiteFlowMode::Stationary,
        _ => SuiteFlowMode::Simulation,
    }
}

fn flow_str(m: SuiteFlowMode) -> &'static str {
    match m {
        SuiteFlowMode::Classic => "classic",
        SuiteFlowMode::Stationary => "stationary",
        SuiteFlowMode::Simulation => "simulation",
    }
}

fn parse_flow(raw: &str) -> SuiteFlowMode {
    normalize_flow(Some(raw))
}

fn normalize_freq(raw: Option<&str>) -> FrequencyScope {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("high") => FrequencyScope::High,
        Some("high_medium") | Some("high-medium") => FrequencyScope::HighMedium,
        Some("custom") => FrequencyScope::Custom,
        _ => FrequencyScope::All,
    }
}

fn freq_str(f: FrequencyScope) -> &'static str {
    match f {
        FrequencyScope::High => "high",
        FrequencyScope::HighMedium => "high_medium",
        FrequencyScope::All => "all",
        FrequencyScope::Custom => "custom",
    }
}

fn parse_freq(raw: &str) -> FrequencyScope {
    normalize_freq(Some(raw))
}

fn status_str(s: SuiteStatus) -> &'static str {
    match s {
        SuiteStatus::Active => "active",
        SuiteStatus::Completed => "completed",
        SuiteStatus::Cancelled => "cancelled",
        SuiteStatus::Interrupted => "interrupted",
    }
}

fn parse_status(raw: &str) -> SuiteStatus {
    match raw {
        "completed" => SuiteStatus::Completed,
        "cancelled" => SuiteStatus::Cancelled,
        "interrupted" => SuiteStatus::Interrupted,
        _ => SuiteStatus::Active,
    }
}

fn passage_status_str(s: PassageStatus) -> &'static str {
    match s {
        PassageStatus::Pending => "pending",
        PassageStatus::Active => "active",
        PassageStatus::Submitted => "submitted",
    }
}

fn parse_passage_status(raw: &str) -> PassageStatus {
    match raw {
        "active" => PassageStatus::Active,
        "submitted" => PassageStatus::Submitted,
        _ => PassageStatus::Pending,
    }
}

fn empty_aggregate(total: u32) -> SuiteAggregate {
    SuiteAggregate {
        submitted_passages: 0,
        total_passages: total,
        correct: 0.0,
        total_questions: 0.0,
        accuracy: 0.0,
        percentage: 0.0,
        duration: 0,
    }
}

fn recompute_aggregate(sequence: &[SuitePassageEntry]) -> SuiteAggregate {
    let mut correct = 0.0;
    let mut total_q = 0.0;
    let mut duration = 0u64;
    let mut submitted = 0u32;
    for entry in sequence {
        if entry.status != PassageStatus::Submitted {
            continue;
        }
        submitted += 1;
        if let Some(score) = &entry.score_info {
            correct += score.get("correct").and_then(|v| v.as_f64()).unwrap_or(0.0);
            total_q += score
                .get("totalQuestions")
                .or_else(|| score.get("total"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            duration += score.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
        }
    }
    let accuracy = if total_q > 0.0 {
        correct / total_q
    } else {
        0.0
    };
    SuiteAggregate {
        submitted_passages: submitted,
        total_passages: sequence.len() as u32,
        correct,
        total_questions: total_q,
        accuracy,
        percentage: (accuracy * 100.0).round(),
        duration,
    }
}

/// Classic: free navigation after create (all pending except first active).
/// Simulation: sequential only (enforced on submit).
/// Stationary: same sequential submit; UI may freeze navigation (policy flag only).
pub fn create_suite_session(
    conn: &Connection,
    cmd: &CreateSuiteCommand,
) -> DbResult<ReadingSuiteSession> {
    if cmd.sequence.is_empty() {
        return Err(DbError::Validation(
            "suite sequence required (provide P1/P2/P3 asset seeds)".into(),
        ));
    }
    if let Some(key) = cmd.idempotency_key.as_deref() {
        if !key.trim().is_empty() {
            if let Some(prev) = load_idempotent(conn, "suite.create", key)? {
                return Ok(prev);
            }
        }
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let now = chrono::Utc::now().to_rfc3339();
    let session_id = format!("suite-{}", Uuid::new_v4());
    let flow = normalize_flow(cmd.flow_mode.as_deref());
    let freq = normalize_freq(cmd.frequency_scope.as_deref());
    let timer = cmd
        .timer
        .clone()
        .unwrap_or_else(|| TimerState::new_suite(now_ms))
        .normalize(now_ms);

    let sequence: Vec<SuitePassageEntry> = cmd
        .sequence
        .iter()
        .enumerate()
        .map(|(i, seed)| {
            let cat = seed
                .category
                .clone()
                .unwrap_or_else(|| format!("P{}", i + 1));
            SuitePassageEntry {
                index: i as u32,
                asset_id: seed.asset_id.clone(),
                exam_id: seed.asset_id.clone(),
                title: seed.title.clone().unwrap_or_else(|| seed.asset_id.clone()),
                category: cat,
                status: if i == 0 {
                    PassageStatus::Active
                } else {
                    PassageStatus::Pending
                },
                attempt_id: None,
                session_id: None,
                submitted_at: None,
                score_info: None,
            }
        })
        .collect();

    let session = ReadingSuiteSession {
        session_id: session_id.clone(),
        activity: "reading".into(),
        practice_mode: "suite".into(),
        status: SuiteStatus::Active,
        flow_mode: flow,
        frequency_scope: freq,
        timer,
        current_index: 0,
        total_passages: sequence.len() as u32,
        sequence,
        aggregate: empty_aggregate(cmd.sequence.len() as u32),
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_at: None,
    };

    persist_suite(conn, &session)?;
    if let Some(key) = cmd.idempotency_key.as_deref() {
        if !key.trim().is_empty() {
            store_idempotent(conn, "suite.create", key, &session_id, &session)?;
        }
    }
    Ok(session)
}

pub fn get_suite_session(conn: &Connection, suite_id: &str) -> DbResult<ReadingSuiteSession> {
    load_suite(conn, suite_id)
}

pub fn submit_suite_passage(
    conn: &Connection,
    cmd: &SubmitSuitePassageCommand,
) -> DbResult<SubmitSuitePassageResult> {
    if cmd.idempotency_key.trim().is_empty() {
        return Err(DbError::Validation("idempotency_key required".into()));
    }
    if let Some(prev) = load_idempotent_submit(conn, &cmd.idempotency_key)? {
        return Ok(prev);
    }

    let mut session = load_suite(conn, &cmd.suite_id)?;
    if session.status != SuiteStatus::Active {
        return Err(DbError::Validation("suite is not active".into()));
    }

    let passage_index = session
        .sequence
        .iter()
        .position(|e| e.asset_id == cmd.asset_id || e.exam_id == cmd.asset_id)
        .ok_or_else(|| DbError::Validation(format!("asset not in suite: {}", cmd.asset_id)))?;

    // Simulation/stationary: must submit current index only.
    // Classic still advances from current_index for aggregate correctness.
    if session.flow_mode != SuiteFlowMode::Classic && passage_index as u32 != session.current_index
    {
        return Err(DbError::Validation(
            "submit the active suite passage before moving on".into(),
        ));
    }
    if session.flow_mode == SuiteFlowMode::Classic {
        // allow any pending/active that is not yet submitted
        if session.sequence[passage_index].status == PassageStatus::Submitted {
            return Err(DbError::Validation("passage already submitted".into()));
        }
    } else if session.sequence[passage_index].status == PassageStatus::Submitted {
        return Err(DbError::Validation("passage already submitted".into()));
    }

    let attempt_id = format!("reading-{}-p{}", session.session_id, passage_index + 1);
    let submit = submit_reading_attempt(
        conn,
        &ReadingSubmitCommand {
            attempt_id: attempt_id.clone(),
            asset_id: cmd.asset_id.clone(),
            payload: cmd.payload.clone(),
            answers: cmd.answers.clone(),
            marked_questions: cmd.marked_questions.clone(),
            duration_ms: cmd.duration_ms,
            title_snapshot: cmd.title_snapshot.clone(),
            idempotency_key: format!("suite-pass-{}", cmd.idempotency_key),
        },
    )?;

    // Tag attempt as suite mode
    conn.execute(
        "UPDATE attempts SET mode = ?1, suite_id = ?2 WHERE id = ?3",
        params![
            match AttemptMode::Suite {
                AttemptMode::Suite => "suite",
                _ => "suite",
            },
            session.session_id,
            attempt_id
        ],
    )?;

    session.timer = session.timer.merge_snapshot(cmd.timer_snapshot.as_ref());

    let score_info = json!({
        "correct": submit.score.correct,
        "total": submit.score.total,
        "totalQuestions": submit.score.total,
        "accuracy": submit.score.accuracy,
        "percentage": submit.score.percentage,
        "duration": submit.attempt.duration_ms / 1000,
    });

    {
        let passage = &mut session.sequence[passage_index];
        passage.status = PassageStatus::Submitted;
        passage.attempt_id = Some(attempt_id.clone());
        passage.session_id = Some(attempt_id);
        passage.submitted_at = submit.attempt.submitted_at.clone();
        passage.score_info = Some(score_info);
    }

    let next = (passage_index + 1) as u32;
    if next < session.sequence.len() as u32 {
        session.sequence[next as usize].status = PassageStatus::Active;
        session.current_index = next;
    } else {
        session.current_index = passage_index as u32;
        session.status = SuiteStatus::Completed;
        session.completed_at = submit.attempt.submitted_at.clone();
    }
    session.aggregate = recompute_aggregate(&session.sequence);
    session.updated_at = chrono::Utc::now().to_rfc3339();

    persist_suite(conn, &session)?;

    let result = SubmitSuitePassageResult {
        suite_session: session.clone(),
        submission: submit,
    };
    store_idempotent_submit(conn, &cmd.idempotency_key, &session.session_id, &result)?;
    Ok(result)
}

pub fn cancel_suite(conn: &Connection, suite_id: &str) -> DbResult<ReadingSuiteSession> {
    let mut session = load_suite(conn, suite_id)?;
    if session.status == SuiteStatus::Active {
        session.status = SuiteStatus::Cancelled;
        session.updated_at = chrono::Utc::now().to_rfc3339();
        persist_suite(conn, &session)?;
    }
    Ok(session)
}

fn persist_suite(conn: &Connection, session: &ReadingSuiteSession) -> DbResult<()> {
    let timer_json =
        serde_json::to_string(&session.timer).map_err(|e| DbError::Message(e.to_string()))?;
    let agg_json =
        serde_json::to_string(&session.aggregate).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO reading_suites (
            id, mode, flow_mode, status, current_index, timer_policy_json, created_at, updated_at,
            frequency_scope, seed, aggregate_json, completed_at, timer_state_json
         ) VALUES (?1, 'suite', ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, ?5)
         ON CONFLICT(id) DO UPDATE SET
            flow_mode = excluded.flow_mode,
            status = excluded.status,
            current_index = excluded.current_index,
            timer_policy_json = excluded.timer_policy_json,
            timer_state_json = excluded.timer_state_json,
            updated_at = excluded.updated_at,
            frequency_scope = excluded.frequency_scope,
            aggregate_json = excluded.aggregate_json,
            completed_at = excluded.completed_at",
        params![
            session.session_id,
            flow_str(session.flow_mode),
            status_str(session.status),
            session.current_index as i64,
            timer_json,
            session.created_at,
            session.updated_at,
            freq_str(session.frequency_scope),
            agg_json,
            session.completed_at,
        ],
    )?;

    conn.execute(
        "DELETE FROM reading_suite_items WHERE suite_id = ?1",
        params![session.session_id],
    )?;
    for entry in &session.sequence {
        let score_json = entry.score_info.as_ref().map(|v| v.to_string());
        conn.execute(
            "INSERT INTO reading_suite_items (
                suite_id, item_index, asset_id, attempt_id, status, title, category, submitted_at, score_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session.session_id,
                entry.index as i64,
                entry.asset_id,
                entry.attempt_id,
                passage_status_str(entry.status),
                entry.title,
                entry.category,
                entry.submitted_at,
                score_json,
            ],
        )?;
    }
    Ok(())
}

fn load_suite(conn: &Connection, suite_id: &str) -> DbResult<ReadingSuiteSession> {
    let row = conn.query_row(
        "SELECT id, flow_mode, status, current_index, created_at, updated_at,
                frequency_scope, aggregate_json, completed_at,
                COALESCE(timer_state_json, timer_policy_json)
         FROM reading_suites WHERE id = ?1",
        params![suite_id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?,
            ))
        },
    );
    let (
        id,
        flow_mode,
        status,
        current_index,
        created_at,
        updated_at,
        frequency_scope,
        aggregate_json,
        completed_at,
        timer_json,
    ) = row.map_err(|_| DbError::Message(format!("suite not found: {suite_id}")))?;

    let timer: TimerState = timer_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_else(|| {
            let fallback = chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|d| d.timestamp_millis())
                .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());
            TimerState::new_suite(fallback)
        });

    let mut stmt = conn.prepare(
        "SELECT item_index, asset_id, attempt_id, status, title, category, submitted_at, score_json
         FROM reading_suite_items WHERE suite_id = ?1 ORDER BY item_index ASC",
    )?;
    let rows = stmt.query_map(params![suite_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<String>>(7)?,
        ))
    })?;

    let mut sequence = Vec::new();
    for row in rows {
        let (idx, asset_id, attempt_id, st, title, category, submitted_at, score_json) = row?;
        sequence.push(SuitePassageEntry {
            index: idx as u32,
            exam_id: asset_id.clone(),
            asset_id,
            title: title.unwrap_or_default(),
            category: category.unwrap_or_default(),
            status: parse_passage_status(&st),
            session_id: attempt_id.clone(),
            attempt_id,
            submitted_at,
            score_info: score_json.and_then(|s| serde_json::from_str(&s).ok()),
        });
    }

    let aggregate = aggregate_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_else(|| recompute_aggregate(&sequence));

    Ok(ReadingSuiteSession {
        session_id: id,
        activity: "reading".into(),
        practice_mode: "suite".into(),
        status: parse_status(&status),
        flow_mode: parse_flow(&flow_mode),
        frequency_scope: parse_freq(&frequency_scope),
        timer,
        current_index: current_index as u32,
        total_passages: sequence.len() as u32,
        sequence,
        aggregate,
        created_at,
        updated_at,
        completed_at,
    })
}

fn store_idempotent(
    conn: &Connection,
    scope: &str,
    key: &str,
    entity_id: &str,
    session: &ReadingSuiteSession,
) -> DbResult<()> {
    let json = serde_json::to_string(session).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO mode_idempotency (scope, idempotency_key, entity_id, response_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(scope, idempotency_key) DO UPDATE SET response_json = excluded.response_json",
        params![
            scope,
            key,
            entity_id,
            json,
            chrono::Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

fn load_idempotent(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> DbResult<Option<ReadingSuiteSession>> {
    let mut stmt = conn.prepare(
        "SELECT response_json FROM mode_idempotency WHERE scope = ?1 AND idempotency_key = ?2",
    )?;
    let mut rows = stmt.query(params![scope, key])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        let s = serde_json::from_str(&json).map_err(|e| DbError::Message(e.to_string()))?;
        Ok(Some(s))
    } else {
        Ok(None)
    }
}

fn store_idempotent_submit(
    conn: &Connection,
    key: &str,
    entity_id: &str,
    result: &SubmitSuitePassageResult,
) -> DbResult<()> {
    let json = serde_json::to_string(result).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO mode_idempotency (scope, idempotency_key, entity_id, response_json, created_at)
         VALUES ('suite.submit', ?1, ?2, ?3, ?4)
         ON CONFLICT(scope, idempotency_key) DO UPDATE SET response_json = excluded.response_json",
        params![key, entity_id, json, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn load_idempotent_submit(
    conn: &Connection,
    key: &str,
) -> DbResult<Option<SubmitSuitePassageResult>> {
    let mut stmt = conn.prepare(
        "SELECT response_json FROM mode_idempotency WHERE scope = 'suite.submit' AND idempotency_key = ?1",
    )?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        let s = serde_json::from_str(&json).map_err(|e| DbError::Message(e.to_string()))?;
        Ok(Some(s))
    } else {
        Ok(None)
    }
}

// silence unused import warning if TimerMode only used in tests elsewhere
#[allow(dead_code)]
fn _timer_mode_link() -> TimerMode {
    TimerMode::Elapsed
}
