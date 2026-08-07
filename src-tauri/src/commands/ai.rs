//! AI configuration IPC commands.

use ielts_application::{ChatMessage, CompletionRequest, LanguageModel};
use ielts_domain::dto::{AiConfigDto, AiUpsertConfigCommand, CommandResponse};
use ielts_domain::ErrorEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use uuid::Uuid;

use crate::ai::{list_ai_configs_with_vault, load_runtime_for_config, normalize_provider};
use crate::app::state::{AppDb, AppVault};
use ielts_db::{ai_secret_name, legacy_ai_secret_name, DbError};

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
    match list_ai_configs_with_vault(db.inner(), vault.inner()) {
        Ok(configs) => CommandResponse::success(configs),
        Err(error) => CommandResponse::failure(config_error(error)),
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
    if let Some(secret) = cmd
        .api_key
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let ref_id = match vault.0.set_secret(&secret_name, secret) {
            Ok(ref_id) => ref_id,
            Err(error) => return CommandResponse::failure(config_error(error)),
        };
        if let Err(error) =
            db.with_conn(|conn| ielts_db::put_secret_ref(conn, &secret_name, &ref_id).map(|_| ()))
        {
            return CommandResponse::failure(config_error(error));
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
    let result = db
        .with_conn(|conn| ielts_db::upsert_ai_config(conn, &config))
        .and_then(|_| {
            list_ai_configs_with_vault(db.inner(), vault)?
                .into_iter()
                .find(|item| item.id == config.id)
                .ok_or_else(|| DbError::Message("AI config disappeared after save".into()))
        });
    match result {
        Ok(config) => CommandResponse::success(config),
        Err(error) => CommandResponse::failure(config_error(error)),
    }
}

#[tauri::command]
pub fn ai_set_default_config(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    id: String,
) -> CommandResponse<AiConfigDto> {
    let vault = vault.inner();
    let result = list_ai_configs_with_vault(db.inner(), vault)
        .and_then(|configs| {
            configs
                .into_iter()
                .find(|config| config.id == id)
                .ok_or_else(|| DbError::Validation("AI config not found".into()))
        })
        .and_then(|mut config| {
            if !config.is_enabled {
                return Err(DbError::Validation("AI config is disabled".into()));
            }
            if !config.has_secret {
                return Err(DbError::Validation("AI config has no API key".into()));
            }
            db.with_conn(|conn| ielts_db::set_default_ai_config(conn, Some(&config)))?;
            config.is_default = true;
            Ok(config)
        });
    match result {
        Ok(config) => CommandResponse::success(config),
        Err(error) => CommandResponse::failure(config_error(error)),
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
        Ok(deleted)
    });
    let result = match result {
        Ok(deleted) => list_ai_configs_with_vault(db.inner(), vault)
            .map(|_| deleted)
            .map(|deleted| {
                let _ = vault.0.delete_secret(&secret_name);
                let _ = vault.0.delete_secret(&legacy_secret_name);
                deleted
            }),
        Err(error) => Err(error),
    };
    match result {
        Ok(deleted) => CommandResponse::success(deleted),
        Err(error) => CommandResponse::failure(config_error(error)),
    }
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

#[tauri::command]
pub async fn ai_test_provider(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    config_id: String,
) -> Result<CommandResponse<AiProviderTestResult>, ErrorEnvelope> {
    let runtime = match load_runtime_for_config(&db, &vault, &config_id) {
        Ok(runtime) => runtime,
        Err(error) => return Ok(CommandResponse::failure(config_error(error))),
    };
    let response = runtime
        .complete(CompletionRequest {
            messages: vec![
                ChatMessage::new("system", "Return JSON only."),
                ChatMessage::new("user", "Return exactly {\"ok\":true}."),
            ],
            temperature: 0.0,
        })
        .await;
    match response {
        Ok(response) => {
            let valid = serde_json::from_str::<Value>(&response.content)
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
                latency_ms: response.latency_ms,
            }))
        }
        Err(error) => Ok(CommandResponse::failure(ErrorEnvelope::new(
            "ai.provider_test_failed",
            error.message,
            error.retryable,
        ))),
    }
}
