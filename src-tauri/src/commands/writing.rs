//! Writing draft + evaluation Tauri commands (Phase 5).

use ielts_domain::dto::{CommandResponse, SaveDraftCommand, SubmitAttemptCommand};
use ielts_domain::ErrorEnvelope;
use tauri::State;

use crate::app::state::AppDb;
use ielts_db::{
    get_writing_draft, list_events, load_evaluation_for_attempt, request_cancel, save_writing_draft,
    start_evaluation, submit_writing_attempt, DeterministicProvider, EvaluationEvent,
    EvaluationRunResult, StartEvaluationCommand, WritingDraft,
};

fn map_err(err: ielts_db::DbError) -> ErrorEnvelope {
    ErrorEnvelope {
        code: "writing.error".into(),
        message: err.to_string(),
        retryable: false,
        context: None,
        cause_id: None,
    }
}

#[tauri::command]
pub fn writing_save_draft(
    db: State<'_, AppDb>,
    cmd: SaveDraftCommand,
) -> CommandResponse<WritingDraft> {
    match db.with_conn(|conn| save_writing_draft(conn, &cmd)) {
        Ok(d) => CommandResponse::success(d),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn writing_get_draft(
    db: State<'_, AppDb>,
    attempt_id: String,
) -> CommandResponse<Option<WritingDraft>> {
    match db.with_conn(|conn| get_writing_draft(conn, &attempt_id)) {
        Ok(d) => CommandResponse::success(d),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn writing_submit_attempt(
    db: State<'_, AppDb>,
    cmd: SubmitAttemptCommand,
) -> CommandResponse<ielts_domain::dto::AttemptRecord> {
    match db.with_conn(|conn| submit_writing_attempt(conn, &cmd)) {
        Ok(a) => CommandResponse::success(a),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn writing_start_evaluation(
    db: State<'_, AppDb>,
    cmd: StartEvaluationCommand,
) -> CommandResponse<EvaluationRunResult> {
    // Phase 5: deterministic local provider. Real AI providers land with secret vault wiring.
    let provider = DeterministicProvider;
    match db.with_conn(|conn| start_evaluation(conn, &cmd, &provider)) {
        Ok(r) => CommandResponse::success(r),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn writing_list_evaluation_events(
    db: State<'_, AppDb>,
    evaluation_id: String,
    after_sequence: Option<u32>,
) -> CommandResponse<Vec<EvaluationEvent>> {
    let after = after_sequence.unwrap_or(0);
    match db.with_conn(|conn| list_events(conn, &evaluation_id, after)) {
        Ok(events) => CommandResponse::success(events),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn writing_cancel_evaluation(
    db: State<'_, AppDb>,
    evaluation_id: String,
) -> CommandResponse<bool> {
    match db.with_conn(|conn| request_cancel(conn, &evaluation_id)) {
        Ok(ok) => CommandResponse::success(ok),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn writing_get_evaluation(
    db: State<'_, AppDb>,
    attempt_id: String,
) -> CommandResponse<Option<ielts_domain::dto::WritingEvaluationV4>> {
    match db.with_conn(|conn| load_evaluation_for_attempt(conn, &attempt_id)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}
