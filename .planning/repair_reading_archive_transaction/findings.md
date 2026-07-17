# Findings

- Scoped work started 2026-07-15. Current worktree is shared; do not overwrite unrelated changes or commit planning files.
- Archive path currently crosses `practice-client.js` → Tauri `history.rs` → `ielts-db/import/reading_archive.rs`; export still lives in JavaScript and explicitly catches per-item failures, which is the exact false-success bug.
- Library owns download/file-reading UX and must keep its existing DOM events; only its success/error decision should move to the structured Rust result.
- Current importer invokes `upsert_attempt` record-by-record with no transaction and records errors as a successful `ImportReport`; a bad row can therefore leave earlier writes durable.
- Existing archive input accepts both product `submissions[]` and legacy `records[]`; compatibility should stay at the validator/converter boundary, while new export must emit one canonical v2 document.
- `AttemptRecord` already gives a complete camelCase serializable SQLite snapshot (attempt, answers, annotations). The history loader already hydrates those child records, so Rust can produce a direct canonical archive without reconstructing old browser submissions.
- `upsert_attempt` enforces immutable identity and writes child rows, making it suitable inside one transaction once validation has completed; current importer must not keep calling it on the bare connection.
- The command envelope only carries data on `ok: true`; the new import data must therefore include `committed`, `imported`, `failed`, and `report`, and the JS adapter must convert a non-committed data result into an exception before Library success messaging.
- Existing command registration has a reading command group, so native archive commands can live there while retaining the old history import command as a backward-compatible adapter.
- The legacy Fastify-only facade test still asserts v1 inflated submissions; it is not the Tauri product contract. New v2 keeps top-level `activity/schemaVersion/exportedAt/count/submissions` so Library backups and file-selection DOM remain compatible while the record payload becomes the canonical `AttemptRecord` snapshot.
- `attempts.asset_id` has a foreign key to `practice_assets`; archive tests must seed/import an asset stub before calling `upsert_attempt`, matching the product importer.
- Kept the old `import_reading_archive_value` Tauri command name only as a compatibility alias, but removed its divergent `importedCount/errors` wrapper. Both command names now expose the same canonical `imported/failed/report/committed` result.
- `upsert_attempt` intentionally leaves annotations intact for live practice writes. Archive import must delete that attempt's annotations inside its transaction before upsert, otherwise a restore is not a true snapshot and stale annotations survive.
- Legacy fixture declares numeric `schemaVersion: 1`; unknown string archive schemas must be rejected rather than silently running the legacy converter over a v2-shaped direct attempt and losing answers/title.
