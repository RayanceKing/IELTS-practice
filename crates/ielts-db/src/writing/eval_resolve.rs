//! Resolve active writing prompts + temperature for evaluation (product path).

use rusqlite::Connection;
use serde_json::Value;

use ielts_domain::domain::WritingTaskType;

use crate::settings::{get_setting, list_settings};
use crate::sqlite::DbResult;

const NS_PROMPTS: &str = "prompts";
const NS_MODEL: &str = "model";

#[derive(Debug, Clone)]
pub struct ResolvedWritingEvalPolicy {
    pub system_prompt: String,
    pub temperature: f32,
    pub prompt_id: Option<String>,
    pub prompt_version: String,
}

/// Default schema instruction when no active prompt template is configured.
pub const DEFAULT_SYSTEM_PROMPT: &str = "Return JSON only with this shape: {\"score\":{\"overall\":0,\"taskResponse\":0,\"coherence\":0,\"lexical\":0,\"grammar\":0},\"feedback\":{\"overall\":\"\",\"plan\":[],\"paragraphs\":[],\"sentences\":[],\"rewrites\":[]}}. Scores must be IELTS bands from 0 to 9.";

pub fn resolve_writing_eval_policy(
    conn: &Connection,
    task_type: Option<WritingTaskType>,
) -> DbResult<ResolvedWritingEvalPolicy> {
    let prompt = resolve_active_prompt(conn, task_type)?;
    let temperature = resolve_temperature(conn, task_type)?;
    let (system_prompt, prompt_id, prompt_version) = match prompt {
        Some(p) => {
            let body = extract_prompt_body(&p.value).unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.into());
            let version = p
                .value
                .get("version")
                .or_else(|| p.value.get("promptVersion"))
                .or_else(|| p.value.get("prompt_version"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| format!("prompt-{}", p.key));
            (body, Some(p.key), version)
        }
        None => (DEFAULT_SYSTEM_PROMPT.to_string(), None, "prompt-v1".into()),
    };
    Ok(ResolvedWritingEvalPolicy {
        system_prompt,
        temperature,
        prompt_id,
        prompt_version,
    })
}

#[derive(Debug, Clone)]
struct PromptRow {
    key: String,
    value: Value,
}

fn resolve_active_prompt(
    conn: &Connection,
    task_type: Option<WritingTaskType>,
) -> DbResult<Option<PromptRow>> {
    let rows = list_settings(conn, Some(NS_PROMPTS))?;
    let items: Vec<PromptRow> = rows
        .into_iter()
        .map(|e| PromptRow {
            key: e.key,
            value: e.value,
        })
        .collect();
    if items.is_empty() {
        return Ok(None);
    }

    let task_key = task_type.map(task_type_key);
    // Prefer active + matching task, then active any, then first matching task, then first.
    let active_match = items.iter().find(|p| {
        is_active(&p.value) && task_matches(&p.value, task_key)
    });
    if let Some(p) = active_match {
        return Ok(Some(p.clone()));
    }
    let active_any = items.iter().find(|p| is_active(&p.value));
    if let Some(p) = active_any {
        return Ok(Some(p.clone()));
    }
    if let Some(task) = task_key {
        if let Some(p) = items.iter().find(|p| task_matches(&p.value, Some(task))) {
            return Ok(Some(p.clone()));
        }
    }
    Ok(items.into_iter().next())
}

fn is_active(value: &Value) -> bool {
    value
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn task_matches(value: &Value, task_key: Option<&str>) -> bool {
    let Some(want) = task_key else {
        return true;
    };
    let raw = value
        .get("taskType")
        .or_else(|| value.get("task_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if raw.is_empty() {
        return true;
    }
    raw == want || raw.replace('_', "") == want.replace('_', "")
}

fn task_type_key(task: WritingTaskType) -> &'static str {
    match task {
        WritingTaskType::Task1 => "task1",
        WritingTaskType::Task2 => "task2",
    }
}

fn extract_prompt_body(value: &Value) -> Option<String> {
    for key in ["body", "content", "system", "systemPrompt", "prompt", "text"] {
        if let Some(s) = value.get(key).and_then(|v| v.as_str()) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn resolve_temperature(conn: &Connection, task_type: Option<WritingTaskType>) -> DbResult<f32> {
    let mode = read_string_setting(conn, NS_MODEL, "temperature_mode")?
        .unwrap_or_else(|| "balanced".into())
        .to_ascii_lowercase();
    let (t1, t2) = match mode.as_str() {
        "precise" => (0.3, 0.3),
        "creative" => (0.8, 0.8),
        "custom" => (
            read_f32_setting(conn, NS_MODEL, "temperature_task1")?.unwrap_or(0.3),
            read_f32_setting(conn, NS_MODEL, "temperature_task2")?.unwrap_or(0.5),
        ),
        // balanced + unknown
        _ => (0.5, 0.5),
    };
    let temp = match task_type {
        Some(WritingTaskType::Task1) => t1,
        Some(WritingTaskType::Task2) => t2,
        None => t2,
    };
    Ok(temp.clamp(0.0, 2.0))
}

fn read_string_setting(conn: &Connection, namespace: &str, key: &str) -> DbResult<Option<String>> {
    let Some(entry) = get_setting(conn, namespace, key)? else {
        return Ok(None);
    };
    Ok(match entry.value {
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => {
            let raw = other.to_string();
            Some(raw.trim_matches('"').to_string())
        }
    })
}

fn read_f32_setting(conn: &Connection, namespace: &str, key: &str) -> DbResult<Option<f32>> {
    let Some(entry) = get_setting(conn, namespace, key)? else {
        return Ok(None);
    };
    Ok(match entry.value {
        Value::Number(n) => n.as_f64().map(|v| v as f32),
        Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::upsert_setting;
    use rusqlite::Connection;

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

    #[test]
    fn resolves_custom_temperature_and_active_prompt() {
        let conn = connection();
        upsert_setting(
            &conn,
            NS_MODEL,
            "temperature_mode",
            &Value::String("custom".into()),
        )
        .unwrap();
        upsert_setting(&conn, NS_MODEL, "temperature_task1", &Value::from(0.25)).unwrap();
        upsert_setting(&conn, NS_MODEL, "temperature_task2", &Value::from(0.75)).unwrap();
        upsert_setting(
            &conn,
            NS_PROMPTS,
            "p1",
            &serde_json::json!({
                "id": "p1",
                "active": true,
                "taskType": "task2",
                "body": "You are a strict IELTS examiner."
            }),
        )
        .unwrap();

        let policy = resolve_writing_eval_policy(&conn, Some(WritingTaskType::Task2)).unwrap();
        assert!((policy.temperature - 0.75).abs() < f32::EPSILON);
        assert!(policy.system_prompt.contains("strict IELTS"));
        assert_eq!(policy.prompt_id.as_deref(), Some("p1"));
    }
}
