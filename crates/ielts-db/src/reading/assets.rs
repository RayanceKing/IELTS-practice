//! Reading asset index + fingerprint (Phase 6).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use ielts_domain::domain::{Activity, AssetSourceKind};
use ielts_domain::dto::PracticeAssetV2;

use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexEntry {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub frequency: Option<String>,
    pub fingerprint: String,
    pub schema_version: u32,
    pub content_ref: Option<String>,
}

pub fn fingerprint_payload(payload: &Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

pub fn upsert_practice_asset(conn: &Connection, asset: &PracticeAssetV2) -> DbResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let meta = asset
        .metadata
        .as_ref()
        .map(|v| v.to_string());
    conn.execute(
        "INSERT INTO practice_assets (
            id, activity, source_kind, source_key, title, category, difficulty, frequency,
            content_ref, schema_version, fingerprint, pdf_only, metadata_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            category = excluded.category,
            difficulty = excluded.difficulty,
            frequency = excluded.frequency,
            content_ref = excluded.content_ref,
            schema_version = excluded.schema_version,
            fingerprint = excluded.fingerprint,
            pdf_only = excluded.pdf_only,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at",
        params![
            asset.id,
            "reading",
            match asset.source_kind {
                AssetSourceKind::Builtin => "builtin",
                AssetSourceKind::Imported => "imported",
                AssetSourceKind::Freeform => "freeform",
            },
            asset.source_key,
            asset.title,
            asset.category,
            asset.difficulty,
            asset.frequency,
            asset.content_ref,
            asset.schema_version as i64,
            asset.fingerprint,
            if asset.pdf_only { 1 } else { 0 },
            meta,
            now,
        ],
    )?;
    Ok(())
}

pub fn list_assets(conn: &Connection, activity: Option<Activity>) -> DbResult<Vec<AssetIndexEntry>> {
    let mut sql = String::from(
        "SELECT id, title, category, difficulty, frequency, fingerprint, schema_version, content_ref
         FROM practice_assets",
    );
    if activity.is_some() {
        sql.push_str(" WHERE activity = ?1");
    }
    sql.push_str(" ORDER BY category, title");
    let mut stmt = conn.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(AssetIndexEntry {
            id: row.get(0)?,
            title: row.get(1)?,
            category: row.get(2)?,
            difficulty: row.get(3)?,
            frequency: row.get(4)?,
            fingerprint: row.get(5)?,
            schema_version: row.get::<_, i64>(6)? as u32,
            content_ref: row.get(7)?,
        })
    };
    let rows = if let Some(act) = activity {
        let a = match act {
            Activity::Reading => "reading",
            Activity::Writing => "writing",
        };
        stmt.query_map(params![a], map_row)?
    } else {
        stmt.query_map([], map_row)?
    };
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Load a reading payload JSON file and register it as a practice asset.
pub fn import_asset_payload_file(conn: &Connection, path: &Path) -> DbResult<PracticeAssetV2> {
    let raw = fs::read_to_string(path)?;
    let payload: Value =
        serde_json::from_str(&raw).map_err(|e| DbError::Validation(format!("asset json: {e}")))?;
    import_asset_payload(conn, &payload, Some(path.display().to_string()))
}

pub fn import_asset_payload(
    conn: &Connection,
    payload: &Value,
    content_ref: Option<String>,
) -> DbResult<PracticeAssetV2> {
    let exam_id = payload
        .get("examId")
        .or_else(|| payload.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::Validation("asset missing examId".into()))?
        .to_string();
    let title = payload
        .pointer("/meta/title")
        .or_else(|| payload.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or(&exam_id)
        .to_string();
    let category = payload
        .pointer("/meta/category")
        .or_else(|| payload.get("category"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let frequency = payload
        .pointer("/meta/frequency")
        .or_else(|| payload.get("frequency"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let fp = fingerprint_payload(payload);
    let asset = PracticeAssetV2 {
        schema_version: PracticeAssetV2::SCHEMA_VERSION,
        id: exam_id.clone(),
        activity: Activity::Reading,
        source_kind: AssetSourceKind::Imported,
        source_key: Some(exam_id.clone()),
        title,
        category,
        difficulty: None,
        frequency,
        content_ref,
        fingerprint: fp,
        pdf_only: false,
        metadata: Some(json_meta(payload)),
    };
    upsert_practice_asset(conn, &asset)?;
    Ok(asset)
}

fn json_meta(payload: &Value) -> Value {
    json!({
        "questionCount": payload.get("questionCount").cloned().unwrap_or(Value::Null),
        "hasAnswerKey": payload.get("answerKey").is_some(),
    })
}

use serde_json::json;

/// Scan a directory of JSON reading payloads.
pub fn scan_asset_directory(dir: &Path) -> DbResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub fn load_answer_key(payload: &Value) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    if let Some(obj) = payload.get("answerKey").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            map.insert(normalize_qid(k), v.clone());
        }
    }
    map
}

pub fn load_controls(payload: &Value) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    if let Some(obj) = payload.get("interactionModel").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if let Some(control) = v.get("control") {
                map.insert(normalize_qid(k), control.clone());
            }
        }
    }
    map
}

pub fn load_kinds(payload: &Value) -> serde_json::Map<String, Value> {
    let mut map = HashMap::new();
    if let Some(groups) = payload.get("questionGroups").and_then(|v| v.as_array()) {
        for g in groups {
            let kind = g
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            if let Some(ids) = g.get("questionIds").and_then(|v| v.as_array()) {
                for id in ids {
                    if let Some(s) = id.as_str() {
                        map.insert(normalize_qid(s), kind.clone());
                    }
                }
            }
        }
    }
    map.into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect()
}

fn normalize_qid(s: &str) -> String {
    crate::reading::scoring::normalize_question_id(s)
}
