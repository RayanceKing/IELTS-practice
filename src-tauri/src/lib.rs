//! Tauri 2 shell for IELTS Practice.
//!
//! Phase 2: boot Vue UI without Fastify; diagnostics / paths / routes.
//! Phase 4: unified history, settings, backup, secret-ref commands.

pub mod app;
pub mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::logging::init();

    let paths = app::state::AppPaths::discover();
    if let Err(err) = paths.ensure_layout() {
        tracing::error!(error = %err, "failed to ensure app data layout");
    }

    let db = match app::state::AppDb::open(&paths) {
        Ok(db) => db,
        Err(err) => {
            tracing::error!(error = %err, "failed to open v2 database");
            panic!("failed to open v2 database: {err}");
        }
    };
    let vault = match app::state::AppVault::open(&paths) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!(error = %err, "failed to open secret vault");
            panic!("failed to open secret vault: {err}");
        }
    };

    let builder = tauri::Builder::default()
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
        .manage(paths)
        .manage(db)
        .manage(vault)
        .invoke_handler(tauri::generate_handler![
            commands::ai::ai_test_provider,
            commands::ai::ai_list_configs,
            commands::ai::ai_upsert_config,
            commands::ai::ai_delete_config,
            commands::ai::ai_set_default_config,
            commands::diagnostics::get_app_info,
            commands::diagnostics::check_for_updates,
            commands::diagnostics::get_startup_diagnostics,
            commands::diagnostics::get_performance_budgets,
            commands::diagnostics::get_query_plan_baselines,
            commands::paths::get_app_data_paths,
            commands::paths::discover_legacy_data_dirs,
            commands::routes::normalize_shell_route,
            commands::routes::resolve_legacy_route,
            commands::history::list_history,
            commands::history::get_history_detail,
            commands::history::export_history,
            commands::history::delete_history_attempt,
            commands::history::import_reading_archive_value,
            commands::settings::list_settings,
            commands::settings::upsert_setting,
            commands::settings::migrate_local_preferences,
            commands::settings::set_secret,
            commands::settings::list_secret_refs,
            commands::settings::delete_secret,
            commands::backup::create_backup,
            commands::backup::import_backup_path,
            commands::writing::writing_save_draft,
            commands::writing::writing_get_draft,
            commands::writing::writing_submit_attempt,
            commands::writing::writing_start_evaluation,
            commands::writing::writing_list_evaluation_events,
            commands::writing::writing_cancel_evaluation,
            commands::writing::writing_get_evaluation,
            commands::reading::reading_list_assets,
            commands::reading::reading_get_asset_payload,
            commands::reading::reading_save_draft,
            commands::reading::reading_get_open_draft,
            commands::reading::reading_patch_answer,
            commands::reading::reading_submit_attempt,
            commands::modes::suite_create,
            commands::modes::suite_get,
            commands::modes::suite_submit_passage,
            commands::modes::suite_cancel,
            commands::modes::endless_create,
            commands::modes::endless_get,
            commands::modes::endless_advance,
            commands::modes::endless_submit,
            commands::modes::memorize_create,
            commands::modes::memorize_finish,
            commands::modes::timer_elapsed_seconds,
            commands::modes::timer_should_auto_submit,
            commands::enrichment::annotation_upsert,
            commands::enrichment::annotation_list,
            commands::enrichment::annotation_delete,
            commands::enrichment::annotation_revalidate,
            commands::enrichment::dictionary_lookup,
            commands::enrichment::dictionary_import,
            commands::enrichment::vocab_upsert,
            commands::enrichment::vocab_list,
            commands::enrichment::vocab_review,
            commands::enrichment::vocab_delete,
            commands::enrichment::coach_ensure_thread,
            commands::enrichment::coach_append_message,
            commands::enrichment::coach_list_messages,
            commands::enrichment::coach_record_failure,
            commands::enrichment::coach_run,
        ])
        .setup(|app| {
            let paths = app.state::<app::state::AppPaths>();
            let db = app.state::<app::state::AppDb>();
            let reading_pack = app.path().resource_dir()?.join("reading");
            let seed_report =
                db.with_conn(|conn| ielts_db::seed_builtin_reading_pack(conn, &reading_pack))?;
            tracing::info!(
                pack_id = %seed_report.pack_id,
                assets = seed_report.imported,
                "validated and indexed bundled reading resources"
            );
            match db.with_conn(|conn| ielts_db::recover_interrupted_sessions(conn)) {
                Ok(n) if n > 0 => {
                    tracing::warn!(count = n, "marked interrupted evaluation sessions")
                }
                Ok(_) => {}
                Err(err) => tracing::error!(error = %err, "failed to recover evaluation sessions"),
            }
            tracing::info!(
                app_data = %paths.app_data.display(),
                db = %paths.v2_db_path().display(),
                legacy_candidates = paths.legacy_candidates.len(),
                "IELTS Practice Tauri shell ready (no Fastify localhost API)"
            );
            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running IELTS Practice Tauri application");
}
