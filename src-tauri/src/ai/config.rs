use std::collections::HashSet;
use std::time::Duration;

use ielts_db::{get_setting, list_secret_refs, DbError, DbResult, NS_AI};
use ielts_domain::dto::{AiConfigDto, SecretRef};

use crate::app::state::{AppDb, AppVault};

use super::{AiProviderConfig, AiRuntime};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_TIMEOUT_SECONDS: u64 = 45;
const API_KEY_REQUIRED_ON_THIS_DEVICE: &str =
    "当前设备未找到可用 API Key；请在设置中重新填写该配置的 API Key 后再使用";

fn provider_defaults(provider: &str) -> (&'static str, &'static str) {
    match provider.trim().to_ascii_lowercase().as_str() {
        "openrouter" => ("https://openrouter.ai/api/v1", "openai-compatible"),
        "deepseek" => ("https://api.deepseek.com/v1", "openai-compatible"),
        "openai" => (DEFAULT_BASE_URL, "openai-compatible"),
        _ => (DEFAULT_BASE_URL, "openai-compatible"),
    }
}

pub(crate) fn normalize_provider(provider: &str, base_url: Option<&str>) -> (String, String) {
    let (default_url, normalized) = provider_defaults(provider);
    let url = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_url);
    (normalized.to_string(), url.to_string())
}

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

fn available_secret_ref_ids(db: &AppDb, vault: &AppVault) -> DbResult<HashSet<String>> {
    let refs = db.with_conn(list_secret_refs)?;
    Ok(refs
        .into_iter()
        .filter_map(|reference| vault_has_secret(vault, &reference).then_some(reference.ref_id))
        .collect())
}

fn reconcile_default_ai_config_with_refs(
    conn: &rusqlite::Connection,
    available_refs: &HashSet<String>,
) -> DbResult<Option<AiConfigDto>> {
    ielts_db::reconcile_default_ai_config_with_secret_availability(conn, |reference| {
        available_refs.contains(&reference.ref_id)
    })
}

pub(crate) fn list_ai_configs_with_vault(
    db: &AppDb,
    vault: &AppVault,
) -> DbResult<Vec<AiConfigDto>> {
    let available_refs = available_secret_ref_ids(db, vault)?;
    db.with_conn(|conn| list_ai_configs_with_refs(conn, &available_refs))
}

fn list_ai_configs_with_refs(
    conn: &rusqlite::Connection,
    available_refs: &HashSet<String>,
) -> DbResult<Vec<AiConfigDto>> {
    reconcile_default_ai_config_with_refs(conn, available_refs)?;
    let mut configs = ielts_db::list_ai_configs_with_secret_availability(conn, |reference| {
        available_refs.contains(&reference.ref_id)
    })?;
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

pub(crate) fn load_provider_config(db: &AppDb, vault: &AppVault) -> DbResult<AiProviderConfig> {
    let available_refs = available_secret_ref_ids(db, vault)?;
    db.with_conn(|conn| {
        let config = reconcile_default_ai_config_with_refs(conn, &available_refs)?
            .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))?;
        provider_config_for_config(conn, &config)
    })
}

fn load_provider_config_for_id(
    db: &AppDb,
    vault: &AppVault,
    config_id: &str,
) -> DbResult<AiProviderConfig> {
    let available_refs = available_secret_ref_ids(db, vault)?;
    db.with_conn(|conn| {
        let configs = ielts_db::list_ai_configs_with_secret_availability(conn, |reference| {
            available_refs.contains(&reference.ref_id)
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

fn resolve_secret_ref(conn: &rusqlite::Connection, name: &str) -> DbResult<SecretRef> {
    list_secret_refs(conn)?
        .into_iter()
        .find(|secret_ref| secret_ref.name == name)
        .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))
}

fn resolve_api_key(vault: &AppVault, secret_ref: &SecretRef) -> DbResult<String> {
    vault
        .0
        .get_secret_by_ref(&secret_ref.ref_id)?
        .filter(|secret| !secret.trim().is_empty())
        .ok_or_else(|| DbError::Validation(API_KEY_REQUIRED_ON_THIS_DEVICE.into()))
}

pub(crate) fn load_runtime(db: &AppDb, vault: &AppVault) -> DbResult<AiRuntime> {
    let config = load_provider_config(db, vault)?;
    load_runtime_from_provider_config(db, vault, config)
}

pub(crate) fn load_runtime_for_config(
    db: &AppDb,
    vault: &AppVault,
    config_id: &str,
) -> DbResult<AiRuntime> {
    let config = load_provider_config_for_id(db, vault, config_id)?;
    load_runtime_from_provider_config(db, vault, config)
}

pub(crate) fn load_runtime_from_provider_config(
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
    let secret_ref = db.with_conn(|conn| resolve_secret_ref(conn, &config.secret_name))?;
    let api_key = resolve_api_key(vault, &secret_ref)?;
    AiRuntime::new(config, api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn provider_test_selects_requested_config_even_when_disabled() {
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
        let target = select_config_for_test(vec![selected], "selected").unwrap();
        assert_eq!(target.default_model, "gpt-selected");
        assert!(!target.is_enabled);
    }
}
