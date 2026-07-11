# Release notes — Tauri cutover

## Highlights

- Shipping runtime is Tauri 2 / Rust / Vue only.
- SQLite v2 is the primary store for history, settings, writing eval, reading attempts, modes, annotations, vocab, and coach.
- Electron main/preload and Fastify local API removed from the repository product surface.

## Upgrade

1. Backup existing data.
2. Install Tauri build.
3. Import legacy DB/export if needed (see phase10-cutover.md).

## Known issues

- Code signing / updater endpoints require secrets not shipped in-repo.
- Legacy Electron packages are no longer built from this tree.
