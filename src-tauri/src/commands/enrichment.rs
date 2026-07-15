//! Annotations, dictionary, vocab, coach Tauri commands (Phase 8).

use ielts_domain::dto::CommandResponse;
use ielts_domain::ErrorEnvelope;
use tauri::State;

use crate::app::state::{AppDb, AppVault};
use crate::commands::{ai::load_runtime, coach_provider};
use ielts_db::{
    append_coach_message, complete_coach_run, delete_annotation, delete_vocab, ensure_coach_thread,
    import_dictionary, list_annotations, list_coach_messages, list_vocab, lookup_term,
    record_coach_failure, revalidate_annotations, review_vocab, upsert_annotation, upsert_vocab,
    AnnotationRecord, AppendCoachMessageCommand, CoachMessage, CoachRunResult, CoachThread,
    DictionaryEntry, EnsureCoachThreadCommand, ImportDictionaryCommand, RecordCoachFailureCommand,
    ReviewVocabCommand, RunCoachCommand, UpsertAnnotationCommand, UpsertVocabCommand,
    VocabularyItem,
};

fn map_err(err: ielts_db::DbError) -> ErrorEnvelope {
    ErrorEnvelope {
        code: "enrichment.error".into(),
        message: err.to_string(),
        retryable: false,
        context: None,
        cause_id: None,
    }
}

#[tauri::command]
pub fn annotation_upsert(
    db: State<'_, AppDb>,
    cmd: UpsertAnnotationCommand,
) -> CommandResponse<AnnotationRecord> {
    match db.with_conn(|conn| upsert_annotation(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn annotation_list(
    db: State<'_, AppDb>,
    asset_id: String,
    attempt_id: Option<String>,
) -> CommandResponse<Vec<AnnotationRecord>> {
    match db.with_conn(|conn| list_annotations(conn, &asset_id, attempt_id.as_deref())) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn annotation_delete(
    db: State<'_, AppDb>,
    id: String,
    asset_id: String,
    attempt_id: Option<String>,
) -> CommandResponse<bool> {
    match db.with_conn(|conn| delete_annotation(conn, &id, &asset_id, attempt_id.as_deref())) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn annotation_revalidate(
    db: State<'_, AppDb>,
    asset_id: String,
    attempt_id: Option<String>,
    scope: String,
    document: String,
) -> CommandResponse<Vec<AnnotationRecord>> {
    match db.with_conn(|conn| {
        revalidate_annotations(conn, &asset_id, attempt_id.as_deref(), &scope, &document)
    }) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn dictionary_lookup(db: State<'_, AppDb>, term: String) -> CommandResponse<DictionaryEntry> {
    match db.with_conn(|conn| lookup_term(conn, &term)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn dictionary_import(
    db: State<'_, AppDb>,
    cmd: ImportDictionaryCommand,
) -> CommandResponse<u32> {
    match db.with_conn(|conn| import_dictionary(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn vocab_upsert(
    db: State<'_, AppDb>,
    cmd: UpsertVocabCommand,
) -> CommandResponse<VocabularyItem> {
    match db.with_conn(|conn| upsert_vocab(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn vocab_list(
    db: State<'_, AppDb>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> CommandResponse<Vec<VocabularyItem>> {
    match db.with_conn(|conn| list_vocab(conn, limit.unwrap_or(100), offset.unwrap_or(0))) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn vocab_review(
    db: State<'_, AppDb>,
    cmd: ReviewVocabCommand,
) -> CommandResponse<VocabularyItem> {
    match db.with_conn(|conn| review_vocab(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn vocab_delete(db: State<'_, AppDb>, id: String) -> CommandResponse<bool> {
    match db.with_conn(|conn| delete_vocab(conn, &id)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn coach_ensure_thread(
    db: State<'_, AppDb>,
    cmd: EnsureCoachThreadCommand,
) -> CommandResponse<CoachThread> {
    match db.with_conn(|conn| ensure_coach_thread(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn coach_append_message(
    db: State<'_, AppDb>,
    cmd: AppendCoachMessageCommand,
) -> CommandResponse<CoachMessage> {
    match db.with_conn(|conn| append_coach_message(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn coach_list_messages(
    db: State<'_, AppDb>,
    thread_id: String,
    after_sequence: Option<u32>,
    limit: Option<u32>,
) -> CommandResponse<Vec<CoachMessage>> {
    match db.with_conn(|conn| {
        list_coach_messages(conn, &thread_id, after_sequence, limit.unwrap_or(100))
    }) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn coach_record_failure(
    db: State<'_, AppDb>,
    cmd: RecordCoachFailureCommand,
) -> CommandResponse<CoachThread> {
    match db.with_conn(|conn| record_coach_failure(conn, &cmd)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub async fn coach_run(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    cmd: RunCoachCommand,
) -> Result<CommandResponse<CoachRunResult>, ErrorEnvelope> {
    let user_message = match db.with_conn(|conn| {
        append_coach_message(
            conn,
            &AppendCoachMessageCommand {
                thread_id: cmd.thread_id.clone(),
                role: "user".into(),
                content: cmd.content.clone(),
                structured_payload: cmd.question_context.clone(),
                status: "completed".into(),
            },
        )
    }) {
        Ok(message) => message,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };
    let history = match db.with_conn(|conn| list_coach_messages(conn, &cmd.thread_id, None, 100)) {
        Ok(history) => history,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };
    let runtime = match load_runtime(&db, &vault) {
        Ok(runtime) => runtime,
        Err(error) => {
            let envelope = map_err(error);
            let _ = db.with_conn(|conn| {
                record_coach_failure(
                    conn,
                    &RecordCoachFailureCommand {
                        thread_id: cmd.thread_id.clone(),
                        error: serde_json::to_value(&envelope).unwrap_or_default(),
                        preserve_scores: true,
                    },
                )
            });
            return Ok(CommandResponse::failure(envelope));
        }
    };

    // The database lock is not held across this network future.
    match coach_provider::answer(&runtime, &history, cmd.question_context.as_ref()).await {
        Ok((answer, payload)) => match db
            .with_conn(|conn| complete_coach_run(conn, &cmd.thread_id, &answer, Some(payload)))
        {
            Ok(assistant_message) => Ok(CommandResponse::success(CoachRunResult {
                user_message,
                assistant_message,
            })),
            Err(error) => Ok(CommandResponse::failure(map_err(error))),
        },
        Err(error) => {
            let envelope =
                ErrorEnvelope::new("coach.provider_failed", error.message, error.retryable);
            let _ = db.with_conn(|conn| {
                record_coach_failure(
                    conn,
                    &RecordCoachFailureCommand {
                        thread_id: cmd.thread_id,
                        error: serde_json::to_value(&envelope).unwrap_or_default(),
                        preserve_scores: true,
                    },
                )
            });
            Ok(CommandResponse::failure(envelope))
        }
    }
}
