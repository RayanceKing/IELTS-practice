/**
 * Reading attempt repository — Tauri only.
 */

import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

export function newKey(prefix = 'r') {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export async function listReadingAssets() {
  const response = await invokeCommand('reading_list_assets')
  return { source: 'tauri', items: unwrapCommandResponse(response, 'reading_list_assets') || [] }
}

export async function getReadingAssetPayload(assetId) {
  const normalizedAssetId = String(assetId || '').trim()
  if (!normalizedAssetId) {
    const error = new Error('reading_get_asset_payload: assetId is required')
    error.code = 'reading.asset_id_required'
    throw error
  }

  const response = await invokeCommand('reading_get_asset_payload', {
    assetId: normalizedAssetId
  })
  return normalizeReadingAssetEnvelope(
    unwrapCommandResponse(response, 'reading_get_asset_payload')
  )
}

/**
 * Normalize the canonical `{ asset, payload }` DTO. One nested envelope is
 * accepted for old in-memory/cache entries created by the former adapter.
 */
export function normalizeReadingAssetEnvelope(value) {
  const candidate = value?.payload?.asset && Object.prototype.hasOwnProperty.call(value.payload, 'payload')
    ? value.payload
    : value
  if (candidate?.asset && Object.prototype.hasOwnProperty.call(candidate, 'payload')) {
    return { asset: candidate.asset, payload: candidate.payload }
  }
  return { asset: null, payload: candidate ?? null }
}

export async function saveReadingDraft(payload) {
  const response = await invokeCommand('reading_save_draft', {
    cmd: {
      attemptId: payload.attemptId,
      assetId: payload.assetId,
      assetRevision: payload.assetRevision ?? null,
      assetFingerprint: payload.assetFingerprint || null,
      answers: payload.answers || {},
      markedQuestions: payload.markedQuestions || [],
      questionTimeline: payload.questionTimeline || [],
      titleSnapshot: payload.titleSnapshot || null,
      idempotencyKey: payload.idempotencyKey || newKey('draft')
    }
  })
  return { source: 'tauri', attempt: unwrapCommandResponse(response, 'reading_save_draft') }
}

export async function getOpenReadingDraft(assetId) {
  const response = await invokeCommand('reading_get_open_draft', {
    assetId: String(assetId || '').trim()
  })
  return {
    source: 'tauri',
    attempt: unwrapCommandResponse(response, 'reading_get_open_draft') || null
  }
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
      assetRevision: payload.assetRevision ?? null,
      assetFingerprint: payload.assetFingerprint || null,
      answers: payload.answers || {},
      markedQuestions: payload.markedQuestions || [],
      questionTimeline: payload.questionTimeline || [],
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
  getReadingAssetPayload,
  normalizeReadingAssetEnvelope,
  saveReadingDraft,
  getOpenReadingDraft,
  patchReadingAnswer,
  submitReadingAttempt,
  newKey,
  isTauriRuntime
}

export default readingRepository
