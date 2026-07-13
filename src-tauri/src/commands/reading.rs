//! Reading asset + attempt Tauri commands (Phase 6).

use ielts_domain::domain::Activity;
use ielts_domain::dto::CommandResponse;
use ielts_domain::ErrorEnvelope;
use tauri::State;

use crate::app::state::AppDb;
use ielts_db::{
    list_assets, load_practice_asset_payload, patch_reading_answer, save_reading_draft,
    submit_reading_attempt, AssetIndexEntry, ReadingDraftCommand, ReadingSubmitCommand,
    ReadingSubmitResult,
};

fn map_err(err: ielts_db::DbError) -> ErrorEnvelope {
    ErrorEnvelope {
        code: "reading.error".into(),
        message: err.to_string(),
        retryable: false,
        context: None,
        cause_id: None,
    }
}

#[tauri::command]
pub fn reading_list_assets(db: State<'_, AppDb>) -> CommandResponse<Vec<AssetIndexEntry>> {
    match db.with_conn(|conn| list_assets(conn, Some(Activity::Reading))) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn reading_get_asset_payload(
    db: State<'_, AppDb>,
    asset_id: String,
) -> CommandResponse<ielts_domain::dto::PracticeAssetV2Payload> {
    match db.with_conn(|conn| load_practice_asset_payload(conn, &asset_id)) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn reading_save_draft(
    db: State<'_, AppDb>,
    cmd: ReadingDraftCommand,
) -> CommandResponse<ielts_domain::dto::AttemptRecord> {
    match db.with_conn(|conn| save_reading_draft(conn, &cmd)) {
        Ok(a) => CommandResponse::success(a),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn reading_patch_answer(
    db: State<'_, AppDb>,
    attempt_id: String,
    question_id: String,
    answer: serde_json::Value,
    marked: Option<bool>,
) -> CommandResponse<bool> {
    match db.with_conn(|conn| {
        patch_reading_answer(
            conn,
            &attempt_id,
            &question_id,
            &answer,
            marked.unwrap_or(false),
        )
    }) {
        Ok(()) => CommandResponse::success(true),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}

#[tauri::command]
pub fn reading_submit_attempt(
    db: State<'_, AppDb>,
    cmd: ReadingSubmitCommand,
) -> CommandResponse<ReadingSubmitResult> {
    match db.with_conn(|conn| submit_reading_attempt(conn, &cmd)) {
        Ok(r) => CommandResponse::success(r),
        Err(e) => CommandResponse::failure(map_err(e)),
    }
}
