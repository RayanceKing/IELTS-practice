//! Layered settings store + secret reference bookkeeping (Phase 4).
//!
//! Plain preferences live in SQLite `settings`.
//! API keys never live in SQLite value columns — only opaque `secret_refs`.

use rusqlite::{params, Connection};
use serde_json::Value;

use ielts_domain::dto::{SecretRef, SettingEntry};

use crate::sqlite::{DbError, DbResult};

pub const NS_UI: &str = "ui";
pub const NS_PRACTICE: &str = "practice";
pub const NS_AI: &str = "ai";
pub const NS_SYSTEM: &str = "system";
pub const NS_SECRET_REFS: &str = "secret_refs";

/// Preferences that historically lived in localStorage and must migrate.
pub const LEGACY_UI_KEYS: &[&str] = &[
    "theme",
    "three_bg_theme",
    "locale",
    "reduced_motion",
    "library_layout",
    "history_page_size",
];

pub fn get_setting(conn: &Connection, namespace: &str, key: &str) -> DbResult<Option<SettingEntry>> {
    let mut stmt = conn.prepare(
        "SELECT namespace, key, value_json, updated_at FROM settings WHERE namespace = ?1 AND key = ?2",
    )?;
    let mut rows = stmt.query(params![namespace, key])?;
    if let Some(row) = rows.next()? {
        let value_json: String = row.get(2)?;
        let value: Value = serde_json::from_str(&value_json)
            .map_err(|e| DbError::Validation(format!("settings json: {e}")))?;
        Ok(Some(SettingEntry {
            namespace: row.get(0)?,
            key: row.get(1)?,
            value,
            updated_at: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_settings(
    conn: &Connection,
    namespace: Option<&str>,
) -> DbResult<Vec<SettingEntry>> {
    let mut out = Vec::new();
    if let Some(ns) = namespace {
        let mut stmt = conn.prepare(
            "SELECT namespace, key, value_json, updated_at FROM settings WHERE namespace = ?1 ORDER BY key",
        )?;
        let rows = stmt.query_map(params![ns], map_setting_row)?;
        for row in rows {
            out.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT namespace, key, value_json, updated_at FROM settings WHERE namespace != ?1 ORDER BY namespace, key",
        )?;
        // Exclude secret_refs namespace from general listing of "settings" when dumping UI prefs.
        let rows = stmt.query_map(params![NS_SECRET_REFS], map_setting_row)?;
        for row in rows {
            out.push(row?);
        }
    }
    Ok(out)
}

pub fn upsert_setting(
    conn: &Connection,
    namespace: &str,
    key: &str,
    value: &Value,
) -> DbResult<SettingEntry> {
    if namespace == NS_SECRET_REFS {
        return Err(DbError::Validation(
            "use secret_refs API for secret references".into(),
        ));
    }
    if looks_like_secret_payload(namespace, key, value) {
        return Err(DbError::Validation(
            "refusing to store API key / secret material in settings table".into(),
        ));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let value_json =
        serde_json::to_string(value).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO settings (namespace, key, value_json, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(namespace, key) DO UPDATE SET
           value_json = excluded.value_json,
           updated_at = excluded.updated_at",
        params![namespace, key, value_json, now],
    )?;
    Ok(SettingEntry {
        namespace: namespace.to_string(),
        key: key.to_string(),
        value: value.clone(),
        updated_at: now,
    })
}

pub fn delete_setting(conn: &Connection, namespace: &str, key: &str) -> DbResult<bool> {
    let n = conn.execute(
        "DELETE FROM settings WHERE namespace = ?1 AND key = ?2",
        params![namespace, key],
    )?;
    Ok(n > 0)
}

/// Import a flat localStorage map into ui/practice namespaces.
pub fn migrate_local_storage_prefs(
    conn: &Connection,
    prefs: &serde_json::Map<String, Value>,
) -> DbResult<u32> {
    let mut count = 0u32;
    for (key, value) in prefs {
        if key_looks_like_secret(key) {
            continue;
        }
        let namespace = if LEGACY_UI_KEYS.contains(&key.as_str())
            || key.starts_with("ui.")
            || key.starts_with("theme")
        {
            NS_UI
        } else if key.starts_with("practice") || key.starts_with("reading") || key.starts_with("writing")
        {
            NS_PRACTICE
        } else {
            NS_SYSTEM
        };
        let clean_key = key.trim_start_matches("ui.").to_string();
        upsert_setting(conn, namespace, &clean_key, value)?;
        count += 1;
    }
    Ok(count)
}

/// Persist only a secret *reference* (never the secret value).
pub fn put_secret_ref(conn: &Connection, name: &str, ref_id: &str) -> DbResult<SecretRef> {
    if name.trim().is_empty() || ref_id.trim().is_empty() {
        return Err(DbError::Validation("secret name/ref_id required".into()));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({ "refId": ref_id, "name": name });
    let value_json =
        serde_json::to_string(&payload).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO settings (namespace, key, value_json, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(namespace, key) DO UPDATE SET
           value_json = excluded.value_json,
           updated_at = excluded.updated_at",
        params![NS_SECRET_REFS, name, value_json, now],
    )?;
    Ok(SecretRef {
        name: name.to_string(),
        ref_id: ref_id.to_string(),
        updated_at: now,
    })
}

pub fn list_secret_refs(conn: &Connection) -> DbResult<Vec<SecretRef>> {
    let mut stmt = conn.prepare(
        "SELECT key, value_json, updated_at FROM settings WHERE namespace = ?1 ORDER BY key",
    )?;
    let rows = stmt.query_map(params![NS_SECRET_REFS], |row| {
        let key: String = row.get(0)?;
        let value_json: String = row.get(1)?;
        let updated_at: String = row.get(2)?;
        Ok((key, value_json, updated_at))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (key, value_json, updated_at) = row?;
        let value: Value = serde_json::from_str(&value_json).unwrap_or(Value::Null);
        let ref_id = value
            .get("refId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        out.push(SecretRef {
            name: key,
            ref_id,
            updated_at,
        });
    }
    Ok(out)
}

pub fn delete_secret_ref(conn: &Connection, name: &str) -> DbResult<bool> {
    let n = conn.execute(
        "DELETE FROM settings WHERE namespace = ?1 AND key = ?2",
        params![NS_SECRET_REFS, name],
    )?;
    Ok(n > 0)
}

fn map_setting_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SettingEntry> {
    let value_json: String = row.get(2)?;
    let value = serde_json::from_str(&value_json).unwrap_or(Value::Null);
    Ok(SettingEntry {
        namespace: row.get(0)?,
        key: row.get(1)?,
        value,
        updated_at: row.get(3)?,
    })
}

fn key_looks_like_secret(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    k.contains("api_key")
        || k.contains("apikey")
        || k.contains("secret")
        || k.contains("token")
        || k.contains("password")
}

fn looks_like_secret_payload(namespace: &str, key: &str, value: &Value) -> bool {
    if key_looks_like_secret(key) || key_looks_like_secret(namespace) {
        return true;
    }
    if let Some(s) = value.as_str() {
        // Heuristic: long opaque strings under ai namespace
        if namespace == NS_AI && s.len() >= 20 && !s.contains(' ') {
            return true;
        }
    }
    if let Some(obj) = value.as_object() {
        return obj.keys().any(|k| key_looks_like_secret(k));
    }
    false
}
