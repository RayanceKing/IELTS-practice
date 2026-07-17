# P0 data-truth repair batch

## Goal

Make the packaged Tauri client seed the official writing catalog on a fresh database, preserve the user's selected writing source mode, and project Rust reading history DTOs into one stable camelCase frontend view model.

## Scope guard

- Do not edit `App.vue`, `PracticeLibraryPage.vue`, CSS, or Reading page components.
- Keep Rust/SQLite as the only durable truth; no frontend settings catalog copy.

## Phases

- [completed] Inspect seed, evaluation mode, and history adapter boundaries.
- [completed] Implement the three bounded fixes plus focused tests.
- [completed] Run focused Rust, Vue, and node checks; report exact evidence.
