//! Unified history Tauri commands (Phase 4).

use ielts_domain::dto::{
    CommandResponse, ExportHistoryCommand, ExportHistoryResult, HistoryDetailResponse,
    ListHistoryPage, ListHistoryQuery,
};
use ielts_domain::ErrorEnvelope;
use tauri::State;

use crate::app::state::{AppDb, AppPaths};

fn map_db_err(err: ielts_db::DbError) -> ErrorEnvelope {
    ErrorEnvelope {
        code: "db.error".into(),
        message: err.to_string(),
        retryable: false,
        context: None,
        cause_id: None,
    }
}

#[tauri::command]
pub fn list_history(
    db: State<'_, AppDb>,
    query: ListHistoryQuery,
) -> CommandResponse<ListHistoryPage> {
    match db.with_conn(|conn| ielts_db::list_history(conn, &query)) {
        Ok(page) => CommandResponse::success(page),
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}

#[tauri::command]
pub fn get_history_detail(
    db: State<'_, AppDb>,
    attempt_id: String,
) -> CommandResponse<HistoryDetailResponse> {
    match db.with_conn(|conn| ielts_db::get_history_detail(conn, &attempt_id)) {
        Ok(detail) => CommandResponse::success(detail),
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}

#[tauri::command]
pub fn export_history(
    db: State<'_, AppDb>,
    paths: State<'_, AppPaths>,
    cmd: ExportHistoryCommand,
) -> CommandResponse<ExportHistoryResult> {
    match db.with_conn(|conn| ielts_db::export_history(conn, cmd.format, cmd.query.as_ref())) {
        Ok(mut result) => {
            // Optionally persist export under app exports dir for file dialogs later.
            let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            let ext = match result.format {
                ielts_domain::dto::HistoryExportFormat::Csv => "csv",
                ielts_domain::dto::HistoryExportFormat::Markdown => "md",
                ielts_domain::dto::HistoryExportFormat::Json => "json",
            };
            let path = paths.exports.join(format!("history-{stamp}.{ext}"));
            if let Err(err) = std::fs::write(&path, result.body.as_bytes()) {
                tracing::warn!(error = %err, "failed to write history export file");
            } else {
                tracing::info!(path = %path.display(), "history export written");
            }
            // body still returned for UI download/copy
            let _ = &mut result;
            CommandResponse::success(result)
        }
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}

#[tauri::command]
pub fn delete_history_attempt(db: State<'_, AppDb>, attempt_id: String) -> CommandResponse<bool> {
    match db.with_conn(|conn| ielts_db::delete_attempt(conn, &attempt_id)) {
        Ok(ok) => CommandResponse::success(ok),
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingArchiveImportResult {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub attempt_ids: Vec<String>,
    pub errors: Vec<String>,
}

/// Product reading archive import (`submissions[]` or cold `records[]`).
#[tauri::command]
pub fn import_reading_archive_value(
    db: State<'_, AppDb>,
    value: serde_json::Value,
) -> CommandResponse<ReadingArchiveImportResult> {
    match db.with_conn(|conn| ielts_db::import_reading_archive_value(conn, &value)) {
        Ok(report) => CommandResponse::success(ReadingArchiveImportResult {
            imported_count: report.imported,
            skipped_count: 0,
            failed_count: report.failed,
            attempt_ids: report.attempt_ids,
            errors: report.errors,
        }),
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}
