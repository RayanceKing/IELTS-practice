//! Versioned, lossless backup / restore for the canonical SQLite store.
//! Ordinary backups contain opaque keychain references, never secret bytes.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params, params_from_iter, Connection, Transaction};
use sha2::{Digest, Sha256};

use ielts_domain::dto::{
    AttemptRecord, BackupManifest, BackupPackage, BackupSqlValue, BackupTable, ImportBackupReport,
    SecretRef, SettingEntry,
};

use crate::attempts::upsert_attempt;
use crate::migrate::current_version;
use crate::settings::{list_secret_refs, list_settings, put_secret_ref, upsert_setting};
use crate::sqlite::{DbError, DbResult};

pub const BACKUP_SCHEMA_VERSION: u32 = 2;
const LEGACY_BACKUP_SCHEMA_VERSION: u32 = 1;

// Parent tables precede their children. Restore inserts in this order and
// clears in reverse order, so foreign keys remain enabled for the whole
// transaction.
const CANONICAL_TABLES: &[&str] = &[
    "practice_assets",
    "writing_topics",
    "reading_suites",
    "attempts",
    "attempt_answers",
    "attempt_annotations",
    "writing_evaluations",
    "writing_drafts",
    "attempt_idempotency",
    "evaluation_sessions",
    "evaluation_checkpoints",
    "evaluation_events",
    "evaluation_lineage",
    "reading_suite_items",
    "endless_sessions",
    "mode_idempotency",
    "coach_threads",
    "coach_messages",
    "vocabulary_items",
    "vocabulary_review_state",
    "dictionary_entries",
    "settings",
    "migration_meta",
];

pub fn create_backup_package(conn: &Connection, app_version: &str) -> DbResult<BackupPackage> {
    let attempts = load_all_attempts(conn)?;
    let settings = list_settings(conn, None)?;
    let secret_refs = list_secret_refs(conn)?;
    let database = snapshot_database(conn)?;
    let database_rows = database
        .iter()
        .map(|table| table.rows.len() as u64)
        .sum::<u64>();

    let mut package = BackupPackage {
        manifest: BackupManifest {
            schema_version: BACKUP_SCHEMA_VERSION,
            database_schema_version: current_version(conn)? as u32,
            created_at: chrono::Utc::now().to_rfc3339(),
            app_version: app_version.to_string(),
            includes_secrets: false,
            attempt_count: attempts.len() as u32,
            settings_count: settings.len() as u32,
            secret_ref_count: secret_refs.len() as u32,
            table_count: database.len() as u32,
            row_count: database_rows + secret_refs.len() as u64,
            checksum_sha256: String::new(),
        },
        attempts,
        settings,
        secret_refs,
        database,
    };
    package.manifest.checksum_sha256 = checksum_package(&package)?;
    // Creating a file is already an export boundary. Refuse to return a
    // package that would fail the same integrity or secret-policy checks on
    // import.
    validate_backup(&package)?;
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
    serde_json::from_str(&raw).map_err(|e| DbError::Validation(format!("backup parse: {e}")))
}

/// Validate format, counts, checksum, JSON payloads, secret policy and logical
/// references without touching the target database.
pub fn validate_backup(package: &BackupPackage) -> DbResult<Vec<String>> {
    if package.manifest.schema_version == 0
        || package.manifest.schema_version > BACKUP_SCHEMA_VERSION
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
    verify_checksum(package)?;
    validate_secret_policy(package)?;

    if package.manifest.schema_version == LEGACY_BACKUP_SCHEMA_VERSION {
        if !package.database.is_empty() {
            return Err(DbError::Validation(
                "legacy backup schema v1 must not contain a database snapshot".into(),
            ));
        }
        let mut warnings = validate_legacy_counts(package);
        warnings.push(
            "legacy backup schema v1 is incomplete; only attempt summaries, settings and secret references can be restored"
                .into(),
        );
        return Ok(warnings);
    }

    validate_v2_counts(package)?;
    let tables = table_map(&package.database)?;
    validate_json_cells(&tables)?;
    validate_logical_references(&tables)?;
    validate_redundant_views(package, &tables)?;
    Ok(Vec::new())
}

pub fn import_backup(
    conn: &Connection,
    package: &BackupPackage,
    dry_run: bool,
) -> DbResult<ImportBackupReport> {
    let mut report = empty_report(dry_run);
    match validate_backup(package) {
        Ok(warnings) => report.warnings = warnings,
        Err(error) => {
            report.ok = false;
            report.errors.push(error.to_string());
            return Ok(report);
        }
    }

    if package.manifest.schema_version == LEGACY_BACKUP_SCHEMA_VERSION {
        return import_legacy_backup(conn, package, dry_run, report);
    }

    if let Err(error) = validate_target_schema(conn, package) {
        report.ok = false;
        report.errors.push(error.to_string());
        return Ok(report);
    }

    // Dry-run executes the exact restore against the real schema, then drops
    // the transaction. This verifies CHECK/UNIQUE/FK constraints without
    // leaving any persistent mutation.
    let tx = conn.unchecked_transaction()?;
    match restore_v2_snapshot(&tx, package) {
        Ok(()) => {
            report.attempt_imported = package.manifest.attempt_count;
            report.settings_imported = package.manifest.settings_count;
            report.secret_refs_imported = package.manifest.secret_ref_count;
            report.tables_imported = package.manifest.table_count;
            report.rows_imported = package.manifest.row_count;
            if dry_run {
                drop(tx);
            } else {
                tx.commit()?;
            }
        }
        Err(error) => {
            drop(tx);
            report.ok = false;
            report.errors.push(error.to_string());
        }
    }
    Ok(report)
}

fn empty_report(dry_run: bool) -> ImportBackupReport {
    ImportBackupReport {
        dry_run,
        ok: true,
        attempt_imported: 0,
        settings_imported: 0,
        secret_refs_imported: 0,
        tables_imported: 0,
        rows_imported: 0,
        errors: Vec::new(),
        warnings: Vec::new(),
    }
}

fn import_legacy_backup(
    conn: &Connection,
    package: &BackupPackage,
    dry_run: bool,
    mut report: ImportBackupReport,
) -> DbResult<ImportBackupReport> {
    if dry_run {
        report.attempt_imported = package.attempts.len() as u32;
        report.settings_imported = package.settings.len() as u32;
        report.secret_refs_imported = package.secret_refs.len() as u32;
        return Ok(report);
    }

    let tx = conn.unchecked_transaction()?;
    let result = (|| -> DbResult<()> {
        for attempt in &package.attempts {
            upsert_attempt(&tx, attempt)?;
            report.attempt_imported += 1;
        }
        for setting in &package.settings {
            upsert_setting(&tx, &setting.namespace, &setting.key, &setting.value)?;
            report.settings_imported += 1;
        }
        for secret in &package.secret_refs {
            put_secret_ref(&tx, &secret.name, &secret.ref_id)?;
            report.secret_refs_imported += 1;
        }
        Ok(())
    })();
    match result {
        Ok(()) => tx.commit()?,
        Err(error) => {
            drop(tx);
            report.ok = false;
            report.errors.push(error.to_string());
        }
    }
    Ok(report)
}

fn snapshot_database(conn: &Connection) -> DbResult<Vec<BackupTable>> {
    let mut out = Vec::with_capacity(CANONICAL_TABLES.len());
    for table in CANONICAL_TABLES {
        let columns = table_columns(conn, table)?;
        let projection = columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let predicate = if *table == "settings" {
            " WHERE namespace != 'secret_refs'"
        } else {
            ""
        };
        let sql = format!(
            "SELECT {projection} FROM {}{predicate}",
            quote_identifier(table)
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let mut values = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                values.push(sql_value_from_ref(row.get_ref(index)?));
            }
            Ok(values)
        })?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row?);
        }
        // Stable ordering makes the checksum independent of row insertion
        // order and enables exact source/restored snapshot comparison.
        values.sort_by_cached_key(|row| serde_json::to_string(row).unwrap_or_default());
        out.push(BackupTable {
            name: (*table).to_string(),
            columns,
            rows: values,
        });
    }
    Ok(out)
}

fn table_columns(conn: &Connection, table: &str) -> DbResult<Vec<String>> {
    let sql = format!("PRAGMA table_info({})", quote_identifier(table));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    if columns.is_empty() {
        return Err(DbError::Validation(format!(
            "canonical backup table is missing: {table}"
        )));
    }
    Ok(columns)
}

fn sql_value_from_ref(value: ValueRef<'_>) -> BackupSqlValue {
    match value {
        ValueRef::Null => BackupSqlValue::Null,
        ValueRef::Integer(value) => BackupSqlValue::Integer(value),
        ValueRef::Real(value) => BackupSqlValue::Real(value),
        ValueRef::Text(value) => BackupSqlValue::Text(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => BackupSqlValue::Blob(value.to_vec()),
    }
}

fn sql_value(value: &BackupSqlValue) -> SqlValue {
    match value {
        BackupSqlValue::Null => SqlValue::Null,
        BackupSqlValue::Integer(value) => SqlValue::Integer(*value),
        BackupSqlValue::Real(value) => SqlValue::Real(*value),
        BackupSqlValue::Text(value) => SqlValue::Text(value.clone()),
        BackupSqlValue::Blob(value) => SqlValue::Blob(value.clone()),
    }
}

fn validate_target_schema(conn: &Connection, package: &BackupPackage) -> DbResult<()> {
    let target_version = current_version(conn)?;
    if target_version < package.manifest.database_schema_version as i64 {
        return Err(DbError::Validation(format!(
            "backup requires database schema {}, target has {}",
            package.manifest.database_schema_version, target_version
        )));
    }
    for table in &package.database {
        let target_columns = table_columns(conn, &table.name)?;
        if table.columns != target_columns {
            return Err(DbError::Validation(format!(
                "backup table {} columns do not match target schema",
                table.name
            )));
        }
    }
    Ok(())
}

fn restore_v2_snapshot(tx: &Transaction<'_>, package: &BackupPackage) -> DbResult<()> {
    let tables = table_map(&package.database)?;
    for table in CANONICAL_TABLES.iter().rev() {
        tx.execute(&format!("DELETE FROM {}", quote_identifier(table)), [])?;
    }
    for table_name in CANONICAL_TABLES {
        let table = tables[table_name];
        let columns = table
            .columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=table.columns.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO {} ({columns}) VALUES ({placeholders})",
            quote_identifier(table_name)
        );
        let mut stmt = tx.prepare(&sql)?;
        for row in &table.rows {
            let values = row.iter().map(sql_value).collect::<Vec<_>>();
            stmt.execute(params_from_iter(values))?;
        }
    }
    // Secret references are deliberately outside the raw settings snapshot.
    // Preserve their timestamps while never asking the keychain for values.
    for secret in &package.secret_refs {
        let value_json = serde_json::to_string(&serde_json::json!({
            "refId": secret.ref_id,
            "name": secret.name,
        }))
        .map_err(|error| DbError::Message(error.to_string()))?;
        tx.execute(
            "INSERT INTO settings(namespace, key, value_json, updated_at) VALUES ('secret_refs', ?1, ?2, ?3)",
            params![secret.name, value_json, secret.updated_at],
        )?;
    }
    assert_no_foreign_key_violations(tx)?;
    Ok(())
}

fn assert_no_foreign_key_violations(conn: &Connection) -> DbResult<()> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let table: String = row.get(0)?;
        let parent: String = row.get(2)?;
        return Err(DbError::Validation(format!(
            "foreign key violation after restore: {table} -> {parent}"
        )));
    }
    Ok(())
}

fn table_map<'a>(tables: &'a [BackupTable]) -> DbResult<HashMap<&'a str, &'a BackupTable>> {
    let allowed = CANONICAL_TABLES.iter().copied().collect::<HashSet<_>>();
    let mut out = HashMap::new();
    for table in tables {
        if !allowed.contains(table.name.as_str()) {
            return Err(DbError::Validation(format!(
                "backup contains unsupported table: {}",
                table.name
            )));
        }
        if table.columns.is_empty() {
            return Err(DbError::Validation(format!(
                "backup table {} has no columns",
                table.name
            )));
        }
        let unique_columns = table.columns.iter().collect::<HashSet<_>>();
        if unique_columns.len() != table.columns.len() {
            return Err(DbError::Validation(format!(
                "backup table {} contains duplicate columns",
                table.name
            )));
        }
        if table
            .rows
            .iter()
            .any(|row| row.len() != table.columns.len())
        {
            return Err(DbError::Validation(format!(
                "backup table {} contains a row with the wrong width",
                table.name
            )));
        }
        if out.insert(table.name.as_str(), table).is_some() {
            return Err(DbError::Validation(format!(
                "backup contains duplicate table: {}",
                table.name
            )));
        }
    }
    for required in CANONICAL_TABLES {
        if !out.contains_key(required) {
            return Err(DbError::Validation(format!(
                "backup is incomplete; missing canonical table: {required}"
            )));
        }
    }
    Ok(out)
}

fn validate_v2_counts(package: &BackupPackage) -> DbResult<()> {
    if package.manifest.table_count != package.database.len() as u32 {
        return Err(DbError::Validation(format!(
            "manifest table_count {} != payload {}",
            package.manifest.table_count,
            package.database.len()
        )));
    }
    let row_count = package
        .database
        .iter()
        .map(|table| table.rows.len() as u64)
        .sum::<u64>()
        + package.secret_refs.len() as u64;
    if package.manifest.row_count != row_count {
        return Err(DbError::Validation(format!(
            "manifest row_count {} != payload {}",
            package.manifest.row_count, row_count
        )));
    }
    if package.manifest.attempt_count != package.attempts.len() as u32
        || package.manifest.settings_count != package.settings.len() as u32
        || package.manifest.secret_ref_count != package.secret_refs.len() as u32
    {
        return Err(DbError::Validation(
            "manifest summary counts do not match payload".into(),
        ));
    }
    if package.manifest.database_schema_version == 0 {
        return Err(DbError::Validation(
            "backup database_schema_version is required for schema v2".into(),
        ));
    }
    Ok(())
}

fn validate_legacy_counts(package: &BackupPackage) -> Vec<String> {
    let mut warnings = Vec::new();
    for (label, manifest, actual) in [
        (
            "attempt_count",
            package.manifest.attempt_count,
            package.attempts.len() as u32,
        ),
        (
            "settings_count",
            package.manifest.settings_count,
            package.settings.len() as u32,
        ),
        (
            "secret_ref_count",
            package.manifest.secret_ref_count,
            package.secret_refs.len() as u32,
        ),
    ] {
        if manifest != actual {
            warnings.push(format!("manifest {label} {manifest} != payload {actual}"));
        }
    }
    warnings
}

fn validate_redundant_views(
    package: &BackupPackage,
    tables: &HashMap<&str, &BackupTable>,
) -> DbResult<()> {
    let attempt_ids = text_set(tables["attempts"], "id")?;
    let summary_ids = package
        .attempts
        .iter()
        .map(|attempt| attempt.id.clone())
        .collect::<HashSet<_>>();
    if attempt_ids != summary_ids {
        return Err(DbError::Validation(
            "attempt summary does not match canonical attempts table".into(),
        ));
    }
    let setting_keys = composite_text_set(tables["settings"], "namespace", "key")?;
    let summary_setting_keys = package
        .settings
        .iter()
        .map(|setting| (setting.namespace.clone(), setting.key.clone()))
        .collect::<HashSet<_>>();
    if setting_keys != summary_setting_keys {
        return Err(DbError::Validation(
            "settings summary does not match canonical settings table".into(),
        ));
    }
    Ok(())
}

fn validate_json_cells(tables: &HashMap<&str, &BackupTable>) -> DbResult<()> {
    for table in tables.values() {
        for (column_index, column) in table.columns.iter().enumerate() {
            let is_json = column.ends_with("_json") || column == "structured_payload";
            if !is_json {
                continue;
            }
            for (row_index, row) in table.rows.iter().enumerate() {
                if let BackupSqlValue::Text(raw) = &row[column_index] {
                    let value =
                        serde_json::from_str::<serde_json::Value>(raw).map_err(|error| {
                            DbError::Validation(format!(
                                "invalid JSON in {}.{} row {}: {}",
                                table.name, column, row_index, error
                            ))
                        })?;
                    reject_json_secret_material(
                        &value,
                        &format!("{}.{} row {}", table.name, column, row_index),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn validate_logical_references(tables: &HashMap<&str, &BackupTable>) -> DbResult<()> {
    let assets = text_set(tables["practice_assets"], "id")?;
    let suites = text_set(tables["reading_suites"], "id")?;
    let attempts = text_set(tables["attempts"], "id")?;
    let evaluations = text_set(tables["writing_evaluations"], "id")?;
    let threads = text_set(tables["coach_threads"], "id")?;
    let vocab = text_set(tables["vocabulary_items"], "id")?;

    require_refs(tables["writing_topics"], "asset_id", &assets)?;
    require_optional_refs(tables["attempts"], "asset_id", &assets)?;
    require_optional_refs(tables["attempts"], "suite_id", &suites)?;
    require_refs(tables["attempt_answers"], "attempt_id", &attempts)?;
    require_optional_refs(tables["attempt_annotations"], "attempt_id", &attempts)?;
    require_refs(tables["attempt_annotations"], "asset_id", &assets)?;
    for table in [
        "writing_evaluations",
        "writing_drafts",
        "attempt_idempotency",
        "evaluation_sessions",
        "evaluation_lineage",
    ] {
        require_refs(tables[table], "attempt_id", &attempts)?;
    }
    require_optional_refs(tables["attempt_idempotency"], "evaluation_id", &evaluations)?;
    require_refs(tables["evaluation_sessions"], "evaluation_id", &evaluations)?;
    for table in [
        "evaluation_checkpoints",
        "evaluation_events",
        "evaluation_lineage",
    ] {
        require_refs(tables[table], "evaluation_id", &evaluations)?;
    }
    require_optional_refs(tables["evaluation_lineage"], "retry_of", &evaluations)?;
    require_optional_refs(
        tables["evaluation_lineage"],
        "root_evaluation_id",
        &evaluations,
    )?;
    require_refs(tables["reading_suite_items"], "suite_id", &suites)?;
    require_refs(tables["reading_suite_items"], "asset_id", &assets)?;
    require_optional_refs(tables["reading_suite_items"], "attempt_id", &attempts)?;
    require_optional_refs(tables["endless_sessions"], "current_asset_id", &assets)?;
    require_optional_refs(tables["endless_sessions"], "current_attempt_id", &attempts)?;
    require_optional_refs(tables["coach_threads"], "attempt_id", &attempts)?;
    require_optional_refs(tables["coach_threads"], "asset_id", &assets)?;
    require_refs(tables["coach_messages"], "thread_id", &threads)?;
    require_optional_refs(tables["vocabulary_items"], "source_asset_id", &assets)?;
    require_optional_refs(tables["vocabulary_items"], "source_attempt_id", &attempts)?;
    require_refs(tables["vocabulary_review_state"], "item_id", &vocab)?;
    Ok(())
}

fn require_refs(table: &BackupTable, column: &str, valid: &HashSet<String>) -> DbResult<()> {
    let index = column_index(table, column)?;
    for row in &table.rows {
        match &row[index] {
            BackupSqlValue::Text(value) if valid.contains(value) => {}
            BackupSqlValue::Text(value) => {
                return Err(DbError::Validation(format!(
                    "dangling reference {}.{}={value}",
                    table.name, column
                )))
            }
            _ => {
                return Err(DbError::Validation(format!(
                    "required text reference {}.{} is missing",
                    table.name, column
                )))
            }
        }
    }
    Ok(())
}

fn require_optional_refs(
    table: &BackupTable,
    column: &str,
    valid: &HashSet<String>,
) -> DbResult<()> {
    let index = column_index(table, column)?;
    for row in &table.rows {
        match &row[index] {
            BackupSqlValue::Null => {}
            BackupSqlValue::Text(value) if valid.contains(value) => {}
            BackupSqlValue::Text(value) => {
                return Err(DbError::Validation(format!(
                    "dangling reference {}.{}={value}",
                    table.name, column
                )))
            }
            _ => {
                return Err(DbError::Validation(format!(
                    "optional text reference {}.{} has invalid type",
                    table.name, column
                )))
            }
        }
    }
    Ok(())
}

fn text_set(table: &BackupTable, column: &str) -> DbResult<HashSet<String>> {
    let index = column_index(table, column)?;
    let mut values = HashSet::new();
    for row in &table.rows {
        let BackupSqlValue::Text(value) = &row[index] else {
            return Err(DbError::Validation(format!(
                "{}.{} must contain text values",
                table.name, column
            )));
        };
        if !values.insert(value.clone()) {
            return Err(DbError::Validation(format!(
                "duplicate logical id in {}.{}: {}",
                table.name, column, value
            )));
        }
    }
    Ok(values)
}

fn composite_text_set(
    table: &BackupTable,
    first: &str,
    second: &str,
) -> DbResult<HashSet<(String, String)>> {
    let first_index = column_index(table, first)?;
    let second_index = column_index(table, second)?;
    let mut values = HashSet::new();
    for row in &table.rows {
        let (BackupSqlValue::Text(first_value), BackupSqlValue::Text(second_value)) =
            (&row[first_index], &row[second_index])
        else {
            return Err(DbError::Validation(format!(
                "{}.{} and {} must contain text values",
                table.name, first, second
            )));
        };
        values.insert((first_value.clone(), second_value.clone()));
    }
    Ok(values)
}

fn column_index(table: &BackupTable, column: &str) -> DbResult<usize> {
    table
        .columns
        .iter()
        .position(|candidate| candidate == column)
        .ok_or_else(|| {
            DbError::Validation(format!(
                "backup table {} is missing column {column}",
                table.name
            ))
        })
}

fn validate_secret_policy(package: &BackupPackage) -> DbResult<()> {
    for setting in &package.settings {
        reject_secret_setting(&setting.namespace, &setting.key, &setting.value)?;
    }
    if let Some(table) = package
        .database
        .iter()
        .find(|table| table.name == "settings")
    {
        let namespace_index = column_index(table, "namespace")?;
        let key_index = column_index(table, "key")?;
        let value_index = column_index(table, "value_json")?;
        for row in &table.rows {
            let (
                BackupSqlValue::Text(namespace),
                BackupSqlValue::Text(key),
                BackupSqlValue::Text(value_json),
            ) = (&row[namespace_index], &row[key_index], &row[value_index])
            else {
                return Err(DbError::Validation(
                    "backup settings row has invalid SQLite types".into(),
                ));
            };
            if namespace == "secret_refs" {
                return Err(DbError::Validation(
                    "secret references must not be duplicated in the raw settings snapshot".into(),
                ));
            }
            let value = serde_json::from_str(value_json).map_err(|error| {
                DbError::Validation(format!("settings JSON for {namespace}.{key}: {error}"))
            })?;
            reject_secret_setting(namespace, key, &value)?;
        }
    }
    for secret in &package.secret_refs {
        if secret.name.trim().is_empty() || secret.ref_id.trim().is_empty() {
            return Err(DbError::Validation(
                "secret reference name/ref_id must not be empty".into(),
            ));
        }
        if looks_like_secret_text(&secret.ref_id) {
            return Err(DbError::Validation(format!(
                "secret reference {} appears to contain plaintext secret material",
                secret.name
            )));
        }
    }
    Ok(())
}

fn reject_secret_setting(namespace: &str, key: &str, value: &serde_json::Value) -> DbResult<()> {
    let key_lower = key.to_ascii_lowercase();
    let known_reference_metadata = key_lower == "secretname" || key_lower == "hassecret";
    let sensitive_key = sensitive_field_name(&key_lower);
    if sensitive_key && !known_reference_metadata {
        return Err(DbError::Validation(format!(
            "backup setting looks like secret material: {namespace}.{key}"
        )));
    }
    if value.as_str().is_some_and(looks_like_secret_text) {
        return Err(DbError::Validation(format!(
            "backup setting contains plaintext secret material: {namespace}.{key}"
        )));
    }
    Ok(())
}

fn reject_json_secret_material(value: &serde_json::Value, location: &str) -> DbResult<()> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                let key_lower = key.to_ascii_lowercase();
                let metadata_only = key_lower == "secretname" || key_lower == "hassecret";
                if sensitive_field_name(&key_lower) && !metadata_only && !child.is_null() {
                    return Err(DbError::Validation(format!(
                        "backup JSON contains secret-bearing field {key} at {location}"
                    )));
                }
                reject_json_secret_material(child, location)?;
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                reject_json_secret_material(child, location)?;
            }
        }
        serde_json::Value::String(text) if looks_like_secret_text(text) => {
            return Err(DbError::Validation(format!(
                "backup JSON contains plaintext secret material at {location}"
            )))
        }
        _ => {}
    }
    Ok(())
}

fn sensitive_field_name(key_lower: &str) -> bool {
    key_lower.contains("api_key")
        || key_lower.contains("apikey")
        || key_lower.contains("password")
        || key_lower == "secret"
        || key_lower == "token"
        || key_lower == "accesstoken"
        || key_lower == "access_token"
        || key_lower == "authtoken"
        || key_lower == "auth_token"
}

fn looks_like_secret_text(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("sk-")
        || trimmed.starts_with("sk_")
        || trimmed.starts_with("AIza")
        || trimmed.starts_with("xoxb-")
        || trimmed.starts_with("xoxp-")
        || trimmed.to_ascii_lowercase().starts_with("bearer ")
}

fn verify_checksum(package: &BackupPackage) -> DbResult<()> {
    if package.manifest.schema_version >= 2 && package.manifest.checksum_sha256.is_empty() {
        return Err(DbError::Validation(
            "checksum_sha256 is required for backup schema v2".into(),
        ));
    }
    let expected = checksum_package(package)?;
    if !package.manifest.checksum_sha256.is_empty() && package.manifest.checksum_sha256 != expected
    {
        return Err(DbError::Validation(format!(
            "checksum mismatch: manifest {} computed {}",
            package.manifest.checksum_sha256, expected
        )));
    }
    Ok(())
}

fn checksum_package(package: &BackupPackage) -> DbResult<String> {
    let mut for_hash = package.clone();
    for_hash.manifest.checksum_sha256.clear();
    let bytes = serde_json::to_vec(&for_hash).map_err(|e| DbError::Message(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn load_all_attempts(conn: &Connection) -> DbResult<Vec<AttemptRecord>> {
    let mut attempts = load_all_attempt_summaries(conn)?;
    for attempt in &mut attempts {
        let mut answer_stmt = conn.prepare(
            "SELECT question_id, answer_json, is_correct, weight, question_kind, change_count,
                    visit_count, elapsed_ms, marked, answered_at
             FROM attempt_answers WHERE attempt_id = ?1 ORDER BY question_id",
        )?;
        let answers = answer_stmt.query_map(params![attempt.id], |row| {
            let answer_json: String = row.get(1)?;
            Ok(ielts_domain::AttemptAnswer {
                question_id: row.get(0)?,
                answer: serde_json::from_str(&answer_json).unwrap_or(serde_json::Value::Null),
                is_correct: row.get::<_, Option<i64>>(2)?.map(|value| value != 0),
                weight: row.get(3)?,
                question_kind: row.get(4)?,
                change_count: row.get::<_, i64>(5)? as u32,
                visit_count: row.get::<_, i64>(6)? as u32,
                elapsed_ms: row.get::<_, i64>(7)? as u64,
                marked: row.get::<_, i64>(8)? != 0,
                answered_at: row.get(9)?,
            })
        })?;
        for answer in answers {
            attempt.answers.push(answer?);
        }

        let mut annotation_stmt = conn.prepare(
            "SELECT id, attempt_id, asset_id, scope, question_id, kind, anchor_json, note_text
             FROM attempt_annotations WHERE attempt_id = ?1 ORDER BY id",
        )?;
        let annotations = annotation_stmt.query_map(params![attempt.id], |row| {
            let anchor_json: String = row.get(6)?;
            Ok(ielts_domain::AttemptAnnotationDto {
                id: row.get(0)?,
                attempt_id: row.get(1)?,
                asset_id: row.get(2)?,
                scope: row.get(3)?,
                question_id: row.get(4)?,
                kind: row.get(5)?,
                anchor: serde_json::from_str(&anchor_json)
                    .unwrap_or_else(|_| serde_json::json!({})),
                note_text: row.get(7)?,
            })
        })?;
        for annotation in annotations {
            attempt.annotations.push(annotation?);
        }
    }
    Ok(attempts)
}

fn load_all_attempt_summaries(conn: &Connection) -> DbResult<Vec<AttemptRecord>> {
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
            score_scale: row
                .get::<_, Option<String>>(11)?
                .and_then(|scale| match scale.as_str() {
                    "ratio" => Some(ScoreScale::Ratio),
                    "band9" => Some(ScoreScale::Band9),
                    _ => None,
                }),
            correct_count: row.get(12)?,
            question_count: row.get::<_, Option<i64>>(13)?.map(|value| value as u32),
            title_snapshot: row.get(14)?,
            prompt_snapshot: row.get(15)?,
            content_text: row.get(16)?,
            answers: Vec::new(),
            annotations: Vec::new(),
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

// Keep these imports part of the public compatibility surface documented by
// this module even when feature combinations trim callers.
#[allow(dead_code)]
fn _touch_secret_ref(_: &SecretRef) {}
#[allow(dead_code)]
fn _touch_setting(_: &SettingEntry) {}
