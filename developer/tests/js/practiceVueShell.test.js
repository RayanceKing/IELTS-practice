#!/usr/bin/env node
/**
 * Shipping shell contract: this test intentionally targets only the Tauri 2
 * host and the Vue frontend.  Electron/Fastify were retired and must not be
 * smuggled back into a product test as a second source of truth.
 */
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(here, '..', '..', '..')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8').replace(/\r\n/g, '\n')
}

function has(source, snippet, label) {
  assert.ok(source.includes(snippet), `${label} missing: ${snippet}`)
}

function lacks(source, snippet, label) {
  assert.ok(!source.includes(snippet), `${label} must not contain: ${snippet}`)
}

function testVueRoutesAndShell() {
  const routes = read('apps/writing-vue/src/main.js')
  const app = read('apps/writing-vue/src/App.vue')
  const nav = read('apps/writing-vue/src/components/NavBar.vue')

  for (const route of [
    "path: '/library'",
    "path: '/reading/:assetId'",
    "path: '/reading-suite/:sessionId'",
    "path: '/reading/:assetId/review/:sessionId'",
    "path: '/writing'"
  ]) has(routes, route, 'Vue practice route')

  has(app, 'showShellNav', 'shared App shell nav gate')
  has(app, 'framelessRouteNames', 'immersive-only shell escape hatch')
  lacks(app, "'PracticeLibrary'", 'Library must remain inside the shared App shell')
  has(nav, '<router-link to="/" class="brand-block">', 'product brand route')
  for (const label of ["label: '阅读'", "label: '写作'", "label: '历史'", "label: '设置'"]) {
    has(nav, label, 'global product navigation')
  }
}

function testLibraryHasOneProductShell() {
  const library = read('apps/writing-vue/src/views/PracticeLibraryPage.vue')
  const librarySettings = read('apps/writing-vue/src/modules/practice-reading/components/ReadingSettingsPanel.vue')
  const client = read('apps/writing-vue/src/api/client.js')
  const skin = read('apps/writing-vue/src/styles/opensource-skin.css')

  has(library, 'data-practice-reading-home data-library-ready', 'stable Library ready marker')
  has(library, '<h1 class="library-workspace-header__title">阅读练习</h1>', 'local workspace title')
  has(library, 'class="library-view-tabs"', 'Library secondary navigation')
  lacks(library, 'hero-brand-text', 'duplicate Library product brand')
  lacks(library, 'main-nav hero-nav', 'duplicate Library product navigation')
  lacks(library, 'practice-library-legacy', 'legacy Library shell ownership')
  lacks(library, 'practice-library-open-source', 'doubled Library skin ownership')
  assert.ok(!/^\.view\b/m.test(library), 'Library stylesheet leaks a naked .view selector')
  assert.ok(!/^\.btn\b/m.test(library), 'Library stylesheet leaks a naked .btn selector')
  has(skin, '.atlas-source-ui .practice-library', 'canonical Library visual owner')
  for (const retired of [
    'clearPracticeCache',
    'clear-practice-cache',
    'practice_reading_answers_',
    'practice_reading_submission_'
  ]) {
    lacks(`${library}\n${librarySettings}`, retired, 'retired browser-owned Reading cache')
  }
  for (const retired of [
    'notImplemented(',
    'uploadImage()',
    'deleteImage()',
    'getImagePath()',
    'requestEventStream()'
  ]) {
    lacks(client, retired, 'dead Tauri API facade')
  }
}

function testReadingActionOwnership() {
  const page = read('apps/writing-vue/src/views/PracticeReadingPage.vue')
  const nav = read('apps/writing-vue/src/modules/practice-reading/components/ReadingAnswerNav.vue')
  const interactions = read('apps/writing-vue/src/modules/practice-reading/useReadingInteractions.ts')
  const preferences = read('apps/writing-vue/src/modules/practice-reading/useReadingUiPreferences.ts')

  has(nav, 'id="exit-btn"', 'stable Reading exit identifier')
  has(nav, '@click="handleLeave"', 'guarded Reading exit')
  has(nav, "emit('leave')", 'Reading exit event')
  has(page, '@leave="handleLeave"', 'page-owned guarded leave flow')
  has(page, 'const canSnapshot = computed', 'single snapshot capability')
  has(page, 'grid-template-rows: minmax(0, 1fr);', 'constrained Reading workspace row')
  has(page, 'overscroll-behavior: contain;', 'pane-local Reading scroll')
  has(interactions, 'readOnlyModeSource', 'single read-only interaction source')
  lacks(interactions, 'reviewModeSource', 'split review-only interaction source')
  has(page, 'ref="settingsPanel"', 'Reading settings dialog element ref')
  has(page, '@keydown="handleSettingsDialogKeydown"', 'Reading settings Escape/focus handling')
  has(page, 'aria-controls="settings-panel"', 'Reading settings trigger relationship')
  has(preferences, 'function handleSettingsDialogKeydown', 'Reading settings keyboard controller')
  has(preferences, 'function focusFirstPanelControl', 'Reading settings initial focus')
  for (const copy of ["return '提交作答'", "return '清空作答'", "<h3 id=\"notes-panel-title\">阅读笔记</h3>"]) {
    has(page, copy, 'Chinese Reading product copy')
  }
}

function testTauriCommandBoundary() {
  const tauri = read('src-tauri/src/lib.rs')
  const ai = read('src-tauri/src/commands/ai.rs')
  const writing = read('src-tauri/src/commands/writing.rs')
  const settings = read('apps/writing-vue/src/api/settings-repository.js')

  for (const command of [
    'commands::ai::ai_list_configs',
    'commands::ai::ai_test_provider',
    'commands::writing::writing_start_evaluation',
    'commands::writing::writing_submit_attempt'
  ]) has(tauri, command, 'registered Tauri command')
  has(ai, 'config_id: String', 'selected AI config test command parameter')
  has(ai, 'load_runtime_for_config', 'selected AI config runtime loader')
  has(writing, 'load_provider_config(&db, &vault)', 'vault-aware writing preflight')
  has(settings, "invokeCommand('ai_test_provider', { configId })", 'Vue selected-config test command')
}

function testSettingsNativeBackupOwnership() {
  const settings = read('apps/writing-vue/src/views/SettingsPage.vue')

  for (const operation of [
    'createFullAppBackup',
    'restoreFullAppBackup',
    'showNativeBackupList',
    'createBackup(',
    'listBackups(',
    'pickBackupImportPath(',
    'importBackupPath('
  ]) has(settings, operation, 'native Settings backup operation')

  has(settings, 'Tauri 2 原生桌面客户端', 'current Tauri product identity')
  has(settings, 'Rust + SQLite', 'current Rust data identity')
  for (const retired of [
    'ielts_writing_settings_backups_v1',
    'settingsSnapshotListOpen',
    'settingsBackups',
    'exportSettingsData',
    'handleSettingsImport',
    'clear-cache-btn',
    'force-refresh-btn',
    'herbal_green_flat_logo',
    'Phase 05'
  ]) lacks(settings, retired, 'retired Settings mirror or dead action')
}

function testNoRetiredHostBoundary() {
  const shipping = [
    read('apps/writing-vue/src/main.js'),
    read('apps/writing-vue/src/App.vue'),
    read('apps/writing-vue/src/api/tauri-bridge.js'),
    read('src-tauri/src/lib.rs'),
    read('src-tauri/Cargo.toml')
  ].join('\n')
  for (const retired of ['electronAPI', 'window.electron', 'ipcRenderer']) {
    lacks(shipping, retired, 'retired host boundary')
  }
  assert.ok(!/from\s*['"]fastify['"]/.test(shipping), 'shipping code imports Fastify')
  assert.ok(!/\bfastify\s*=/.test(read('src-tauri/Cargo.toml')), 'Tauri host declares Fastify')
}

testVueRoutesAndShell()
testLibraryHasOneProductShell()
testReadingActionOwnership()
testTauriCommandBoundary()
testSettingsNativeBackupOwnership()
testNoRetiredHostBoundary()
console.log('Tauri Vue shell contract: ok')
