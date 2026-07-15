//! Shared AI runtime. Provider configuration and credentials are loaded while
//! the database is locked; network calls only receive owned values.

use std::time::{Duration, Instant};

use ielts_domain::dto::{AiConfigDto, AiUpsertConfigCommand, CommandResponse};
use ielts_domain::ErrorEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

use crate::app::state::{AppDb, AppVault};
use ielts_db::{
    ai_secret_name, get_setting, legacy_ai_secret_name, list_secret_refs, DbError, DbResult,
    ProviderError, NS_AI,
};
use uuid::Uuid;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_TIMEOUT_SECONDS: u64 = 45;
const MAX_RETRIES: u32 = 2;

fn provider_defaults(provider: &str) -> (&'static str, &'static str) {
    match provider.trim().to_ascii_lowercase().as_str() {
        "openrouter" => ("https://openrouter.ai/api/v1", "openai-compatible"),
        "deepseek" => ("https://api.deepseek.com/v1", "openai-compatible"),
        "openai" => (DEFAULT_BASE_URL, "openai-compatible"),
        _ => (DEFAULT_BASE_URL, "openai-compatible"),
    }
}

fn normalize_provider(provider: &str, base_url: Option<&str>) -> (String, String) {
    let (default_url, normalized) = provider_defaults(provider);
    let url = base_url
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(default_url);
    (normalized.to_string(), url.to_string())
}

fn supported_provider(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "openai" | "openrouter" | "deepseek"
    )
}

fn config_error(error: DbError) -> ErrorEnvelope {
    ErrorEnvelope::new("ai.configuration", error.to_string(), false)
}

#[tauri::command]
pub fn ai_list_configs(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
) -> CommandResponse<Vec<AiConfigDto>> {
    match db.with_conn(|conn| list_ai_configs_with_vault(conn, vault.inner())) {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(config_error(e)),
    }
}

#[tauri::command]
pub fn ai_upsert_config(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    cmd: AiUpsertConfigCommand,
) -> CommandResponse<AiConfigDto> {
    if cmd.config_name.trim().is_empty() || cmd.default_model.trim().is_empty() {
        return CommandResponse::failure(config_error(DbError::Validation(
            "config name and default model are required".into(),
        )));
    }
    if !supported_provider(&cmd.provider) {
        return CommandResponse::failure(config_error(DbError::Validation(
            "unsupported AI provider".into(),
        )));
    }
    let id = cmd.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let (_, base_url) = normalize_provider(&cmd.provider, cmd.base_url.as_deref());
    let secret_name = ai_secret_name(&id);
    let vault = vault.inner();
    if let Some(secret) = cmd.api_key.as_deref().filter(|v| !v.trim().is_empty()) {
        let ref_id = match vault.0.set_secret(&secret_name, secret) {
            Ok(v) => v,
            Err(e) => return CommandResponse::failure(config_error(e)),
        };
        if let Err(e) =
            db.with_conn(|conn| ielts_db::put_secret_ref(conn, &secret_name, &ref_id).map(|_| ()))
        {
            return CommandResponse::failure(config_error(e));
        }
    }
    let config = AiConfigDto {
        id: id.clone(),
        config_name: cmd.config_name.trim().to_string(),
        provider: cmd.provider.trim().to_ascii_lowercase(),
        base_url,
        default_model: cmd.default_model.trim().to_string(),
        is_default: false,
        is_enabled: cmd.is_enabled,
        has_secret: false,
    };
    let result = db.with_conn(|conn| {
        ielts_db::upsert_ai_config(conn, &config)?;
        list_ai_configs_with_vault(conn, vault)?
            .into_iter()
            .find(|item| item.id == config.id)
            .ok_or_else(|| DbError::Message("AI config disappeared after save".into()))
    });
    match result {
        Ok(config) => CommandResponse::success(config),
        Err(e) => CommandResponse::failure(config_error(e)),
    }
}

#[tauri::command]
pub fn ai_set_default_config(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    id: String,
) -> CommandResponse<AiConfigDto> {
    let vault = vault.inner();
    let result = db.with_conn(|conn| {
        let mut config = list_ai_configs_with_vault(conn, vault)?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| DbError::Validation("AI config not found".into()))?;
        if !config.is_enabled {
            return Err(DbError::Validation("AI config is disabled".into()));
        }
        if !config.has_secret {
            return Err(DbError::Validation("AI config has no API key".into()));
        }
        ielts_db::set_default_ai_config(conn, Some(&config))?;
        config.is_default = true;
        Ok(config)
    });
    match result {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(config_error(e)),
    }
}

#[tauri::command]
pub fn ai_delete_config(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    id: String,
) -> CommandResponse<bool> {
    let secret_name = ai_secret_name(&id);
    let legacy_secret_name = legacy_ai_secret_name(&id);
    let vault = vault.inner();
    let result = db.with_conn(|conn| {
        let deleted = ielts_db::delete_ai_config(conn, &id)?;
        ielts_db::delete_secret_ref(conn, &secret_name)?;
        ielts_db::delete_secret_ref(conn, &legacy_secret_name)?;
        list_ai_configs_with_vault(conn, vault)?;
        Ok(deleted)
    });
    if result.is_ok() {
        let _ = vault.0.delete_secret(&secret_name);
        let _ = vault.0.delete_secret(&legacy_secret_name);
    }
    match result {
        Ok(v) => CommandResponse::success(v),
        Err(e) => CommandResponse::failure(config_error(e)),
    }
}

#[derive(Debug, Clone)]
pub struct AiProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub secret_name: String,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct AiRuntime {
    pub config: AiProviderConfig,
    pub api_key: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderTestResult {
    pub provider: String,
    pub model: String,
    pub reachable: bool,
    pub authenticated: bool,
    pub latency_ms: u64,
}

const API_KEY_REQUIRED_ON_THIS_DEVICE: &str =
    "当前设备未找到可用 API Key；请在设置中重新填写该配置的 API Key 后再使用";

fn vault_has_secret(vault: &AppVault, reference: &ielts_domain::dto::SecretRef) -> bool {
    match vault.0.get_secret_by_ref(&reference.ref_id) {
        Ok(Some(secret)) => !secret.trim().is_empty(),
        Ok(None) => false,
        Err(error) => {
            tracing::warn!(error = %error, "AI credential is unavailable in the local OS vault");
            false
        }
    }
}

/// Reconcile the SQLite default with the credentials available on this host.
/// A backup only restores opaque references; the vault check is what makes a
/// same-device restore usable and a cross-device restore fail closed.
pub fn reconcile_default_ai_config_with_vault(
    conn: &rusqlite::Connection,
    vault: &AppVault,
) -> DbResult<Option<AiConfigDto>> {
    ielts_db::reconcile_default_ai_config_with_secret_availability(conn, |reference| {
        vault_has_secret(vault, reference)
    })
}

pub fn list_ai_configs_with_vault(
    conn: &rusqlite::Connection,
    vault: &AppVault,
) -> DbResult<Vec<AiConfigDto>> {
    reconcile_default_ai_config_with_vault(conn, vault)?;
    let mut configs = ielts_db::list_ai_configs_with_secret_availability(conn, |reference| {
        vault_has_secret(vault, reference)
    })?;
    // The vault can change between reconciliation and presentation. Do not
    // ever return a row that looks both default and unusable, even in that
    // narrow race; the next successful save/list will select a replacement.
    if configs
        .iter()
        .any(|config| config.is_default && (!config.is_enabled || !config.has_secret))
    {
        ielts_db::set_default_ai_config(conn, None)?;
        for config in &mut configs {
            config.is_default = false;
        }
    }
    Ok(configs)
}

fn provider_config_for_config(
    conn: &rusqlite::Connection,
    config: &AiConfigDto,
) -> DbResult<AiProviderConfig> {
    if !config.has_secret {
        return Err(DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()));
    }
    let (provider, base_url) = normalize_provider(&config.provider, Some(&config.base_url));
    let secret_name = ielts_db::ai_secret_ref_for_config(conn, &config.id)?
        .map(|reference| reference.name)
        .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))?;
    let timeout_seconds = get_setting(conn, NS_AI, "timeoutSeconds")?
        .and_then(|entry| entry.value.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
        .clamp(5, 300);
    Ok(AiProviderConfig {
        provider,
        base_url,
        model: config.default_model.clone(),
        secret_name,
        timeout: Duration::from_secs(timeout_seconds),
    })
}

/// Preflight the currently selected runtime configuration before any writing
/// attempt is submitted. This deliberately verifies the local vault, not only
/// a database secret reference restored from a backup.
pub fn load_provider_config(db: &AppDb, vault: &AppVault) -> DbResult<AiProviderConfig> {
    db.with_conn(|conn| {
        let config = reconcile_default_ai_config_with_vault(conn, vault)?
            .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))?;
        provider_config_for_config(conn, &config)
    })
}

fn load_provider_config_for_id(
    db: &AppDb,
    vault: &AppVault,
    config_id: &str,
) -> DbResult<AiProviderConfig> {
    db.with_conn(|conn| {
        let configs = ielts_db::list_ai_configs_with_secret_availability(conn, |reference| {
            vault_has_secret(vault, reference)
        })?;
        let config = select_config_for_test(configs, config_id)?;
        provider_config_for_config(conn, &config)
    })
}

fn select_config_for_test(configs: Vec<AiConfigDto>, config_id: &str) -> DbResult<AiConfigDto> {
    configs
        .into_iter()
        .find(|config| config.id == config_id)
        .ok_or_else(|| DbError::Validation("AI config not found".into()))
}

pub fn resolve_api_key(
    conn: &rusqlite::Connection,
    vault: &AppVault,
    name: &str,
) -> DbResult<String> {
    let secret_ref = list_secret_refs(conn)?
        .into_iter()
        .find(|secret_ref| secret_ref.name == name)
        .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))?;
    vault
        .0
        .get_secret_by_ref(&secret_ref.ref_id)?
        .filter(|secret| !secret.trim().is_empty())
        .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))
}

pub fn load_runtime(db: &AppDb, vault: &AppVault) -> DbResult<AiRuntime> {
    let config = load_provider_config(db, vault)?;
    load_runtime_from_provider_config(db, vault, config)
}

fn load_runtime_for_config(db: &AppDb, vault: &AppVault, config_id: &str) -> DbResult<AiRuntime> {
    let config = load_provider_config_for_id(db, vault, config_id)?;
    load_runtime_from_provider_config(db, vault, config)
}

fn load_runtime_from_provider_config(
    db: &AppDb,
    vault: &AppVault,
    config: AiProviderConfig,
) -> DbResult<AiRuntime> {
    if config.provider != "openai-compatible" {
        return Err(DbError::Validation(format!(
            "provider does not support network AI requests: {}",
            config.provider
        )));
    }
    let api_key = db.with_conn(|conn| resolve_api_key(conn, vault, &config.secret_name))?;
    let client = reqwest::Client::builder()
        .timeout(config.timeout)
        .build()
        .map_err(|error| DbError::Message(format!("failed to build AI HTTP client: {error}")))?;
    Ok(AiRuntime {
        config,
        api_key,
        client,
    })
}

impl AiRuntime {
    pub async fn chat_completion(
        &self,
        messages: Value,
        temperature: f32,
    ) -> Result<String, ProviderError> {
        let endpoint = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let body = json!({
            "model": self.config.model,
            "temperature": temperature,
            "response_format": { "type": "json_object" },
            "messages": messages
        });

        for attempt in 0..=MAX_RETRIES {
            let response = self
                .client
                .post(&endpoint)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let envelope: ChatResponse = response.json().await.map_err(|error| {
                        provider_error(format!("AI response envelope invalid: {error}"), false)
                    })?;
                    return envelope
                        .choices
                        .into_iter()
                        .next()
                        .map(|choice| choice.message.content)
                        .ok_or_else(|| {
                            provider_error("AI response contained no choices".into(), false)
                        });
                }
                Ok(response) => {
                    let status = response.status();
                    let retryable = status.is_server_error() || status.as_u16() == 429;
                    if retryable && attempt < MAX_RETRIES {
                        retry_delay(attempt).await;
                        continue;
                    }
                    return Err(provider_error(
                        format!("AI provider returned HTTP {}", status.as_u16()),
                        retryable,
                    ));
                }
                Err(error) => {
                    let retryable = error.is_timeout() || error.is_connect();
                    if retryable && attempt < MAX_RETRIES {
                        retry_delay(attempt).await;
                        continue;
                    }
                    return Err(provider_error(
                        format!("AI request failed: {error}"),
                        retryable,
                    ));
                }
            }
        }
        unreachable!("retry loop always returns")
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: Message,
}
#[derive(Deserialize)]
struct Message {
    content: String,
}

async fn retry_delay(attempt: u32) {
    tokio::time::sleep(Duration::from_millis(250 * 2u64.pow(attempt))).await;
}

pub fn provider_error(message: String, retryable: bool) -> ProviderError {
    ProviderError { message, retryable }
}

fn map_err(error: DbError) -> ErrorEnvelope {
    ErrorEnvelope::new("ai.configuration", error.to_string(), false)
}

#[tauri::command]
pub async fn ai_test_provider(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    config_id: String,
) -> Result<CommandResponse<AiProviderTestResult>, ErrorEnvelope> {
    let runtime = match load_runtime_for_config(&db, &vault, &config_id) {
        Ok(runtime) => runtime,
        Err(error) => return Ok(CommandResponse::failure(map_err(error))),
    };
    let started = Instant::now();
    let result = runtime
        .chat_completion(
            json!([
                { "role": "system", "content": "Return JSON only." },
                { "role": "user", "content": "Return exactly {\"ok\":true}." }
            ]),
            0.0,
        )
        .await;
    match result {
        Ok(content) => {
            let valid = serde_json::from_str::<Value>(&content)
                .ok()
                .and_then(|value| value.get("ok").and_then(Value::as_bool))
                == Some(true);
            if !valid {
                return Ok(CommandResponse::failure(ErrorEnvelope::new(
                    "ai.invalid_test_response",
                    "AI provider returned an invalid connectivity response",
                    false,
                )));
            }
            Ok(CommandResponse::success(AiProviderTestResult {
                provider: runtime.config.provider,
                model: runtime.config.model,
                reachable: true,
                authenticated: true,
                latency_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            }))
        }
        Err(error) => Ok(CommandResponse::failure(ErrorEnvelope::new(
            "ai.provider_test_failed",
            error.message,
            error.retryable,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_error_does_not_expose_credentials() {
        let error = provider_error("AI provider returned HTTP 401".into(), false);
        assert_eq!(error.message, "AI provider returned HTTP 401");
    }

    #[test]
    fn named_providers_map_to_openai_compatible_endpoints() {
        for (provider, expected) in [
            ("openai", "https://api.openai.com/v1"),
            ("openrouter", "https://openrouter.ai/api/v1"),
            ("deepseek", "https://api.deepseek.com/v1"),
        ] {
            let (runtime_provider, base_url) = normalize_provider(provider, None);
            assert_eq!(runtime_provider, "openai-compatible");
            assert_eq!(base_url, expected);
        }
    }

    #[test]
    fn provider_test_selects_the_requested_config_instead_of_the_default() {
        let default = AiConfigDto {
            id: "default".into(),
            config_name: "Default".into(),
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-default".into(),
            is_default: true,
            is_enabled: true,
            has_secret: true,
        };
        let selected = AiConfigDto {
            id: "selected".into(),
            config_name: "Selected".into(),
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            default_model: "gpt-selected".into(),
            is_default: false,
            is_enabled: false,
            has_secret: true,
        };

        let target = select_config_for_test(vec![default, selected.clone()], "selected").unwrap();
        assert_eq!(target.id, "selected");
        assert_eq!(target.default_model, "gpt-selected");
        assert!(!target.is_default);
    }
}
