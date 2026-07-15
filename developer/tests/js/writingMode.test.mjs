import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  requireWritingAttemptMode,
  writingTopicModeToAttemptMode
} from '../../../apps/writing-vue/src/api/writing-mode.js'

assert.equal(writingTopicModeToAttemptMode('free'), 'freeform')
assert.equal(writingTopicModeToAttemptMode('bank'), 'bank')
assert.equal(requireWritingAttemptMode('freeform'), 'freeform')
assert.equal(requireWritingAttemptMode('bank'), 'bank')
assert.throws(() => writingTopicModeToAttemptMode(''), /free or bank/)
assert.throws(() => requireWritingAttemptMode(undefined), /freeform or bank/)
assert.throws(() => requireWritingAttemptMode('single'), /freeform or bank/)

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
const clientSource = fs.readFileSync(path.join(repoRoot, 'apps/writing-vue/src/api/client.js'), 'utf8')
const composeSource = fs.readFileSync(path.join(repoRoot, 'apps/writing-vue/src/views/ComposePage.vue'), 'utf8')
const retrySource = fs.readFileSync(path.join(repoRoot, 'apps/writing-vue/src/views/EvaluatingPage.vue'), 'utf8')
assert.match(clientSource, /const mode = requireWritingAttemptMode\(payload\.mode\)/)
assert.doesNotMatch(clientSource, /mode:\s*payload\.mode\s*\|\|\s*['"]bank['"]/)
assert.match(composeSource, /mode:\s*attemptMode/)
assert.match(retrySource, /mode:\s*retryPayload\.mode/)

console.log('writing mode: ok')
