/**
 * Reading attempt repository — Tauri only.
 */

import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

function newKey(prefix = 'r') {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export async function listReadingAssets() {
  const response = await invokeCommand('reading_list_assets')
  return { source: 'tauri', items: unwrapCommandResponse(response, 'reading_list_assets') || [] }
}

export async function saveReadingDraft(payload) {
  const response = await invokeCommand('reading_save_draft', {
    cmd: {
      attemptId: payload.attemptId,
      assetId: payload.assetId,
      answers: payload.answers || {},
      markedQuestions: payload.markedQuestions || [],
      titleSnapshot: payload.titleSnapshot || null,
      idempotencyKey: payload.idempotencyKey || newKey('draft')
    }
  })
  return { source: 'tauri', attempt: unwrapCommandResponse(response, 'reading_save_draft') }
}

export async function patchReadingAnswer(attemptId, questionId, answer, marked = false) {
  const response = await invokeCommand('reading_patch_answer', {
    attemptId,
    questionId,
    answer,
    marked
  })
  return !!unwrapCommandResponse(response, 'reading_patch_answer')
}

export async function submitReadingAttempt(payload) {
  const response = await invokeCommand('reading_submit_attempt', {
    cmd: {
      attemptId: payload.attemptId,
      assetId: payload.assetId,
      payload: payload.assetPayload || payload.payload,
      answers: payload.answers || {},
      markedQuestions: payload.markedQuestions || [],
      durationMs: payload.durationMs ?? null,
      titleSnapshot: payload.titleSnapshot || null,
      idempotencyKey: payload.idempotencyKey || newKey('submit')
    }
  })
  return {
    source: 'tauri',
    result: unwrapCommandResponse(response, 'reading_submit_attempt')
  }
}

export const readingRepository = {
  listReadingAssets,
  saveReadingDraft,
  patchReadingAnswer,
  submitReadingAttempt,
  newKey,
  isTauriRuntime
}

export default readingRepository
