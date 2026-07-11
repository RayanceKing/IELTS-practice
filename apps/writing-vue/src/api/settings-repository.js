/**
 * Settings + backup + secret-ref client (Phase 4).
 * Tauri path uses Rust commands; Electron keeps existing HTTP clients.
 */

import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

export async function listSettings(namespace) {
  if (!isTauriRuntime()) return { source: 'electron', items: [] }
  const response = await invokeCommand('list_settings', { namespace: namespace || null })
  return { source: 'tauri', items: unwrapCommandResponse(response, 'list_settings') || [] }
}

export async function upsertSetting(namespace, key, value) {
  if (!isTauriRuntime()) return { source: 'electron', entry: null }
  const response = await invokeCommand('upsert_setting', {
    cmd: { namespace, key, value }
  })
  return { source: 'tauri', entry: unwrapCommandResponse(response, 'upsert_setting') }
}

export async function migrateLocalPreferences(prefs) {
  if (!isTauriRuntime()) return { source: 'electron', count: 0 }
  const response = await invokeCommand('migrate_local_preferences', { prefs })
  return {
    source: 'tauri',
    count: unwrapCommandResponse(response, 'migrate_local_preferences') || 0
  }
}

export async function setSecret(name, secret) {
  if (!isTauriRuntime()) {
    throw new Error('set_secret requires Tauri runtime; refuse localStorage key storage')
  }
  const response = await invokeCommand('set_secret', { cmd: { name, secret } })
  return unwrapCommandResponse(response, 'set_secret')
}

export async function listSecretRefs() {
  if (!isTauriRuntime()) return []
  const response = await invokeCommand('list_secret_refs')
  return unwrapCommandResponse(response, 'list_secret_refs') || []
}

export async function createBackup(appVersion) {
  if (!isTauriRuntime()) return { source: 'electron', manifest: null }
  const response = await invokeCommand('create_backup', { appVersion: appVersion || null })
  return { source: 'tauri', manifest: unwrapCommandResponse(response, 'create_backup') }
}

export async function importBackupPath(path, dryRun = true) {
  if (!isTauriRuntime()) return { source: 'electron', report: null }
  const response = await invokeCommand('import_backup_path', { path, dryRun })
  return { source: 'tauri', report: unwrapCommandResponse(response, 'import_backup_path') }
}

export const settingsRepository = {
  listSettings,
  upsertSetting,
  migrateLocalPreferences,
  setSecret,
  listSecretRefs,
  createBackup,
  importBackupPath,
  isTauriRuntime
}

export default settingsRepository
