use std::path::Path;

use serde_json::Value;

use crate::import::repository::import_reading_submission_json;
use crate::sqlite::{DbError, DbResult};
use rusqlite::Connection;

#[derive(Debug, Default)]
pub struct ImportReport {
    pub imported: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub attempt_ids: Vec<String>,
}

pub fn import_reading_archive_value(conn: &Connection, doc: &Value) -> DbResult<ImportReport> {
    let records = doc
        .get("records")
        .and_then(|v| v.as_array())
        .ok_or_else(|| DbError::Import("reading archive missing records[]".into()))?;

    let mut report = ImportReport::default();
    for (idx, record) in records.iter().enumerate() {
        match import_reading_submission_json(conn, record) {
            Ok(id) => {
                report.imported += 1;
                report.attempt_ids.push(id);
            }
            Err(err) => {
                report.failed += 1;
                report.errors.push(format!("records[{idx}]: {err}"));
            }
        }
    }
    Ok(report)
}

pub fn import_reading_archive_file(conn: &Connection, path: &Path) -> DbResult<ImportReport> {
    let text = std::fs::read_to_string(path)?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| DbError::Import(e.to_string()))?;
    import_reading_archive_value(conn, &doc)
}
