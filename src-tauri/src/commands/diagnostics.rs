use serde::Serialize;
use tauri::State;
use tauri_plugin_updater::UpdaterExt;

use crate::app::state::AppPaths;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub tauri_version: String,
    pub product_name: String,
    pub host: String,
    pub fastify_enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupDiagnostics {
    pub boot_id: String,
    pub started_at: String,
    pub app_data: String,
    pub logs_dir: String,
    pub legacy_data_dirs: Vec<String>,
    pub fastify_enabled: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub configured: bool,
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub message: String,
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdaterStatus, String> {
    let config: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json"))
        .map_err(|error| format!("invalid updater configuration: {error}"))?;
    let updater = &config["plugins"]["updater"];
    let configured = updater["active"].as_bool().unwrap_or(false)
        && updater["endpoints"].as_array().is_some_and(|items| !items.is_empty())
        && updater["pubkey"].as_str().is_some_and(|key| !key.trim().is_empty());
    if !configured {
        return Ok(UpdaterStatus {
            configured,
            update_available: false,
            current_version: env!("CARGO_PKG_VERSION").into(),
            latest_version: None,
            message: "自动更新尚未配置发布端点和签名公钥。".into(),
        });
    }

    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?;
    Ok(UpdaterStatus {
        configured,
        update_available: update.is_some(),
        current_version: env!("CARGO_PKG_VERSION").into(),
        latest_version: update.map(|item| item.version),
        message: "更新检查完成。".into(),
    })
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "ielts-practice".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        tauri_version: tauri::VERSION.into(),
        product_name: "IELTS Practice".into(),
        host: "tauri".into(),
        // Explicit: Phase 2 shell must not start localhost business API.
        fastify_enabled: false,
    }
}

#[tauri::command]
pub fn get_startup_diagnostics(paths: State<'_, AppPaths>) -> StartupDiagnostics {
    StartupDiagnostics {
        boot_id: uuid::Uuid::new_v4().to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        app_data: paths.app_data.display().to_string(),
        logs_dir: paths.logs.display().to_string(),
        legacy_data_dirs: paths
            .legacy_candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        fastify_enabled: false,
        notes: vec![
            "Phase 10 cutover: Tauri/Rust is the only shipping runtime.".into(),
            "No localhost Fastify business API is started.".into(),
            "SQLite v2 is primary store; legacy importers remain for one-shot migration.".into(),
            "Shadow dual-read is test-only; production path is single-source.".into(),
            "Updater plugin is present; endpoints/pubkey inactive until release signing.".into(),
        ],
    }
}

#[tauri::command]
pub fn get_performance_budgets() -> ielts_domain::dto::CommandResponse<serde_json::Value> {
    let b = ielts_db::DEFAULT_BUDGETS;
    ielts_domain::dto::CommandResponse::success(serde_json::json!({
        "coldStartInteractiveMs": b.cold_start_interactive_ms,
        "warmStartInteractiveMs": b.warm_start_interactive_ms,
        "libraryFirstPaintMs": b.library_first_paint_ms,
        "answerLocalSaveMs": b.answer_local_save_ms,
        "historyFirstPageMs": b.history_first_page_ms,
        "resultOpenMs": b.result_open_ms,
        "evaluationUiLatencyMs": b.evaluation_ui_latency_ms
    }))
}

#[tauri::command]
pub fn get_query_plan_baselines(
    db: tauri::State<'_, crate::app::state::AppDb>,
) -> ielts_domain::dto::CommandResponse<Vec<ielts_db::QueryPlanBaseline>> {
    match db.with_conn(|conn| ielts_db::collect_query_plan_baselines(conn)) {
        Ok(v) => ielts_domain::dto::CommandResponse::success(v),
        Err(e) => ielts_domain::dto::CommandResponse::failure(ielts_domain::ErrorEnvelope {
            code: "perf.query_plan".into(),
            message: e.to_string(),
            retryable: false,
            context: None,
            cause_id: None,
        }),
    }
}
