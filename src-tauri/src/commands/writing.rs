//! Writing draft + evaluation Tauri commands (Phase 5).

use ielts_domain::dto::{CommandResponse, SaveDraftCommand, SubmitAttemptCommand};
use ielts_domain::ErrorEnvelope;
use tauri::State;

use crate::app::state::{AppDb, AppVault};
use crate::commands::ai::{load_provider_config, load_runtime};
use ielts_db::{
    finish_evaluation, get_writing_draft, list_events, load_evaluation_for_attempt,
    prepare_evaluation, request_cancel, save_writing_draft, start_evaluation,
    submit_writing_attempt, DeterministicProvider, EvaluationEvent, EvaluationRunResult,
    StartEvaluationCommand, WritingDraft,
};

mod openai_provider;

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
pub async fn writing_start_evaluation(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    cmd: StartEvaluationCommand,
) -> Result<CommandResponse<EvaluationRunResult>, ErrorEnvelope> {
    let config = match db.with_conn(load_provider_config) {
        Ok(config) => config,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };

    if config.provider == "deterministic" {
        return Ok(
            match db.with_conn(|conn| start_evaluation(conn, &cmd, &DeterministicProvider)) {
                Ok(result) => CommandResponse::success(result),
                Err(error) => CommandResponse::failure(map_err(error)),
            },
        );
    }

    let runtime = match load_runtime(&db, &vault) {
        Ok(runtime) => runtime,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };
    let prepared = match db
        .with_conn(|conn| prepare_evaluation(conn, &cmd, &config.provider, &config.model))
    {
        Ok(prepared) => prepared,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };
    if let Some(existing) = prepared.existing.clone() {
        return Ok(CommandResponse::success(existing));
    }

    // No SQLite guard exists while this future performs network I/O.
    let provider_result = openai_provider::evaluate(&runtime, &prepared).await;
    let result = match provider_result {
        Ok(output) => db.with_conn(|conn| {
            finish_evaluation(
                conn,
                &prepared,
                Ok(output.score),
                Some(output.feedback),
                None,
            )
        }),
        Err(error) => {
            db.with_conn(|conn| finish_evaluation(conn, &prepared, Err(error), None, None))
        }
    };
    Ok(match result {
        Ok(result) => CommandResponse::success(result),
        Err(error) => CommandResponse::failure(map_err(error)),
    })
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
