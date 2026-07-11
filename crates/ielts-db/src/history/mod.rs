//! Unified history query, export, and detail loading (Phase 4).

use rusqlite::{params, params_from_iter, Connection, ToSql};

use ielts_domain::domain::{Activity, AttemptMode, AttemptStatus, ScoreScale};
use ielts_domain::dto::{
    ExportHistoryResult, HistoryDetailResponse, HistoryExportFormat, ListHistoryPage,
    ListHistoryQuery, WritingEvaluationV4,
};
use ielts_domain::{history_item_from_attempt, AttemptRecord, HistoryListItemVm};

use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, Default)]
struct HistoryFilter {
    activity: Option<Activity>,
    search: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    min_score: Option<f64>,
    max_score: Option<f64>,
}

impl From<&ListHistoryQuery> for HistoryFilter {
    fn from(q: &ListHistoryQuery) -> Self {
        Self {
            activity: q.activity,
            search: q
                .search
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            start_date: q.start_date.clone().filter(|s| !s.is_empty()),
            end_date: q.end_date.clone().filter(|s| !s.is_empty()),
            min_score: q.min_score,
            max_score: q.max_score,
        }
    }
}

fn build_where(filter: &HistoryFilter) -> (String, Vec<Box<dyn ToSql>>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut binds: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(activity) = filter.activity {
        clauses.push("activity = ?".into());
        binds.push(Box::new(activity_str(activity).to_string()));
    }
    if let Some(start) = &filter.start_date {
        clauses.push("date(COALESCE(submitted_at, started_at)) >= date(?)".into());
        binds.push(Box::new(start.clone()));
    }
    if let Some(end) = &filter.end_date {
        clauses.push("date(COALESCE(submitted_at, started_at)) <= date(?)".into());
        binds.push(Box::new(end.clone()));
    }
    if let Some(min) = filter.min_score {
        clauses.push("score_value IS NOT NULL AND score_value >= ?".into());
        binds.push(Box::new(min));
    }
    if let Some(max) = filter.max_score {
        clauses.push("score_value IS NOT NULL AND score_value <= ?".into());
        binds.push(Box::new(max));
    }
    if let Some(search) = &filter.search {
        clauses.push(
            "(IFNULL(title_snapshot,'') LIKE ? OR IFNULL(prompt_snapshot,'') LIKE ? OR IFNULL(content_text,'') LIKE ? OR id LIKE ?)"
                .into(),
        );
        let like = format!("%{search}%");
        binds.push(Box::new(like.clone()));
        binds.push(Box::new(like.clone()));
        binds.push(Box::new(like.clone()));
        binds.push(Box::new(like));
    }

    let sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    (sql, binds)
}

pub fn list_history(conn: &Connection, query: &ListHistoryQuery) -> DbResult<ListHistoryPage> {
    let filter = HistoryFilter::from(query);
    let (where_sql, binds) = build_where(&filter);
    let limit = query.limit.max(1).min(200);
    let offset = query.offset;

    let count_sql = format!("SELECT COUNT(*) FROM attempts {where_sql}");
    let total: u32 = {
        let mut stmt = conn.prepare(&count_sql)?;
        let params_iter = params_from_iter(binds.iter().map(|b| b.as_ref()));
        stmt.query_row(params_iter, |r| r.get::<_, i64>(0))? as u32
    };

    let list_sql = format!(
        "SELECT id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
                duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
                prompt_snapshot, content_text, schema_version
         FROM attempts
         {where_sql}
         ORDER BY COALESCE(submitted_at, started_at) DESC, id DESC
         LIMIT ? OFFSET ?"
    );

    let mut all_binds = binds;
    all_binds.push(Box::new(limit as i64));
    all_binds.push(Box::new(offset as i64));

    let mut stmt = conn.prepare(&list_sql)?;
    let params_iter = params_from_iter(all_binds.iter().map(|b| b.as_ref()));
    let rows = stmt.query_map(params_iter, map_attempt_row)?;

    let mut items = Vec::new();
    for row in rows {
        let attempt = row?;
        items.push(history_item_from_attempt(&attempt));
    }

    let next_cursor = if offset + limit < total {
        Some(format!("{}", offset + limit))
    } else {
        None
    };

    Ok(ListHistoryPage {
        items,
        total,
        limit,
        offset,
        next_cursor,
    })
}

pub fn get_history_detail(conn: &Connection, attempt_id: &str) -> DbResult<HistoryDetailResponse> {
    let attempt = load_attempt(conn, attempt_id)?;
    let summary = history_item_from_attempt(&attempt);
    let evaluation = if attempt.activity == Activity::Writing {
        load_evaluation(conn, attempt_id)?
    } else {
        None
    };
    Ok(HistoryDetailResponse {
        summary,
        attempt,
        evaluation,
    })
}

pub fn export_history(
    conn: &Connection,
    format: HistoryExportFormat,
    query: Option<&ListHistoryQuery>,
) -> DbResult<ExportHistoryResult> {
    let mut q = query.cloned().unwrap_or(ListHistoryQuery {
        activity: None,
        limit: 10_000,
        offset: 0,
        cursor: None,
        search: None,
        start_date: None,
        end_date: None,
        min_score: None,
        max_score: None,
    });
    q.limit = q.limit.max(1).min(50_000);
    q.offset = 0;
    let page = list_history(conn, &q)?;
    let body = match format {
        HistoryExportFormat::Csv => render_csv(&page.items),
        HistoryExportFormat::Markdown => render_markdown(&page.items),
        HistoryExportFormat::Json => serde_json::to_string_pretty(&page.items)
            .map_err(|e| DbError::Message(e.to_string()))?,
    };
    Ok(ExportHistoryResult {
        format,
        body,
        record_count: page.items.len() as u32,
    })
}

pub fn delete_attempt(conn: &Connection, attempt_id: &str) -> DbResult<bool> {
    let n = conn.execute("DELETE FROM attempts WHERE id = ?1", params![attempt_id])?;
    Ok(n > 0)
}

fn render_csv(items: &[HistoryListItemVm]) -> String {
    let mut out = String::from("id,activity,title,status,mode,submitted_at,duration_ms,score_value,score_scale,score_display\n");
    for item in items {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&item.id),
            activity_str(item.activity),
            csv_escape(&item.title),
            format!("{:?}", item.status).to_ascii_lowercase(),
            mode_str(item.mode),
            csv_escape(item.submitted_at.as_deref().unwrap_or("")),
            item.duration_ms,
            item.score_value.map(|v| v.to_string()).unwrap_or_default(),
            item.score_scale
                .map(|s| match s {
                    ScoreScale::Ratio => "ratio",
                    ScoreScale::Band9 => "band9",
                })
                .unwrap_or(""),
            csv_escape(&item.score_display),
        ));
    }
    out
}

fn render_markdown(items: &[HistoryListItemVm]) -> String {
    let mut out = String::from("# IELTS Practice History\n\n");
    out.push_str("| Activity | Title | Score | Submitted | Duration |\n");
    out.push_str("|---|---|---|---|---|\n");
    for item in items {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} ms |\n",
            activity_str(item.activity),
            md_escape(&item.title),
            item.score_display,
            item.submitted_at.as_deref().unwrap_or("—"),
            item.duration_ms
        ));
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|")
}

fn map_attempt_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttemptRecord> {
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
}

fn load_attempt(conn: &Connection, id: &str) -> DbResult<AttemptRecord> {
    let mut attempt = conn.query_row(
        "SELECT id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
                duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
                prompt_snapshot, content_text, schema_version
         FROM attempts WHERE id = ?1",
        params![id],
        map_attempt_row,
    )?;

    let mut stmt = conn.prepare(
        "SELECT question_id, answer_json, is_correct, weight, question_kind, change_count, visit_count,
                elapsed_ms, marked, answered_at
         FROM attempt_answers WHERE attempt_id = ?1 ORDER BY question_id",
    )?;
    let answers = stmt.query_map(params![id], |row| {
        let answer_json: String = row.get(1)?;
        let answer = serde_json::from_str(&answer_json).unwrap_or(serde_json::Value::Null);
        Ok(ielts_domain::AttemptAnswer {
            question_id: row.get(0)?,
            answer,
            is_correct: row
                .get::<_, Option<i64>>(2)?
                .map(|v| v != 0),
            weight: row.get(3)?,
            question_kind: row.get(4)?,
            change_count: row.get::<_, i64>(5)? as u32,
            visit_count: row.get::<_, i64>(6)? as u32,
            elapsed_ms: row.get::<_, i64>(7)? as u64,
            marked: row.get::<_, i64>(8)? != 0,
            answered_at: row.get(9)?,
        })
    })?;
    for a in answers {
        attempt.answers.push(a?);
    }

    let mut ann_stmt = conn.prepare(
        "SELECT id, attempt_id, asset_id, scope, question_id, kind, anchor_json, note_text
         FROM attempt_annotations WHERE attempt_id = ?1",
    )?;
    let anns = ann_stmt.query_map(params![id], |row| {
        let anchor_json: String = row.get(6)?;
        let anchor = serde_json::from_str(&anchor_json).unwrap_or(serde_json::json!({}));
        Ok(ielts_domain::AttemptAnnotationDto {
            id: row.get(0)?,
            attempt_id: row.get::<_, Option<String>>(1)?,
            asset_id: row.get(2)?,
            scope: row.get(3)?,
            question_id: row.get(4)?,
            kind: row.get(5)?,
            anchor,
            note_text: row.get(7)?,
        })
    })?;
    for a in anns {
        attempt.annotations.push(a?);
    }

    Ok(attempt)
}

fn load_evaluation(conn: &Connection, attempt_id: &str) -> DbResult<Option<WritingEvaluationV4>> {
    let result: Result<String, rusqlite::Error> = conn.query_row(
        "SELECT result_json FROM writing_evaluations WHERE attempt_id = ?1",
        params![attempt_id],
        |r| r.get(0),
    );
    match result {
        Ok(json) => {
            let v: WritingEvaluationV4 = serde_json::from_str(&json)
                .map_err(|e| DbError::Message(format!("evaluation parse: {e}")))?;
            Ok(Some(v))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
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

fn parse_status(raw: &str) -> AttemptStatus {
    use AttemptStatus::*;
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
