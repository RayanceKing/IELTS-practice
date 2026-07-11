use serde::Serialize;
use tauri::State;

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
            "Phase 2 shell: Vue UI only; domain services not fully migrated.".into(),
            "No localhost Fastify business API is started.".into(),
            "User database migration is deferred to Phase 3.".into(),
            "Updater plugin is present; endpoints/pubkey inactive until release signing.".into(),
        ],
    }
}
