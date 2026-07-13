use ielts_db::{
    delete_secret_ref, get_setting, list_secret_refs, migrate::open_and_migrate, put_secret_ref,
    set_default_ai_config, upsert_ai_config, upsert_setting, NS_AI,
};
use ielts_domain::dto::AiConfigDto;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn provider_config_rejects_nested_plaintext_credentials() {
    let dir = tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("v2.db")).unwrap();
    let legacy_provider = json!({
        "id": "legacy-openai",
        "provider": "openai",
        "base_url": "https://api.openai.com/v1",
        "default_model": "gpt-4o-mini",
        "api_key": "sk-legacy-plaintext-must-never-reach-v2"
    });

    let error = upsert_setting(&conn, "provider_configs", "legacy-openai", &legacy_provider)
        .expect_err("nested API keys must be rejected");
    assert!(error.to_string().contains("secret") || error.to_string().contains("API key"));

    let stored: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE value_json LIKE '%sk-legacy%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, 0);
}

#[test]
fn active_runtime_config_is_complete_and_contains_only_a_secret_name() {
    let dir = tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("v2.db")).unwrap();
    let config = AiConfigDto {
        id: "legacy-openai".into(),
        config_name: "Legacy OpenAI".into(),
        provider: "openai".into(),
        base_url: "https://api.openai.com/v1".into(),
        default_model: "gpt-4o-mini".into(),
        is_default: false,
        is_enabled: true,
        has_secret: true,
    };
    upsert_ai_config(&conn, &config).unwrap();
    set_default_ai_config(&conn, Some(&config)).unwrap();
    put_secret_ref(&conn, "ai.config.legacy-openai", "keyring:legacy-openai").unwrap();

    for key in [
        "provider",
        "baseUrl",
        "model",
        "secretName",
        "timeoutSeconds",
    ] {
        assert!(
            get_setting(&conn, NS_AI, key).unwrap().is_some(),
            "missing {key}"
        );
    }
    let dump: String = conn
        .query_row(
            "SELECT group_concat(value_json, '|') FROM settings",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!dump.contains("sk-"));
    assert!(dump.contains("legacy-openai.api_key"));
}

#[test]
fn deleting_secret_reference_removes_only_the_reference_record() {
    let dir = tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("v2.db")).unwrap();
    put_secret_ref(&conn, "first.api_key", "keyring:first").unwrap();
    put_secret_ref(&conn, "second.api_key", "keyring:second").unwrap();

    assert!(delete_secret_ref(&conn, "first.api_key").unwrap());
    assert!(!delete_secret_ref(&conn, "first.api_key").unwrap());
    let refs = list_secret_refs(&conn).unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "second.api_key");
}
