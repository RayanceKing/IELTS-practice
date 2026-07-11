//! Tauri 2 shell for IELTS Practice.
//!
//! Phase 2 goals:
//! - boot existing Vue UI without localhost Fastify
//! - minimal capabilities
//! - diagnostics / path discovery / legacy route helpers
//! - no user-database migration yet

pub mod app;
pub mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::logging::init();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app::state::AppPaths::discover())
        .invoke_handler(tauri::generate_handler![
            commands::diagnostics::get_app_info,
            commands::diagnostics::get_startup_diagnostics,
            commands::paths::get_app_data_paths,
            commands::paths::discover_legacy_data_dirs,
            commands::routes::normalize_shell_route,
            commands::routes::resolve_legacy_route,
        ])
        .setup(|app| {
            let paths = app.state::<app::state::AppPaths>();
            if let Err(err) = paths.ensure_layout() {
                tracing::error!(error = %err, "failed to ensure app data layout");
            }
            tracing::info!(
                app_data = %paths.app_data.display(),
                legacy_candidates = paths.legacy_candidates.len(),
                "IELTS Practice Tauri shell ready (no Fastify localhost API)"
            );
            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running IELTS Practice Tauri application");
}
