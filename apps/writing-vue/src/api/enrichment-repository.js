/**
 * Annotations / dictionary / vocab / coach repository (Phase 8).
 * Coach failures must never rewrite attempt scores — only thread error state.
 */

import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

export async function upsertAnnotation(cmd) {
  if (!isTauriRuntime()) return { source: 'electron', annotation: null }
  const response = await invokeCommand('annotation_upsert', { cmd })
  return { source: 'tauri', annotation: unwrapCommandResponse(response, 'annotation_upsert') }
}

export async function listAnnotations(assetId, attemptId = null) {
  if (!isTauriRuntime()) return { source: 'electron', items: [] }
  const response = await invokeCommand('annotation_list', { assetId, attemptId })
  return { source: 'tauri', items: unwrapCommandResponse(response, 'annotation_list') || [] }
}

export async function deleteAnnotation(id) {
  if (!isTauriRuntime()) return false
  const response = await invokeCommand('annotation_delete', { id })
  return !!unwrapCommandResponse(response, 'annotation_delete')
}

export async function revalidateAnnotations(assetId, scope, document) {
  if (!isTauriRuntime()) return { source: 'electron', items: [] }
  const response = await invokeCommand('annotation_revalidate', { assetId, scope, document })
  return { source: 'tauri', items: unwrapCommandResponse(response, 'annotation_revalidate') || [] }
}

export async function lookupDictionary(term) {
  if (!isTauriRuntime()) return { source: 'electron', entry: null }
  const response = await invokeCommand('dictionary_lookup', { term })
  return { source: 'tauri', entry: unwrapCommandResponse(response, 'dictionary_lookup') }
}

export async function upsertVocab(cmd) {
  if (!isTauriRuntime()) return { source: 'electron', item: null }
  const response = await invokeCommand('vocab_upsert', { cmd })
  return { source: 'tauri', item: unwrapCommandResponse(response, 'vocab_upsert') }
}

export async function listVocab(limit = 100, offset = 0) {
  if (!isTauriRuntime()) return { source: 'electron', items: [] }
  const response = await invokeCommand('vocab_list', { limit, offset })
  return { source: 'tauri', items: unwrapCommandResponse(response, 'vocab_list') || [] }
}

export async function reviewVocab(itemId, grade) {
  if (!isTauriRuntime()) return { source: 'electron', item: null }
  const response = await invokeCommand('vocab_review', { cmd: { itemId, grade } })
  return { source: 'tauri', item: unwrapCommandResponse(response, 'vocab_review') }
}

export async function ensureCoachThread(cmd) {
  if (!isTauriRuntime()) return { source: 'electron', thread: null }
  const response = await invokeCommand('coach_ensure_thread', { cmd })
  return { source: 'tauri', thread: unwrapCommandResponse(response, 'coach_ensure_thread') }
}

export async function appendCoachMessage(cmd) {
  if (!isTauriRuntime()) return { source: 'electron', message: null }
  const response = await invokeCommand('coach_append_message', { cmd })
  return { source: 'tauri', message: unwrapCommandResponse(response, 'coach_append_message') }
}

export async function listCoachMessages(threadId, afterSequence = 0, limit = 100) {
  if (!isTauriRuntime()) return { source: 'electron', items: [] }
  const response = await invokeCommand('coach_list_messages', {
    threadId,
    afterSequence,
    limit
  })
  return { source: 'tauri', items: unwrapCommandResponse(response, 'coach_list_messages') || [] }
}

export async function recordCoachFailure(threadId, error) {
  if (!isTauriRuntime()) return { source: 'electron', thread: null }
  const response = await invokeCommand('coach_record_failure', {
    cmd: {
      threadId,
      error: error && typeof error === 'object' ? error : { message: String(error || 'coach failure') },
      preserveScores: true
    }
  })
  return { source: 'tauri', thread: unwrapCommandResponse(response, 'coach_record_failure') }
}

export const enrichmentRepository = {
  upsertAnnotation,
  listAnnotations,
  deleteAnnotation,
  revalidateAnnotations,
  lookupDictionary,
  upsertVocab,
  listVocab,
  reviewVocab,
  ensureCoachThread,
  appendCoachMessage,
  listCoachMessages,
  recordCoachFailure,
  isTauriRuntime
}

export default enrichmentRepository
