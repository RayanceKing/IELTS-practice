//! Layered settings store + secret reference bookkeeping (Phase 4).
//!
//! Plain preferences live in SQLite `settings`.
//! API keys never live in SQLite value columns — only opaque `secret_refs`.

use rusqlite::{params, Connection};
use serde_json::Value;

use ielts_domain::dto::{AiConfigDto, SecretRef, SettingEntry};

use crate::sqlite::{DbError, DbResult};

pub const NS_UI: &str = "ui";
pub const NS_PRACTICE: &str = "practice";
pub const NS_AI: &str = "ai";
pub const NS_SYSTEM: &str = "system";
pub const NS_SECRET_REFS: &str = "secret_refs";
const AI_CONFIG_PREFIX: &str = "config:";
const AI_DEFAULT_ID: &str = "defaultConfigId";

/// Preferences that historically lived in localStorage and must migrate.
pub const LEGACY_UI_KEYS: &[&str] = &[
    "theme",
    "three_bg_theme",
    "locale",
    "reduced_motion",
    "library_layout",
    "history_page_size",
];

pub fn get_setting(
    conn: &Connection,
    namespace: &str,
    key: &str,
) -> DbResult<Option<SettingEntry>> {
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

pub fn list_settings(conn: &Connection, namespace: Option<&str>) -> DbResult<Vec<SettingEntry>> {
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
    let value_json = serde_json::to_string(value).map_err(|e| DbError::Message(e.to_string()))?;
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

pub fn list_ai_configs(conn: &Connection) -> DbResult<Vec<AiConfigDto>> {
    let default_id = get_setting(conn, NS_AI, AI_DEFAULT_ID)?
        .and_then(|entry| entry.value.as_str().map(str::to_owned));
    let refs = list_secret_refs(conn)?;
    let mut configs = Vec::new();
    for entry in list_settings(conn, Some(NS_AI))? {
        if !entry.key.starts_with(AI_CONFIG_PREFIX) {
            continue;
        }
        let mut config: AiConfigDto = serde_json::from_value(entry.value)
            .map_err(|e| DbError::Validation(format!("AI config json: {e}")))?;
        config.is_default = default_id.as_deref() == Some(config.id.as_str());
        config.has_secret = refs
            .iter()
            .any(|item| item.name == ai_secret_name(&config.id));
        configs.push(config);
    }
    configs.sort_by(|a, b| a.config_name.cmp(&b.config_name).then(a.id.cmp(&b.id)));
    Ok(configs)
}

pub fn upsert_ai_config(conn: &Connection, config: &AiConfigDto) -> DbResult<()> {
    let mut stored = config.clone();
    stored.is_default = false;
    stored.has_secret = false;
    let value = serde_json::to_value(stored).map_err(|e| DbError::Message(e.to_string()))?;
    upsert_setting(
        conn,
        NS_AI,
        &format!("{AI_CONFIG_PREFIX}{}", config.id),
        &value,
    )?;
    Ok(())
}

pub fn delete_ai_config(conn: &Connection, id: &str) -> DbResult<bool> {
    delete_setting(conn, NS_AI, &format!("{AI_CONFIG_PREFIX}{id}"))
}

pub fn set_default_ai_config(conn: &Connection, config: Option<&AiConfigDto>) -> DbResult<()> {
    match config {
        Some(config) => {
            upsert_setting(
                conn,
                NS_AI,
                AI_DEFAULT_ID,
                &Value::String(config.id.clone()),
            )?;
            write_ai_runtime_value(conn, "provider", &Value::String("openai-compatible".into()))?;
            write_ai_runtime_value(conn, "baseUrl", &Value::String(config.base_url.clone()))?;
            write_ai_runtime_value(conn, "model", &Value::String(config.default_model.clone()))?;
            write_ai_runtime_value(
                conn,
                "secretName",
                &Value::String(ai_secret_name(&config.id)),
            )?;
        }
        None => {
            delete_setting(conn, NS_AI, AI_DEFAULT_ID)?;
            write_ai_runtime_value(conn, "provider", &Value::String("deterministic".into()))?;
            for key in ["baseUrl", "model", "secretName"] {
                delete_setting(conn, NS_AI, key)?;
            }
        }
    }
    Ok(())
}

pub fn ai_secret_name(id: &str) -> String {
    format!("ai.config.{id}")
}

fn write_ai_runtime_value(conn: &Connection, key: &str, value: &Value) -> DbResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let value_json = serde_json::to_string(value).map_err(|e| DbError::Message(e.to_string()))?;
    conn.execute(
        "INSERT INTO settings (namespace, key, value_json, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(namespace, key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
        params![NS_AI, key, value_json, now],
    )?;
    Ok(())
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
        } else if key.starts_with("practice")
            || key.starts_with("reading")
            || key.starts_with("writing")
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

#[cfg(test)]
mod ai_config_tests {
    use super::*;
    use ielts_domain::dto::AiConfigDto;

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (
                namespace TEXT NOT NULL, key TEXT NOT NULL, value_json TEXT NOT NULL,
                updated_at TEXT NOT NULL, PRIMARY KEY(namespace, key)
            );",
        )
        .unwrap();
        conn
    }

    fn config(id: &str) -> AiConfigDto {
        AiConfigDto {
            id: id.into(),
            config_name: "OpenRouter".into(),
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            default_model: "openai/gpt-4.1-mini".into(),
            is_default: false,
            is_enabled: true,
            has_secret: true,
        }
    }

    #[test]
    fn metadata_never_persists_secret_material() {
        let conn = connection();
        upsert_ai_config(&conn, &config("primary")).unwrap();
        let raw: String = conn
            .query_row(
                "SELECT value_json FROM settings WHERE namespace='ai' AND key='config:primary'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!raw.contains("apiKey"));
        assert!(!raw.contains("secretName"));
        assert!(raw.contains("\"hasSecret\":false"));
    }

    #[test]
    fn default_config_drives_the_single_runtime_settings() {
        let conn = connection();
        let value = config("primary");
        upsert_ai_config(&conn, &value).unwrap();
        set_default_ai_config(&conn, Some(&value)).unwrap();
        assert_eq!(
            get_setting(&conn, NS_AI, "provider")
                .unwrap()
                .unwrap()
                .value,
            Value::String("openai-compatible".into())
        );
        assert_eq!(
            get_setting(&conn, NS_AI, "baseUrl").unwrap().unwrap().value,
            Value::String(value.base_url)
        );
        assert_eq!(
            get_setting(&conn, NS_AI, "model").unwrap().unwrap().value,
            Value::String(value.default_model)
        );
        assert_eq!(
            get_setting(&conn, NS_AI, "secretName")
                .unwrap()
                .unwrap()
                .value,
            Value::String("ai.config.primary".into())
        );
    }
}
