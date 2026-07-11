/**
 * Writing evaluation client (Phase 5).
 * Prefers Tauri persisted state machine; Electron falls back to existing HTTP/SSE.
 */

import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

function newIdempotencyKey(prefix = 'w') {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export async function saveDraft(payload) {
  if (!isTauriRuntime()) return { source: 'electron', draft: null }
  const cmd = {
    attemptId: payload.attemptId,
    activity: 'writing',
    mode: payload.mode || 'bank',
    assetId: payload.assetId || null,
    contentText: payload.contentText || '',
    promptSnapshot: payload.promptSnapshot || null,
    idempotencyKey: payload.idempotencyKey || newIdempotencyKey('draft')
  }
  const response = await invokeCommand('writing_save_draft', { cmd })
  return { source: 'tauri', draft: unwrapCommandResponse(response, 'writing_save_draft') }
}

export async function getDraft(attemptId) {
  if (!isTauriRuntime()) return { source: 'electron', draft: null }
  const response = await invokeCommand('writing_get_draft', { attemptId })
  return { source: 'tauri', draft: unwrapCommandResponse(response, 'writing_get_draft') }
}

export async function submitAttempt(attemptId, idempotencyKey) {
  if (!isTauriRuntime()) return { source: 'electron', attempt: null }
  const response = await invokeCommand('writing_submit_attempt', {
    cmd: {
      attemptId,
      idempotencyKey: idempotencyKey || newIdempotencyKey('submit')
    }
  })
  return { source: 'tauri', attempt: unwrapCommandResponse(response, 'writing_submit_attempt') }
}

/**
 * Start evaluation. Returns full run result with ordered events (Channel-equivalent batch).
 * UI can also poll writing_list_evaluation_events for live updates in later async provider mode.
 */
export async function startEvaluation(payload) {
  if (!isTauriRuntime()) return { source: 'electron', result: null }
  const response = await invokeCommand('writing_start_evaluation', {
    cmd: {
      attemptId: payload.attemptId,
      idempotencyKey: payload.idempotencyKey || newIdempotencyKey('eval'),
      taskType: payload.taskType || null,
      retryOf: payload.retryOf || null
    }
  })
  return {
    source: 'tauri',
    result: unwrapCommandResponse(response, 'writing_start_evaluation')
  }
}

export async function listEvaluationEvents(evaluationId, afterSequence = 0) {
  if (!isTauriRuntime()) return []
  const response = await invokeCommand('writing_list_evaluation_events', {
    evaluationId,
    afterSequence
  })
  return unwrapCommandResponse(response, 'writing_list_evaluation_events') || []
}

export async function cancelEvaluation(evaluationId) {
  if (!isTauriRuntime()) return false
  const response = await invokeCommand('writing_cancel_evaluation', { evaluationId })
  return !!unwrapCommandResponse(response, 'writing_cancel_evaluation')
}

/** Result page must load from DB, not sessionStorage as source of truth. */
export async function getEvaluationForAttempt(attemptId) {
  if (!isTauriRuntime()) return { source: 'electron', evaluation: null }
  const response = await invokeCommand('writing_get_evaluation', { attemptId })
  return {
    source: 'tauri',
    evaluation: unwrapCommandResponse(response, 'writing_get_evaluation')
  }
}

export const writingRepository = {
  saveDraft,
  getDraft,
  submitAttempt,
  startEvaluation,
  listEvaluationEvents,
  cancelEvaluation,
  getEvaluationForAttempt,
  newIdempotencyKey,
  isTauriRuntime
}

export default writingRepository
