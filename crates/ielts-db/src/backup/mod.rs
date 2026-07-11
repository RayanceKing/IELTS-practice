//! Backup package create / dry-run import / import report (Phase 4).
//! Ordinary backups never embed plaintext secrets.

use std::path::Path;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use ielts_domain::dto::{
    AttemptRecord, BackupManifest, BackupPackage, ImportBackupReport, SecretRef, SettingEntry,
};

use crate::import::upsert_attempt;
use crate::settings::{list_secret_refs, list_settings, put_secret_ref, upsert_setting};
use crate::sqlite::{DbError, DbResult};

pub const BACKUP_SCHEMA_VERSION: u32 = 1;

pub fn create_backup_package(
    conn: &Connection,
    app_version: &str,
) -> DbResult<BackupPackage> {
    let attempts = load_all_attempts_summary(conn)?;
    let settings = list_settings(conn, None)?;
    let secret_refs = list_secret_refs(conn)?;

    let mut package = BackupPackage {
        manifest: BackupManifest {
            schema_version: BACKUP_SCHEMA_VERSION,
            created_at: chrono::Utc::now().to_rfc3339(),
            app_version: app_version.to_string(),
            includes_secrets: false,
            attempt_count: attempts.len() as u32,
            settings_count: settings.len() as u32,
            secret_ref_count: secret_refs.len() as u32,
            checksum_sha256: String::new(),
        },
        attempts,
        settings,
        secret_refs,
    };
    package.manifest.checksum_sha256 = checksum_package(&package)?;
    Ok(package)
}

pub fn write_backup_file(package: &BackupPackage, path: &Path) -> DbResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(package).map_err(|e| DbError::Message(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn read_backup_file(path: &Path) -> DbResult<BackupPackage> {
    let raw = std::fs::read_to_string(path)?;
    let package: BackupPackage =
        serde_json::from_str(&raw).map_err(|e| DbError::Validation(format!("backup parse: {e}")))?;
    Ok(package)
}

pub fn validate_backup(package: &BackupPackage) -> DbResult<Vec<String>> {
    let mut warnings = Vec::new();
    if package.manifest.schema_version == 0 || package.manifest.schema_version > BACKUP_SCHEMA_VERSION
    {
        return Err(DbError::Validation(format!(
            "unsupported backup schema_version {}",
            package.manifest.schema_version
        )));
    }
    if package.manifest.includes_secrets {
        return Err(DbError::Validation(
            "backup claims includes_secrets=true; refuse ordinary import".into(),
        ));
    }
    if package.attempts.len() as u32 != package.manifest.attempt_count {
        warnings.push(format!(
            "manifest attempt_count {} != payload {}",
            package.manifest.attempt_count,
            package.attempts.len()
        ));
    }
    if package.settings.len() as u32 != package.manifest.settings_count {
        warnings.push(format!(
            "manifest settings_count {} != payload {}",
            package.manifest.settings_count,
            package.settings.len()
        ));
    }
    let expected = checksum_package(package)?;
    if !package.manifest.checksum_sha256.is_empty()
        && package.manifest.checksum_sha256 != expected
    {
        return Err(DbError::Validation(format!(
            "checksum mismatch: manifest {} computed {}",
            package.manifest.checksum_sha256, expected
        )));
    }
    // Scan for accidental secret material
    for s in &package.settings {
        let key_l = s.key.to_ascii_lowercase();
        if key_l.contains("api_key") || key_l.contains("secret") || key_l.contains("password") {
            return Err(DbError::Validation(format!(
                "backup setting looks like secret material: {}.{}",
                s.namespace, s.key
            )));
        }
        if let Some(text) = s.value.as_str() {
            if s.namespace == "ai" && text.len() >= 20 && !text.contains(' ') {
                return Err(DbError::Validation(format!(
                    "backup setting {}.{} looks like an opaque secret string",
                    s.namespace, s.key
                )));
            }
        }
    }
    Ok(warnings)
}

pub fn import_backup(
    conn: &Connection,
    package: &BackupPackage,
    dry_run: bool,
) -> DbResult<ImportBackupReport> {
    let mut report = ImportBackupReport {
        dry_run,
        ok: true,
        attempt_imported: 0,
        settings_imported: 0,
        secret_refs_imported: 0,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    match validate_backup(package) {
        Ok(warnings) => report.warnings.extend(warnings),
        Err(e) => {
            report.ok = false;
            report.errors.push(e.to_string());
            return Ok(report);
        }
    }

    if dry_run {
        report.attempt_imported = package.attempts.len() as u32;
        report.settings_imported = package.settings.len() as u32;
        report.secret_refs_imported = package.secret_refs.len() as u32;
        return Ok(report);
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| DbError::Sqlite(e))?;

    for attempt in &package.attempts {
        match upsert_attempt(&tx, attempt) {
            Ok(()) => report.attempt_imported += 1,
            Err(e) => {
                report.ok = false;
                report.errors.push(format!("attempt {}: {e}", attempt.id));
            }
        }
    }

    for setting in &package.settings {
        match upsert_setting(&tx, &setting.namespace, &setting.key, &setting.value) {
            Ok(_) => report.settings_imported += 1,
            Err(e) => {
                report.ok = false;
                report
                    .errors
                    .push(format!("setting {}.{}: {e}", setting.namespace, setting.key));
            }
        }
    }

    for secret in &package.secret_refs {
        match put_secret_ref(&tx, &secret.name, &secret.ref_id) {
            Ok(_) => report.secret_refs_imported += 1,
            Err(e) => {
                report.ok = false;
                report
                    .errors
                    .push(format!("secret_ref {}: {e}", secret.name));
            }
        }
    }

    if report.ok {
        tx.commit()?;
    } else {
        // drop tx = rollback
        drop(tx);
    }

    Ok(report)
}

fn checksum_package(package: &BackupPackage) -> DbResult<String> {
    // Hash payload without the checksum field itself.
    let mut for_hash = package.clone();
    for_hash.manifest.checksum_sha256.clear();
    let bytes =
        serde_json::to_vec(&for_hash).map_err(|e| DbError::Message(e.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn load_all_attempts_summary(conn: &Connection) -> DbResult<Vec<AttemptRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
                duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
                prompt_snapshot, content_text, schema_version
         FROM attempts
         ORDER BY COALESCE(submitted_at, started_at) DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        use ielts_domain::domain::{Activity, AttemptMode, AttemptStatus, ScoreScale};
        Ok(AttemptRecord {
            schema_version: row.get::<_, i64>(17)? as u32,
            id: row.get(0)?,
            activity: match row.get::<_, String>(1)?.as_str() {
                "writing" => Activity::Writing,
                _ => Activity::Reading,
            },
            asset_id: row.get(2)?,
            mode: match row.get::<_, String>(3)?.as_str() {
                "suite" => AttemptMode::Suite,
                "endless" => AttemptMode::Endless,
                "memorize" => AttemptMode::Memorize,
                "freeform" => AttemptMode::Freeform,
                "bank" => AttemptMode::Bank,
                _ => AttemptMode::Single,
            },
            suite_id: row.get(4)?,
            status: match row.get::<_, String>(5)?.as_str() {
                "draft" => AttemptStatus::Draft,
                "active" => AttemptStatus::Active,
                "submitted" => AttemptStatus::Submitted,
                "reviewing" => AttemptStatus::Reviewing,
                "cancelled" => AttemptStatus::Cancelled,
                "failed" => AttemptStatus::Failed,
                "interrupted" => AttemptStatus::Interrupted,
                _ => AttemptStatus::Completed,
            },
            started_at: row.get(6)?,
            submitted_at: row.get(7)?,
            completed_at: row.get(8)?,
            duration_ms: row.get::<_, i64>(9)? as u64,
            score_value: row.get(10)?,
            score_scale: row.get::<_, Option<String>>(11)?.and_then(|s| match s.as_str() {
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
        out.push(row?);
    }
    Ok(out)
}

// silence unused import if SecretRef only used in types
#[allow(dead_code)]
fn _touch_secret_ref(_: &SecretRef) {}
#[allow(dead_code)]
fn _touch_setting(_: &SettingEntry) {}
