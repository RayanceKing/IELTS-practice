//! Backup create / dry-run import / import commands (Phase 4).

use std::path::PathBuf;

use ielts_domain::dto::{BackupManifest, CommandResponse, ImportBackupReport};
use ielts_domain::ErrorEnvelope;
use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::app::state::{AppDb, AppPaths};

fn map_db_err(err: ielts_db::DbError) -> ErrorEnvelope {
    ErrorEnvelope {
        code: "backup.error".into(),
        message: err.to_string(),
        retryable: false,
        context: None,
        cause_id: None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupResult {
    pub manifest: BackupManifest,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFileEntry {
    pub name: String,
    pub path: String,
    pub modified_at: Option<String>,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn create_backup(
    db: State<'_, AppDb>,
    paths: State<'_, AppPaths>,
    app_version: Option<String>,
) -> CommandResponse<CreateBackupResult> {
    let version = app_version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    match db.with_conn(|conn| ielts_db::create_backup_package(conn, &version)) {
        Ok(package) => {
            let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            let path = paths.backups.join(format!("ielts-backup-{stamp}.json"));
            if let Err(e) = std::fs::create_dir_all(&paths.backups) {
                return CommandResponse::failure(ErrorEnvelope {
                    code: "backup.path".into(),
                    message: format!("cannot create backups dir: {e}"),
                    retryable: false,
                    context: None,
                    cause_id: None,
                });
            }
            if let Err(e) = ielts_db::write_backup_file(&package, &path) {
                return CommandResponse::failure(map_db_err(e));
            }
            tracing::info!(path = %path.display(), "backup written");
            CommandResponse::success(CreateBackupResult {
                manifest: package.manifest,
                path: path.display().to_string(),
            })
        }
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}

#[tauri::command]
pub fn list_backups(paths: State<'_, AppPaths>) -> CommandResponse<Vec<BackupFileEntry>> {
    let mut entries = Vec::new();
    let dir = &paths.backups;
    if !dir.exists() {
        return CommandResponse::success(entries);
    }
    let read = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            return CommandResponse::failure(ErrorEnvelope {
                code: "backup.path".into(),
                message: format!("cannot read backups dir: {e}"),
                retryable: false,
                context: None,
                cause_id: None,
            })
        }
    };
    for item in read.flatten() {
        let path = item.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let meta = item.metadata().ok();
        let modified_at = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            });
        entries.push(BackupFileEntry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            path: path.display().to_string(),
            modified_at,
            size_bytes: meta.map(|m| m.len()).unwrap_or(0),
        });
    }
    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    CommandResponse::success(entries)
}

#[tauri::command]
pub fn pick_backup_import_path(app: tauri::AppHandle) -> CommandResponse<Option<String>> {
    let file = app
        .dialog()
        .file()
        .add_filter("IELTS Backup", &["json"])
        .blocking_pick_file();
    let path = file.map(|f| f.to_string());
    CommandResponse::success(path)
}

#[tauri::command]
pub fn import_backup_path(
    db: State<'_, AppDb>,
    path: String,
    dry_run: bool,
) -> CommandResponse<ImportBackupReport> {
    let path = PathBuf::from(path);
    let canon = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return CommandResponse::failure(ErrorEnvelope {
                code: "backup.path".into(),
                message: format!("invalid backup path: {e}"),
                retryable: false,
                context: None,
                cause_id: None,
            });
        }
    };
    let package = match ielts_db::read_backup_file(&canon) {
        Ok(p) => p,
        Err(e) => return CommandResponse::failure(map_db_err(e)),
    };
    match db.with_conn(|conn| ielts_db::import_backup(conn, &package, dry_run)) {
        Ok(report) => CommandResponse::success(report),
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}
