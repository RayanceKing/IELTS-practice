# Phase 10 cutover — migration, backup, known limits

> Date: 2026-07-12  
> Runtime: **Tauri 2 + Rust only**. Electron + Fastify removed from the product tree.

## What changed

| Removed from product surface | Kept |
|---|---|
| `electron/` main/preload/local-api-server | `crates/ielts-db` legacy importers |
| `server/` Fastify/SSE/business HTTP | SQLite v2 migrations + domain crates |
| `package.json` electron-builder main path | `src-tauri` bundle + Vue `apps/writing-vue` |
| Shadow-read dual-write production path | One-shot legacy import tools in DB crate |

## Data migration

1. **Backup first** (Tauri settings → backup, or copy app data dir).
2. On first Tauri launch, SQLite v2 is created/migrated under the app data path.
3. Legacy Electron SQLite / browser export JSON can be imported via Phase 3/4 importers:
   - `migrate_legacy_sqlite_to_v2`
   - reading archive import
   - backup package import (`create_backup` / `import_backup_path`)
4. Secrets never travel in ordinary backups; vault refs only.

## Known limits

- Updater pubkey/endpoints inactive until release signing secrets are configured.
- Real AI provider, packaged reading-resource parity and complete legacy asset removal are not yet accepted.
- Visual regression, keyboard coverage and device P95 measurements remain release blockers.
- CI now puts the static suite and packaged Windows Tauri WebView flow in the first shipping gate. A workflow definition is not proof of success; retain the uploaded reports from a passing run.
- Release builds generate a SHA-256 bundle manifest and fail when the bundle is empty.
- Windows/macOS code-signing, updater signing and rollback drills require external secrets and real devices. They remain incomplete until those runs pass.

## Developer commands

```bash
npm run prepare:writing
npm run build:writing
cargo test -p ielts-db
cargo tauri dev
cargo tauri build
```

## Rollback

Restore a pre-cutover tag/commit that still contains `electron/` + `server/` if emergency dual-runtime support is required. Prefer restoring user data from backup rather than re-enabling dual-write.

## Required release evidence

- `static-ci-report.json` with status `pass`.
- `suite-practice-flow-report.json` with target `packaged-tauri-2` and status `passed`.
- One non-empty `tauri-bundle-<platform>.json` manifest per release platform.
- Platform signing/notarization logs and an updater install/rollback record. These cannot be produced without release secrets and devices.
