use ielts_db::{
    create_backup_package, import_backup, list_ai_configs,
    list_ai_configs_with_secret_availability, migrate, open_connection, put_secret_ref,
    reconcile_default_ai_config_with_secret_availability, set_default_ai_config, upsert_ai_config,
    upsert_setting, validate_backup, DbOpenOptions,
};
use ielts_domain::dto::{AiConfigDto, BackupPackage, BackupSqlValue};
use rusqlite::Connection;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn open_v2(path: impl Into<std::path::PathBuf>) -> Connection {
    let mut conn = open_connection(&DbOpenOptions::create(path.into())).unwrap();
    migrate(&mut conn).unwrap();
    conn
}

fn seed_complete_user_state(conn: &Connection) {
    conn.execute_batch(
        r#"
        INSERT INTO practice_assets (
          id, activity, source_kind, source_key, title, category, difficulty, frequency,
          content_ref, schema_version, fingerprint, pdf_only, metadata_json, created_at, updated_at
        ) VALUES
          ('asset-reading', 'reading', 'imported', 'reading:1', 'Reading One', 'P1', 'medium', 'high',
           'C:/fixtures/reading.json', 2, 'fp-reading', 0, '{"revision":2}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
          ('asset-writing', 'writing', 'freeform', 'writing:1', 'Writing One', NULL, NULL, NULL,
           NULL, 2, 'fp-writing', 0, '{"taskType":"task2"}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');

        INSERT INTO writing_topics (
          asset_id, task_type, title_json, image_path, is_official, created_at, updated_at
        ) VALUES (
          'asset-writing', 'task2', '{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Writing One"}]}]}',
          NULL, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
        );

        INSERT INTO reading_suites (
          id, mode, flow_mode, status, current_index, timer_policy_json, created_at, updated_at,
          frequency_scope, seed, aggregate_json, completed_at, timer_state_json
        ) VALUES (
          'suite-1', 'suite', 'linear', 'active', 0, '{"limitMs":3600000}',
          '2026-01-01T00:00:00Z', '2026-01-01T00:00:01Z', 'all', 'seed-1',
          '{"correct":1}', NULL, '{"elapsedMs":1234,"running":true}'
        );

        INSERT INTO attempts (
          id, activity, asset_id, mode, suite_id, status, started_at, submitted_at, completed_at,
          duration_ms, score_value, score_scale, correct_count, question_count, title_snapshot,
          prompt_snapshot, content_text, schema_version, created_at, updated_at
        ) VALUES
          ('r1', 'reading', 'asset-reading', 'suite', 'suite-1', 'completed',
           '2026-01-01T00:00:00Z', '2026-01-01T00:10:00Z', '2026-01-01T00:10:00Z',
           600000, 1.0, 'ratio', 1.0, 1, 'Reading One', NULL, NULL, 2,
           '2026-01-01T00:00:00Z', '2026-01-01T00:10:00Z'),
          ('w1', 'writing', 'asset-writing', 'bank', NULL, 'completed',
           '2026-01-02T00:00:00Z', '2026-01-02T00:40:00Z', '2026-01-02T00:41:00Z',
           2400000, 7.0, 'band9', NULL, NULL, 'Writing One', 'Discuss both views', 'Essay body', 2,
           '2026-01-02T00:00:00Z', '2026-01-02T00:41:00Z'),
          ('m1', 'reading', 'asset-reading', 'memorize', NULL, 'active',
           '2026-01-03T00:00:00Z', NULL, NULL, 1200, NULL, NULL, NULL, 1, 'Memorize One', NULL, NULL, 2,
           '2026-01-03T00:00:00Z', '2026-01-03T00:00:01Z');

        INSERT INTO attempt_answers (
          attempt_id, question_id, answer_json, is_correct, weight, question_kind,
          change_count, visit_count, elapsed_ms, marked, answered_at
        ) VALUES ('r1', 'q1', '"A"', 1, 1.0, 'choice', 2, 3, 4567, 1, '2026-01-01T00:09:00Z');

        INSERT INTO attempt_annotations (
          id, attempt_id, asset_id, scope, question_id, kind, anchor_json, note_text, created_at, updated_at
        ) VALUES (
          'ann-1', 'r1', 'asset-reading', 'question', 'q1', 'note', '{"start":2,"end":4}',
          'why A?', '2026-01-01T00:05:00Z', '2026-01-01T00:06:00Z'
        );

        INSERT INTO writing_evaluations (
          id, attempt_id, status, stage, provider_id, model, rubric_version, prompt_version,
          result_json, degradation_json, error_json, started_at, completed_at, updated_at
        ) VALUES (
          'eval-1', 'w1', 'completed', 'done', 'openai', 'gpt-test', 'rubric-4', 'prompt-2',
          '{"overallBand":7.0,"feedback":"clear"}', '[]', NULL,
          '2026-01-02T00:40:00Z', '2026-01-02T00:41:00Z', '2026-01-02T00:41:00Z'
        );

        INSERT INTO writing_drafts (
          attempt_id, content_text, prompt_snapshot, task_type, word_count, idempotency_key, updated_at
        ) VALUES ('w1', 'Essay body', 'Discuss both views', 'task2', 2, 'draft-key-1', '2026-01-02T00:39:00Z');

        INSERT INTO attempt_idempotency (
          scope, idempotency_key, attempt_id, evaluation_id, response_json, created_at
        ) VALUES (
          'writing.submit', 'submit-key-1', 'w1', 'eval-1', '{"evaluationId":"eval-1"}', '2026-01-02T00:40:00Z'
        );

        INSERT INTO evaluation_sessions (
          id, attempt_id, evaluation_id, status, stage, revision, sequence, retry_of,
          cancel_requested, provider_id, model, started_at, updated_at, completed_at
        ) VALUES (
          'session-1', 'w1', 'eval-1', 'completed', 'done', 1, 2, NULL, 0,
          'openai', 'gpt-test', '2026-01-02T00:40:00Z', '2026-01-02T00:41:00Z', '2026-01-02T00:41:00Z'
        );

        INSERT INTO evaluation_checkpoints (evaluation_id, stage, revision, payload_json, created_at)
        VALUES ('eval-1', 'scored', 1, '{"band":7}', '2026-01-02T00:40:30Z');

        INSERT INTO evaluation_events (
          evaluation_id, sequence, revision, event_type, stage, payload_json, created_at
        ) VALUES ('eval-1', 1, 1, 'progress', 'scored', '{"percent":80}', '2026-01-02T00:40:30Z');

        INSERT INTO evaluation_lineage (
          evaluation_id, attempt_id, retry_of, root_evaluation_id, created_at
        ) VALUES ('eval-1', 'w1', NULL, 'eval-1', '2026-01-02T00:40:00Z');

        INSERT INTO reading_suite_items (
          suite_id, item_index, asset_id, attempt_id, status, title, category, submitted_at, score_json
        ) VALUES (
          'suite-1', 0, 'asset-reading', 'r1', 'completed', 'Reading One', 'P1',
          '2026-01-01T00:10:00Z', '{"correct":1,"total":1}'
        );

        INSERT INTO endless_sessions (
          id, status, pool_policy_json, pool_json, current_asset_id, current_attempt_id,
          completed_asset_ids_json, created_at, updated_at
        ) VALUES (
          'endless-1', 'active', '{"frequency":"all"}', '["asset-reading"]',
          'asset-reading', 'r1', '["asset-reading"]', '2026-01-04T00:00:00Z', '2026-01-04T00:01:00Z'
        );

        INSERT INTO mode_idempotency (scope, idempotency_key, entity_id, response_json, created_at)
        VALUES
          ('memorize_submit', 'memorize-key-1', 'm1', '{"revealed":true}', '2026-01-03T00:01:00Z'),
          ('timer_pause', 'timer-key-1', 'suite-1', '{"elapsedMs":1234}', '2026-01-01T00:01:00Z');

        INSERT INTO coach_threads (
          id, attempt_id, asset_id, status, created_at, updated_at, kind, last_error_json
        ) VALUES (
          'thread-1', 'r1', 'asset-reading', 'active', '2026-01-05T00:00:00Z',
          '2026-01-05T00:01:00Z', 'chat', NULL
        );

        INSERT INTO coach_messages (
          id, thread_id, role, content, structured_payload, status, created_at, sequence
        ) VALUES
          ('message-1', 'thread-1', 'user', 'Why A?', '{"questionId":"q1"}', 'completed', '2026-01-05T00:00:10Z', 1),
          ('message-2', 'thread-1', 'assistant', 'Because the passage says so.', NULL, 'completed', '2026-01-05T00:00:20Z', 2);

        INSERT INTO agent_runs (
          id, provider_id, model, status, rounds, tool_call_count, result_json, error_json,
          created_at, updated_at, completed_at
        ) VALUES (
          'agent-run-1', 'openai-compatible', 'gpt-test', 'completed', 2, 1,
          '{"model":"gpt-test","hasContent":true}', NULL,
          '2026-01-05T01:00:00Z', '2026-01-05T01:00:02Z', '2026-01-05T01:00:02Z'
        );

        INSERT INTO agent_tool_calls (
          run_id, call_id, sequence, round_index, tool_name, status, arguments_json,
          result_json, error_json, started_at, completed_at
        ) VALUES (
          'agent-run-1', 'tool-call-1', 1, 1, 'read_file', 'succeeded',
          '{"path":"notes.txt"}', '{"path":"notes.txt","bytes":5,"sha256":"abc"}', NULL,
          '2026-01-05T01:00:01Z', '2026-01-05T01:00:01Z'
        );

        INSERT INTO vocabulary_items (
          id, term, normalized_term, definition, phonetic, part_of_speech, example,
          source_asset_id, source_attempt_id, tags_json, created_at, updated_at
        ) VALUES (
          'vocab-1', 'Atlas', 'atlas', 'a book of maps', '/atlas/', 'noun', 'Open the atlas.',
          'asset-reading', 'r1', '["reading"]', '2026-01-06T00:00:00Z', '2026-01-06T00:00:00Z'
        );

        INSERT INTO vocabulary_review_state (
          item_id, ease, interval_days, repetitions, due_at, last_reviewed_at, lapses
        ) VALUES ('vocab-1', 2.6, 3, 2, '2026-01-09T00:00:00Z', '2026-01-06T00:00:00Z', 1);

        INSERT INTO dictionary_entries (
          term, normalized_term, definition, phonetic, part_of_speech, example, source_label, license, payload_json
        ) VALUES ('atlas', 'atlas', 'a book of maps', '/atlas/', 'noun', 'Open the atlas.', 'fixture', 'CC0', '{"rank":1}');

        INSERT INTO settings (namespace, key, value_json, updated_at)
        VALUES
          ('ui', 'theme', '"dark"', '2026-01-07T00:00:00Z'),
          ('ai', 'secretName', '"ai.config.primary"', '2026-01-07T00:00:01Z'),
          ('ai', 'config:primary', '{"id":"primary","hasSecret":false}', '2026-01-07T00:00:02Z');

        INSERT INTO migration_meta (key, value) VALUES ('legacy_import_complete', 'true');
        "#,
    )
    .unwrap();
    put_secret_ref(conn, "ai.config.primary", "kv:fixture:primary").unwrap();
}

fn rechecksum(package: &mut BackupPackage) {
    package.manifest.checksum_sha256.clear();
    let bytes = serde_json::to_vec(package).unwrap();
    package.manifest.checksum_sha256 = hex::encode(Sha256::digest(bytes));
}

#[test]
fn full_backup_roundtrip_preserves_every_user_truth_table() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("source.db"));
    seed_complete_user_state(&source);
    source
        .execute(
            "INSERT INTO reading_timer_states(scope, owner_id, state_json, updated_at)
             VALUES ('attempt', 'r1', ?1, '2026-07-17T00:00:00Z')",
            [json!({
                "source": "single",
                "anchorMs": 1_000,
                "effectiveStartTimeMs": 1_000,
                "mode": "elapsed",
                "pausedOffsetMs": 0,
                "pausedAtMs": 6_000,
                "running": false
            })
            .to_string()],
        )
        .unwrap();

    let package = create_backup_package(&source, "roundtrip-test").unwrap();
    assert_eq!(package.manifest.schema_version, 7);
    assert_eq!(
        package.manifest.table_count as usize,
        package.database.len()
    );
    assert_eq!(package.manifest.secret_ref_count, 1);
    assert!(package
        .attempts
        .iter()
        .find(|attempt| attempt.id == "r1")
        .is_some_and(|attempt| attempt.answers.len() == 1 && attempt.annotations.len() == 1));
    let serialized = serde_json::to_string(&package).unwrap();
    assert!(!serialized.contains("sk-plaintext-must-never-appear"));

    let target = open_v2(dir.path().join("target.db"));
    target
        .execute_batch(
            "INSERT INTO settings(namespace,key,value_json,updated_at)
             VALUES ('ui','sentinel','true','2026-01-01T00:00:00Z')",
        )
        .unwrap();
    let before_dry_run = create_backup_package(&target, "before-dry-run")
        .unwrap()
        .database;
    let dry_run = import_backup(&target, &package, true).unwrap();
    assert!(dry_run.ok, "{:?}", dry_run.errors);
    assert_eq!(dry_run.rows_imported, package.manifest.row_count);
    assert_eq!(
        create_backup_package(&target, "after-dry-run")
            .unwrap()
            .database,
        before_dry_run,
        "dry-run must leave the target byte-for-byte equivalent at the logical row level"
    );

    let restored = import_backup(&target, &package, false).unwrap();
    assert!(restored.ok, "{:?}", restored.errors);
    assert_eq!(restored.tables_imported, package.manifest.table_count);
    assert_eq!(restored.rows_imported, package.manifest.row_count);
    assert_eq!(
        target
            .query_row(
                "SELECT COUNT(*) FROM reading_timer_states WHERE scope='attempt' AND owner_id='r1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );

    let target_package = create_backup_package(&target, "after-restore").unwrap();
    assert_eq!(target_package.database, package.database);
    assert_eq!(target_package.attempts, package.attempts);
    assert_eq!(target_package.settings, package.settings);
    assert_eq!(target_package.secret_refs, package.secret_refs);

    assert_eq!(
        target
            .query_row(
                "SELECT answer_json FROM attempt_answers WHERE attempt_id='r1' AND question_id='q1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "\"A\""
    );
    assert_eq!(
        target
            .query_row(
                "SELECT note_text FROM attempt_annotations WHERE id='ann-1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "why A?"
    );
    let restored_evaluation: serde_json::Value = serde_json::from_str(
        &target
            .query_row(
                "SELECT result_json FROM writing_evaluations WHERE id='eval-1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        restored_evaluation,
        json!({"overallBand": 7.0, "feedback": "clear"})
    );
    assert_eq!(
        target
            .query_row(
                "SELECT COUNT(*) FROM mode_idempotency WHERE scope IN ('memorize_submit','timer_pause')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        2
    );
    assert_eq!(
        target
            .query_row("SELECT COUNT(*) FROM coach_messages", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        2
    );
}

#[test]
fn failed_full_restore_rolls_back_without_polluting_target() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("source.db"));
    seed_complete_user_state(&source);
    let mut package = create_backup_package(&source, "rollback-test").unwrap();

    let attempts = package
        .database
        .iter_mut()
        .find(|table| table.name == "attempts")
        .unwrap();
    let status_index = attempts
        .columns
        .iter()
        .position(|column| column == "status")
        .unwrap();
    attempts.rows[0][status_index] = BackupSqlValue::Null;
    rechecksum(&mut package);

    let target = open_v2(dir.path().join("target.db"));
    target
        .execute_batch(
            "INSERT INTO settings(namespace,key,value_json,updated_at)
             VALUES ('ui','sentinel','\"keep\"','2026-01-01T00:00:00Z')",
        )
        .unwrap();
    let before = create_backup_package(&target, "before").unwrap().database;

    let report = import_backup(&target, &package, false).unwrap();
    assert!(!report.ok);
    assert!(!report.errors.is_empty());
    assert_eq!(
        create_backup_package(&target, "after").unwrap().database,
        before
    );
}

#[test]
fn legacy_v1_package_is_read_explicitly_as_partial_compatibility_import() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("source.db"));
    seed_complete_user_state(&source);
    let current = create_backup_package(&source, "legacy-source").unwrap();
    let mut legacy_attempt = current
        .attempts
        .iter()
        .find(|attempt| attempt.id == "w1")
        .unwrap()
        .clone();
    legacy_attempt.asset_id = None;
    legacy_attempt.annotations.clear();

    let legacy_json = json!({
        "manifest": {
            "schemaVersion": 1,
            "createdAt": "2025-01-01T00:00:00Z",
            "appVersion": "legacy",
            "includesSecrets": false,
            "attemptCount": 1,
            "settingsCount": 1,
            "secretRefCount": 0,
            "checksumSha256": ""
        },
        "attempts": [legacy_attempt],
        "settings": [current.settings[0].clone()],
        "secretRefs": []
    });
    let legacy: BackupPackage = serde_json::from_value(legacy_json).unwrap();
    assert!(legacy.database.is_empty());

    let target = open_v2(dir.path().join("target.db"));
    let report = import_backup(&target, &legacy, false).unwrap();
    assert!(report.ok, "{:?}", report.errors);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("incomplete")));
    assert_eq!(
        target
            .query_row("SELECT COUNT(*) FROM attempts WHERE id='w1'", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        1
    );
}

#[test]
fn legacy_v2_snapshot_without_writing_topics_remains_restorable() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("source.db"));
    seed_complete_user_state(&source);
    let mut legacy = create_backup_package(&source, "v2-source").unwrap();

    legacy.manifest.schema_version = 2;
    legacy.manifest.database_schema_version = 5;
    legacy.database.retain(|table| {
        table.name != "writing_topics"
            && table.name != "writing_prompts"
            && table.name != "history_retention_policy"
            && table.name != "reading_timer_states"
            && table.name != "agent_runs"
            && table.name != "agent_tool_calls"
    });
    legacy.manifest.table_count = legacy.database.len() as u32;
    legacy.manifest.row_count = legacy
        .database
        .iter()
        .map(|table| table.rows.len() as u64)
        .sum::<u64>()
        + legacy.secret_refs.len() as u64;
    rechecksum(&mut legacy);

    let target = open_v2(dir.path().join("target.db"));
    seed_complete_user_state(&target);
    let report = import_backup(&target, &legacy, false).unwrap();
    assert!(report.ok, "{:?}", report.errors);
    assert_eq!(report.tables_imported, legacy.manifest.table_count);
    assert_eq!(
        target
            .query_row("SELECT COUNT(*) FROM writing_topics", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0,
        "a v2 backup has no topic projection and must not retain target-only rows"
    );
    assert_eq!(
        target
            .query_row(
                "SELECT COUNT(*) FROM practice_assets WHERE id = 'asset-writing'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
}

#[test]
fn legacy_v4_snapshot_projects_prompt_settings_inside_restore_transaction() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("prompt-v4-source.db"));
    upsert_setting(
        &source,
        "prompts",
        "legacy-task2",
        &json!({
            "id": "legacy-task2",
            "taskType": "task2",
            "version": "legacy-v4",
            "body": "RESTORED PROMPT POLICY",
            "isActive": true,
        }),
    )
    .unwrap();
    let mut legacy = create_backup_package(&source, "v4-prompt-source").unwrap();
    legacy.manifest.schema_version = 4;
    legacy.database.retain(|table| {
        table.name != "writing_prompts"
            && table.name != "reading_timer_states"
            && table.name != "agent_runs"
            && table.name != "agent_tool_calls"
    });
    legacy.manifest.table_count = legacy.database.len() as u32;
    legacy.manifest.row_count = legacy
        .database
        .iter()
        .map(|table| table.rows.len() as u64)
        .sum::<u64>()
        + legacy.secret_refs.len() as u64;
    rechecksum(&mut legacy);

    let target = open_v2(dir.path().join("prompt-v4-target.db"));
    let report = import_backup(&target, &legacy, false).unwrap();
    assert!(report.ok, "{:?}", report.errors);
    assert_eq!(
        target
            .query_row(
                "SELECT body FROM writing_prompts WHERE id = 'legacy-task2' AND is_active = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "RESTORED PROMPT POLICY"
    );
}

#[test]
fn legacy_v5_snapshot_without_reading_timers_remains_restorable() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("timer-v5-source.db"));
    seed_complete_user_state(&source);
    let mut legacy = create_backup_package(&source, "timer-v5-source").unwrap();
    legacy.manifest.schema_version = 5;
    legacy.manifest.database_schema_version = 9;
    legacy.database.retain(|table| {
        table.name != "reading_timer_states"
            && table.name != "agent_runs"
            && table.name != "agent_tool_calls"
    });
    legacy.manifest.table_count = legacy.database.len() as u32;
    legacy.manifest.row_count = legacy
        .database
        .iter()
        .map(|table| table.rows.len() as u64)
        .sum::<u64>()
        + legacy.secret_refs.len() as u64;
    rechecksum(&mut legacy);

    let target = open_v2(dir.path().join("timer-v5-target.db"));
    let report = import_backup(&target, &legacy, false).unwrap();
    assert!(report.ok, "{:?}", report.errors);
    assert_eq!(
        target
            .query_row("SELECT COUNT(*) FROM reading_timer_states", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
}

#[test]
fn legacy_v6_snapshot_without_agent_tables_remains_restorable() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("agent-v6-source.db"));
    seed_complete_user_state(&source);
    let mut legacy = create_backup_package(&source, "agent-v6-source").unwrap();
    legacy.manifest.schema_version = 6;
    legacy.manifest.database_schema_version = 10;
    legacy
        .database
        .retain(|table| table.name != "agent_runs" && table.name != "agent_tool_calls");
    legacy.manifest.table_count = legacy.database.len() as u32;
    legacy.manifest.row_count = legacy
        .database
        .iter()
        .map(|table| table.rows.len() as u64)
        .sum::<u64>()
        + legacy.secret_refs.len() as u64;
    rechecksum(&mut legacy);

    let target = open_v2(dir.path().join("agent-v6-target.db"));
    seed_complete_user_state(&target);
    let report = import_backup(&target, &legacy, false).unwrap();
    assert!(report.ok, "{:?}", report.errors);
    assert_eq!(
        target
            .query_row("SELECT COUNT(*) FROM agent_runs", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0,
        "a v6 backup has no Agent audit rows and must not retain target-only rows"
    );
}

#[test]
fn backup_rejects_dangling_agent_tool_call_reference() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("agent-reference-source.db"));
    seed_complete_user_state(&source);
    let mut package = create_backup_package(&source, "agent-reference-source").unwrap();
    let calls = package
        .database
        .iter_mut()
        .find(|table| table.name == "agent_tool_calls")
        .unwrap();
    let run_id = calls
        .columns
        .iter()
        .position(|column| column == "run_id")
        .unwrap();
    calls.rows[0][run_id] = BackupSqlValue::Text("missing-run".into());
    rechecksum(&mut package);

    let error = validate_backup(&package).unwrap_err();
    assert!(error
        .to_string()
        .contains("dangling reference agent_tool_calls.run_id=missing-run"));
}

#[test]
fn backup_creation_refuses_plaintext_secret_even_if_sql_bypassed_settings_api() {
    let dir = tempdir().unwrap();
    let conn = open_v2(dir.path().join("secret-leak.db"));
    conn.execute(
        "INSERT INTO settings(namespace,key,value_json,updated_at) VALUES ('ai','api_key',?1,?2)",
        rusqlite::params![
            serde_json::to_string("sk-plaintext-must-never-appear").unwrap(),
            "2026-01-01T00:00:00Z"
        ],
    )
    .unwrap();

    let error = create_backup_package(&conn, "secret-policy-test").unwrap_err();
    assert!(error.to_string().contains("secret material"));
}

#[test]
fn cross_device_restore_keeps_ai_reference_unavailable_until_key_is_reentered() {
    let dir = tempdir().unwrap();
    let source = open_v2(dir.path().join("source.db"));
    let config = AiConfigDto {
        id: "portable-openai".into(),
        config_name: "Portable OpenAI".into(),
        provider: "openai".into(),
        base_url: "https://api.openai.com/v1".into(),
        default_model: "gpt-4o-mini".into(),
        is_default: false,
        is_enabled: true,
        has_secret: false,
    };
    upsert_ai_config(&source, &config).unwrap();
    put_secret_ref(
        &source,
        "ai.config.portable-openai.api_key",
        "keyring:source-device-only",
    )
    .unwrap();
    let configured = list_ai_configs(&source).unwrap().pop().unwrap();
    set_default_ai_config(&source, Some(&configured)).unwrap();
    let package = create_backup_package(&source, "cross-device").unwrap();

    let serialized = serde_json::to_string(&package).unwrap();
    assert!(
        !serialized.contains("sk-"),
        "backups never carry API key bytes"
    );
    assert!(serialized.contains("keyring:source-device-only"));

    let target = open_v2(dir.path().join("target.db"));
    let report = import_backup(&target, &package, false).unwrap();
    assert!(report.ok, "{:?}", report.errors);

    // A different device has no matching OS-vault entry. The copied reference
    // remains metadata for same-device recovery, but cannot grant runtime use.
    assert!(
        reconcile_default_ai_config_with_secret_availability(&target, |_| false)
            .unwrap()
            .is_none()
    );
    let configs = list_ai_configs_with_secret_availability(&target, |_| false).unwrap();
    assert_eq!(configs.len(), 1);
    assert!(!configs[0].has_secret);
    assert!(!configs[0].is_default);
}
