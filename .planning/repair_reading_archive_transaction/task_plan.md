# Reading archive transaction repair

## Goal

Make Reading archive export/import a Rust-owned, atomic, reportable operation while preserving current UI and file contracts.

## Scope

- Rust reading archive/export/import, Tauri command/API, Library archive handlers, focused tests.
- Exclude Settings, History, CSS, and retention.

## Phases

- [completed] Inspect current archive data flow and public contracts.
- [completed] Make export canonical and import validate-then-transactional with a structured report.
- [completed] Adapt Library UI to truthfully consume the result.
- [completed] Add regression tests and run focused Rust/Vue checks.
