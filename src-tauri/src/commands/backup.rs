//! Backup create / dry-run import / import commands (Phase 4).

use std::path::PathBuf;

use ielts_domain::dto::{BackupManifest, CommandResponse, ImportBackupReport};
use ielts_domain::ErrorEnvelope;
use tauri::State;

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

#[tauri::command]
pub fn create_backup(
    db: State<'_, AppDb>,
    paths: State<'_, AppPaths>,
    app_version: Option<String>,
) -> CommandResponse<BackupManifest> {
    let version = app_version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    match db.with_conn(|conn| ielts_db::create_backup_package(conn, &version)) {
        Ok(package) => {
            let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            let path = paths.backups.join(format!("ielts-backup-{stamp}.json"));
            if let Err(e) = ielts_db::write_backup_file(&package, &path) {
                return CommandResponse::failure(map_db_err(e));
            }
            tracing::info!(path = %path.display(), "backup written");
            CommandResponse::success(package.manifest)
        }
        Err(e) => CommandResponse::failure(map_db_err(e)),
    }
}

#[tauri::command]
pub fn import_backup_path(
    db: State<'_, AppDb>,
    path: String,
    dry_run: bool,
) -> CommandResponse<ImportBackupReport> {
    let path = PathBuf::from(path);
    // Path must be under user-selected import/export/backup or absolute user pick.
    // Capability fs scope already limits dialog paths; still canonicalize.
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
