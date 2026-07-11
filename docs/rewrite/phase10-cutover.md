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
- Some Vue pages still contain Electron fallback branches for local non-Tauri Vite dev; production shell is Tauri-only (`isTauriRuntime()` true).
- Visual regression screenshots are manual for this cutover; CI covers Rust tests + multi-platform Tauri build.
- Resource release scripts that previously assumed Electron packaging may need path updates for Tauri bundle outputs.

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
