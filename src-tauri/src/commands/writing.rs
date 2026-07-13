//! Writing draft + evaluation Tauri commands (Phase 5).

use ielts_domain::dto::{CommandResponse, SaveDraftCommand, SubmitAttemptCommand};
use ielts_domain::ErrorEnvelope;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::app::state::{AppDb, AppVault};
use crate::commands::ai::{load_provider_config, load_runtime};
use ielts_db::{
    finish_evaluation, get_writing_draft, list_events, load_evaluation_for_attempt,
    prepare_evaluation, request_cancel, save_writing_draft, submit_writing_attempt,
    DeterministicProvider, EvaluationEvent, EvaluationHandle, EvaluationRunResult,
    StartEvaluationCommand, WritingDraft, WritingProvider,
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
    app: AppHandle,
    db: State<'_, AppDb>,
    cmd: StartEvaluationCommand,
    on_event: Channel<EvaluationEvent>,
) -> Result<CommandResponse<EvaluationHandle>, ErrorEnvelope> {
    let config = match db.with_conn(load_provider_config) {
        Ok(config) => config,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };

    // Fail closed when no AI is configured. Deterministic is only for explicit offline mode.
    if config.provider == "unconfigured" || config.provider.trim().is_empty() {
        return Ok(CommandResponse::failure(ErrorEnvelope {
            code: "ai.not_configured".into(),
            message: "未配置 AI：请先在设置中添加并启用默认模型与 API Key。".into(),
            retryable: false,
            context: None,
            cause_id: None,
        }));
    }

    let deterministic = config.provider == "deterministic";
    let prepared = match db
        .with_conn(|conn| prepare_evaluation(conn, &cmd, &config.provider, &config.model))
    {
        Ok(prepared) => prepared,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };
    let handle = prepared.handle.clone();
    if let Some(existing) = prepared.existing.as_ref() {
        send_events(&on_event, &existing.events);
        return Ok(CommandResponse::success(handle));
    }

    if let Ok(initial_events) = db.with_conn(|conn| list_events(conn, &handle.evaluation_id, 0)) {
        send_events(&on_event, &initial_events);
    }

    // The command returns the durable handle now. The task owns only request
    // data and never carries a SQLite guard across provider I/O.
    let task_handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        let db = app.state::<AppDb>();
        let result = if deterministic {
            run_deterministic(&db, &prepared)
        } else {
            let vault = app.state::<AppVault>();
            match load_runtime(&db, &vault) {
                Ok(runtime) => {
                    let provider_result = openai_provider::evaluate(&runtime, &prepared).await;
                    match provider_result {
                        Ok(output) => {
                            let (feedback, review_error) = match output.feedback {
                                Ok(feedback) => (Some(feedback), None),
                                Err(error) => (None, Some(error)),
                            };
                            db.with_conn(|conn| {
                                finish_evaluation(
                                    conn,
                                    &prepared,
                                    Ok(output.score),
                                    feedback,
                                    review_error,
                                )
                            })
                        }
                        Err(error) => db.with_conn(|conn| {
                            finish_evaluation(conn, &prepared, Err(error), None, None)
                        }),
                    }
                }
                Err(error) => db.with_conn(|conn| {
                    finish_evaluation(
                        conn,
                        &prepared,
                        Err(ielts_db::ProviderError {
                            message: error.to_string(),
                            retryable: false,
                        }),
                        None,
                        None,
                    )
                }),
            }
        };
        match result {
            Ok(result) => send_events_after(&on_event, &result, task_handle.sequence),
            Err(error) => {
                tracing::error!(
                    evaluation_id = %task_handle.evaluation_id,
                    error = %error,
                    "background writing evaluation failed"
                );
            }
        }
    });

    Ok(CommandResponse::success(handle))
}

fn run_deterministic(
    db: &AppDb,
    prepared: &ielts_db::PreparedEvaluation,
) -> ielts_db::DbResult<EvaluationRunResult> {
    let provider = DeterministicProvider;
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
    db.with_conn(|conn| finish_evaluation(conn, prepared, score, feedback, review_error))
}

fn send_events(channel: &Channel<EvaluationEvent>, events: &[EvaluationEvent]) {
    for event in events {
        if let Err(error) = channel.send(event.clone()) {
            tracing::debug!(error = %error, "writing evaluation channel closed");
            break;
        }
    }
}

fn send_events_after(
    channel: &Channel<EvaluationEvent>,
    result: &EvaluationRunResult,
    after_sequence: u32,
) {
    let events = result
        .events
        .iter()
        .filter(|event| event.sequence > after_sequence)
        .cloned()
        .collect::<Vec<_>>();
    send_events(channel, &events);
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
