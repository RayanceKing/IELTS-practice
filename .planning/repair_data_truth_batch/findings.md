# Findings

- Active root plan records the same P0 defects: fresh DB lacks official writing seed, `evaluate.start` coerces freeform to bank, and Rust snake_case reading history reaches Library consumers without an adapter.
- Existing worktree changes are shared and must not be overwritten or committed by this subtask.
- The catalog exists at `assets/generated/writing-topics/bc-task2-2024-12_2025-01.catalog.json`; Tauri startup currently seeds only the bundled reading pack in `src-tauri/src/lib.rs`.
- Candidate mode regressions are `apps/writing-vue/src/api/client.js` and `apps/writing-vue/src/api/writing-repository.js`, both defaulting a missing mode to `bank`; Compose already owns a `topicMode` with `bank` and `free` states.
- Library obtains data through `history-repository.js` and `historyStats.js`; the Rust history owner exports `list_history_view_models` from `crates/ielts-db/src/history/mod.rs`.
- Tauri package resources currently include only `../assets/resource-pack/reading`; the writing catalog is absent from `src-tauri/tauri.conf.json`. Startup seeds only `reading`.
- `useDraft` already maps Compose UI mode `free` to Rust `freeform`; the broken final submission path is `evaluate.start` / `writing-repository.saveDraft` defaulting an absent mode to `bank`, while `ComposePage.vue` does not pass `mode`.
- Rust `HistoryListItemVm` serializes camelCase. The current adapter reintroduces snake_case aliases and does not project `accuracy` or `submittedAt`, so `historyStats.js` reads zero/empty fields even for valid reading attempts.
- Reading history components consume `submittedAt`, `duration` (seconds), and `accuracy` (0..1), while the adapter currently emits `submitted_at`, `duration_ms`, and `score_value`. A single camelCase VM can satisfy this without touching Library markup.
- `useDraft` proves the canonical durable writing modes are `freeform` and `bank`; the Compose UI-only `free` token must be converted at the submission boundary.
- `seed_builtin_reading_pack` is the appropriate model: validate a packaged manifest/file, use one transaction, then upsert only the canonical index. The writing seed can use a catalog file with stable `source_id` and must explicitly skip a non-official collision rather than overwrite it.
- All existing Phase 5 writing fixture drafts use `AttemptMode::Bank`; adding a Rust guard for writing modes requires new focused freeform/bank tests but should not break existing tests.
- The Tauri source already re-exports `writing::topics::*`, so the seed function is available to startup without a separate export path. The first fixture seed test now exercises create → unchanged and a user-ID collision.
- `HistoryListItemVm` is camelCase on the wire despite Rust field spelling due `serde(rename_all)`. The frontend should treat that wire DTO as input only and expose a separate camelCase view model, never snake aliases.
- Retry is a second `evaluate.start` caller in `EvaluatingPage.vue`; it also omits the source mode. The mode must be recovered from the durable attempt, not guessed from `topicId` (freeform and bank prompts can both have text).
- The remaining snake_case scan result is the writing-history adapter output consumed by `HistoryPage`, not the Reading VM. It is intentionally outside this Reading-only compatibility change.
- Retry now gets `mode` from the durable `WritingDraft`. A malformed legacy orphan keeps its content and serializes `mode: null`; it is intentionally non-retryable rather than being silently mislabeled.
- Fresh catalog seed validates the actual shipped catalog (232 current rows in focused test), creates only absent official rows, skips user-owned collisions, and avoids churn by fingerprinting existing official rows.
