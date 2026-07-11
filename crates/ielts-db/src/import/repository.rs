use rusqlite::{params, Connection};
use serde_json::Value;

use ielts_domain::adapters::{evaluation_v3_to_v4, reading_submission_to_attempt};
use ielts_domain::domain::{Activity, AttemptMode, ScoreScale};
use ielts_domain::dto::{AttemptRecord, WritingEvaluationV4};

use crate::sqlite::{DbError, DbResult};

pub fn upsert_attempt(conn: &Connection, attempt: &AttemptRecord) -> DbResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO attempts (
            id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
            duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
            prompt_snapshot, content_text, schema_version, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20
        )
        ON CONFLICT(id) DO UPDATE SET
            status=excluded.status,
            submitted_at=excluded.submitted_at,
            completed_at=excluded.completed_at,
            duration_ms=excluded.duration_ms,
            score_value=excluded.score_value,
            score_scale=excluded.score_scale,
            correct_count=excluded.correct_count,
            question_count=excluded.question_count,
            title_snapshot=excluded.title_snapshot,
            prompt_snapshot=excluded.prompt_snapshot,
            content_text=excluded.content_text,
            updated_at=excluded.updated_at
        ",
        params![
            attempt.id,
            activity_str(attempt.activity),
            attempt.asset_id,
            mode_str(attempt.mode),
            attempt.suite_id,
            format!("{:?}", attempt.status).to_ascii_lowercase(),
            attempt.started_at,
            attempt.submitted_at,
            attempt.completed_at,
            attempt.duration_ms as i64,
            attempt.score_value,
            attempt
                .score_scale
                .map(|s| match s {
                    ScoreScale::Ratio => "ratio",
                    ScoreScale::Band9 => "band9",
                }),
            attempt.correct_count,
            attempt.question_count.map(|v| v as i64),
            attempt.title_snapshot,
            attempt.prompt_snapshot,
            attempt.content_text,
            attempt.schema_version as i64,
            now,
            now,
        ],
    )?;

    conn.execute(
        "DELETE FROM attempt_answers WHERE attempt_id = ?1",
        params![attempt.id],
    )?;
    for answer in &attempt.answers {
        conn.execute(
            "INSERT INTO attempt_answers (
                attempt_id, question_id, answer_json, is_correct, weight, question_kind,
                change_count, visit_count, elapsed_ms, marked, answered_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                attempt.id,
                answer.question_id,
                answer.answer.to_string(),
                answer.is_correct.map(|b| if b { 1 } else { 0 }),
                answer.weight,
                answer.question_kind,
                answer.change_count as i64,
                answer.visit_count as i64,
                answer.elapsed_ms as i64,
                if answer.marked { 1 } else { 0 },
                answer.answered_at,
            ],
        )?;
    }

    for ann in &attempt.annotations {
        conn.execute(
            "INSERT OR REPLACE INTO attempt_annotations (
                id, attempt_id, asset_id, scope, question_id, kind, anchor_json, note_text, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                ann.id,
                ann.attempt_id,
                ann.asset_id,
                ann.scope,
                ann.question_id,
                ann.kind,
                ann.anchor.to_string(),
                ann.note_text,
                now,
                now,
            ],
        )?;
    }

    Ok(())
}

pub fn upsert_writing_evaluation(
    conn: &Connection,
    attempt_id: &str,
    evaluation: &WritingEvaluationV4,
) -> DbResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let result_json = serde_json::to_string(evaluation)
        .map_err(|e| DbError::Import(e.to_string()))?;
    let degradation_json = evaluation
        .degradation
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| DbError::Import(e.to_string()))?;
    let error_json = evaluation
        .error
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| DbError::Import(e.to_string()))?;

    // Import path replaces the latest evaluation for this attempt (not all lineage rows).
    // Multiple evaluations from retry lineage are created by the evaluation service, not import.
    conn.execute(
        "DELETE FROM writing_evaluations WHERE attempt_id = ?1",
        params![attempt_id],
    )?;
    conn.execute(
        "INSERT INTO writing_evaluations (
            id, attempt_id, status, stage, provider_id, model, rubric_version, prompt_version,
            result_json, degradation_json, error_json, started_at, completed_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, NULL, NULL, 'ielts-v1', 'default',
            ?5, ?6, ?7, NULL, NULL, ?8
        )",
        params![
            id,
            attempt_id,
            format!("{:?}", evaluation.status).to_ascii_lowercase(),
            format!("{:?}", evaluation.stage).to_ascii_lowercase(),
            result_json,
            degradation_json,
            error_json,
            now,
        ],
    )?;
    Ok(())
}

pub fn ensure_asset_stub(
    conn: &Connection,
    asset_id: &str,
    activity: Activity,
    title: &str,
    source_key: Option<&str>,
) -> DbResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO practice_assets (
            id, activity, source_kind, source_key, title, category, difficulty, frequency,
            content_ref, schema_version, fingerprint, pdf_only, metadata_json, created_at, updated_at
        ) VALUES (
            ?1, ?2, 'imported', ?3, ?4, NULL, NULL, NULL,
            NULL, 2, ?5, 0, NULL, ?6, ?7
        )",
        params![
            asset_id,
            activity_str(activity),
            source_key,
            title,
            format!("import:{asset_id}"),
            now,
            now,
        ],
    )?;
    Ok(())
}

pub fn count_attempts(conn: &Connection) -> DbResult<i64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get(0))?;
    Ok(n)
}

pub fn list_history_view_models(conn: &Connection) -> DbResult<Vec<ielts_domain::HistoryListItemVm>> {
    let mut stmt = conn.prepare(
        "SELECT id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
                duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
                prompt_snapshot, content_text, schema_version
         FROM attempts
         ORDER BY COALESCE(submitted_at, started_at) DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AttemptRecord {
            schema_version: row.get::<_, i64>(17)? as u32,
            id: row.get(0)?,
            activity: parse_activity(&row.get::<_, String>(1)?),
            asset_id: row.get(2)?,
            mode: parse_mode(&row.get::<_, String>(3)?),
            suite_id: row.get(4)?,
            status: parse_status(&row.get::<_, String>(5)?),
            started_at: row.get(6)?,
            submitted_at: row.get(7)?,
            completed_at: row.get(8)?,
            duration_ms: row.get::<_, i64>(9)? as u64,
            score_value: row.get(10)?,
            score_scale: row
                .get::<_, Option<String>>(11)?
                .and_then(|s| match s.as_str() {
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
    })?;

    let mut out = Vec::new();
    for row in rows {
        let attempt = row?;
        out.push(ielts_domain::history_item_from_attempt(&attempt));
    }
    Ok(out)
}

pub fn import_evaluation_json(conn: &Connection, attempt_id: &str, raw: &Value) -> DbResult<()> {
    let v4 = evaluation_v3_to_v4(raw).map_err(|e| DbError::Import(e.to_string()))?;
    upsert_writing_evaluation(conn, attempt_id, &v4)
}

pub fn import_reading_submission_json(conn: &Connection, raw: &Value) -> DbResult<String> {
    let attempt =
        reading_submission_to_attempt(raw).map_err(|e| DbError::Import(e.to_string()))?;
    if let Some(asset_id) = attempt.asset_id.as_deref() {
        ensure_asset_stub(
            conn,
            asset_id,
            Activity::Reading,
            attempt
                .title_snapshot
                .as_deref()
                .unwrap_or("Imported reading"),
            Some(asset_id),
        )?;
    }
    let id = attempt.id.clone();
    upsert_attempt(conn, &attempt)?;
    Ok(id)
}

fn activity_str(activity: Activity) -> &'static str {
    match activity {
        Activity::Reading => "reading",
        Activity::Writing => "writing",
    }
}

fn mode_str(mode: AttemptMode) -> &'static str {
    match mode {
        AttemptMode::Single => "single",
        AttemptMode::Suite => "suite",
        AttemptMode::Endless => "endless",
        AttemptMode::Memorize => "memorize",
        AttemptMode::Freeform => "freeform",
        AttemptMode::Bank => "bank",
    }
}

fn parse_activity(raw: &str) -> Activity {
    match raw {
        "writing" => Activity::Writing,
        _ => Activity::Reading,
    }
}

fn parse_mode(raw: &str) -> AttemptMode {
    match raw {
        "suite" => AttemptMode::Suite,
        "endless" => AttemptMode::Endless,
        "memorize" => AttemptMode::Memorize,
        "freeform" => AttemptMode::Freeform,
        "bank" => AttemptMode::Bank,
        _ => AttemptMode::Single,
    }
}

fn parse_status(raw: &str) -> ielts_domain::AttemptStatus {
    use ielts_domain::AttemptStatus::*;
    match raw {
        "draft" => Draft,
        "active" => Active,
        "submitted" => Submitted,
        "reviewing" => Reviewing,
        "cancelled" => Cancelled,
        "failed" => Failed,
        "interrupted" => Interrupted,
        _ => Completed,
    }
}
