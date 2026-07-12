/**
 * Practice surface — Tauri repositories only (Fastify removed).
 */

import {
  listReadingAssets,
  saveReadingDraft,
  submitReadingAttempt,
  newKey as readingKey
} from '@/api/reading-repository.js'
import {
  createSuite,
  getSuite,
  submitSuitePassage,
  cancelSuite
} from '@/api/modes-repository.js'
import {
  listHistory,
  getHistoryDetail,
  deleteHistoryAttempt,
  exportHistory
} from '@/api/history-repository.js'
import {
  ensureCoachThread,
  appendCoachMessage,
  listCoachMessages,
  recordCoachFailure
} from '@/api/enrichment-repository.js'
import { invokeCommand, unwrapCommandResponse } from '@/api/tauri-bridge.js'

function newKey(prefix = 'p') {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export const practiceAssets = {
  async list(filters = {}, pagination = { page: 1, limit: 20 }) {
    const { items } = await listReadingAssets()
    let rows = items || []
    if (filters.activity && filters.activity !== 'reading') {
      rows = []
    }
    if (filters.search) {
      const q = String(filters.search).toLowerCase()
      rows = rows.filter((item) => String(item.title || item.id || '').toLowerCase().includes(q))
    }
    const page = Number(pagination.page || 1)
    const limit = Number(pagination.limit || 20)
    const offset = (page - 1) * limit
    return {
      data: rows.slice(offset, offset + limit),
      total: rows.length,
      page,
      limit
    }
  },

  async listAll(filters = {}, options = {}) {
    const result = await this.list(filters, { page: 1, limit: 10000 })
    return result
  },

  async get(activity, assetId, options = {}) {
    const { items } = await listReadingAssets()
    const meta = (items || []).find((item) => String(item.id) === String(assetId)) || null
    // Full payload load is not yet a dedicated command; return index entry.
    // Callers that need answerKey must supply payload at submit time or import assets.
    if (!meta) {
      const err = new Error(`asset not found: ${assetId}`)
      err.code = 'not_found'
      throw err
    }
    return {
      ...meta,
      activity: activity || 'reading',
      refresh: !!options.refresh
    }
  }
}

export const practiceSessions = {
  async create(payload) {
    // Map legacy "create session + submit" to reading submit.
    const attemptId = payload.sessionId || payload.attemptId || newKey('reading')
    const result = await submitReadingAttempt({
      attemptId,
      assetId: payload.assetId || payload.examId || payload.asset?.id,
      assetPayload: payload.payload || payload.assetPayload || payload.asset,
      answers: payload.answers || payload.attempt?.answers || {},
      markedQuestions: payload.markedQuestions || payload.attempt?.markedQuestions || [],
      durationMs: payload.durationMs ?? payload.attempt?.durationMs ?? null,
      titleSnapshot: payload.titleSnapshot || payload.title || null,
      idempotencyKey: payload.idempotencyKey || newKey('submit')
    })
    return result.result
  },

  async getState(activity, sessionId) {
    const { detail } = await getHistoryDetail(sessionId)
    return detail || { id: sessionId, activity, status: 'unknown' }
  },

  async cancel(activity, sessionId) {
    // No open cancel command for single reading; treat as no-op success.
    return { id: sessionId, activity, status: 'cancelled' }
  }
}

export const practiceReadingSuite = {
  async create(payload = {}) {
    const { session } = await createSuite(payload)
    return session
  },

  async get(sessionId) {
    const { session } = await getSuite(sessionId)
    return session
  },

  async submitPassage(sessionId, assetId, payload = {}) {
    const { result } = await submitSuitePassage({
      suiteId: sessionId,
      assetId,
      assetPayload: payload.payload || payload.assetPayload || payload.asset,
      answers: payload.answers || payload.attempt?.answers || {},
      markedQuestions: payload.markedQuestions || payload.attempt?.markedQuestions || [],
      durationMs: payload.durationMs ?? payload.attempt?.durationMs ?? null,
      titleSnapshot: payload.titleSnapshot || null,
      timerSnapshot: payload.timerSnapshot || null,
      idempotencyKey: payload.idempotencyKey || newKey('suite-submit')
    })
    return result
  },

  async cancel(sessionId) {
    const { session } = await cancelSuite(sessionId)
    return session
  }
}

export const practiceHistory = {
  async list(filters = {}, pagination = { page: 1, limit: 20 }) {
    const page = Number(pagination.page || 1)
    const limit = Number(pagination.limit || 20)
    const offset = (page - 1) * limit
    const result = await listHistory({
      activity: filters.activity || null,
      limit,
      offset,
      search: filters.search || null
    })
    return {
      data: result.items || [],
      total: result.total,
      page,
      limit
    }
  },

  async listAll(filters = {}, options = {}) {
    return this.list(filters, { page: 1, limit: 10000 })
  },

  async get(activity, recordId) {
    const { detail } = await getHistoryDetail(recordId)
    return detail
  },

  async delete(activity, recordId) {
    await deleteHistoryAttempt(recordId)
    return true
  },

  async clear(filters = {}) {
    const result = await listHistory({
      activity: filters.activity || null,
      limit: 10000,
      offset: 0
    })
    for (const item of result.items || []) {
      await deleteHistoryAttempt(item.id)
    }
    return { deleted: (result.items || []).length }
  },

  async exportArchive(filters = { activity: 'reading' }) {
    const { result } = await exportHistory('json', filters)
    return result
  },

  async importArchive(activity, payload) {
    // Optional cold-path: browser export import via Tauri when command exists.
    try {
      const response = await invokeCommand('import_browser_export_value', {
        value: payload,
        activity: activity || 'reading'
      })
      return unwrapCommandResponse(response, 'import_browser_export_value')
    } catch (err) {
      const error = new Error(
        `importArchive requires optional legacy import command: ${err?.message || err}`
      )
      error.code = 'not_implemented'
      throw error
    }
  }
}

export const practiceCoach = {
  async query(activity, payload, sessionId = null, options = {}) {
    const threadCmd = {
      activity: activity || 'reading',
      attemptId: sessionId || payload?.attemptId || null,
      assetId: payload?.assetId || null,
      scope: payload?.scope || 'practice'
    }
    const { thread } = await ensureCoachThread(threadCmd)
    const threadId = thread?.id
    if (!threadId) {
      const err = new Error('coach thread missing id')
      err.code = 'coach.error'
      throw err
    }

    const userText = payload?.question || payload?.message || payload?.text || ''
    if (userText) {
      await appendCoachMessage({
        threadId,
        role: 'user',
        content: userText
      })
    }

    // No LLM stream on product path yet — surface explicit thread state.
    const { items } = await listCoachMessages(threadId, 0, 100)
    if (typeof options.onEvent === 'function') {
      try {
        options.onEvent({
          event: 'complete',
          data: {
            threadId,
            messages: items,
            message: 'Coach stream not wired to LLM provider on Tauri path yet'
          }
        })
      } catch (_) {
        // ignore listener errors
      }
    }
    return {
      threadId,
      messages: items,
      degraded: true,
      message: 'Coach stream not wired to LLM provider on Tauri path yet'
    }
  }
}

export const practiceMigration = {
  async getStatus() {
    return {
      engine: 'tauri-sqlite-v2',
      fastify: false,
      electron: false
    }
  }
}

export default {
  assets: practiceAssets,
  sessions: practiceSessions,
  readingSuite: practiceReadingSuite,
  history: practiceHistory,
  coach: practiceCoach,
  migration: practiceMigration
}
