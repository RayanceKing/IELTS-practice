/**
 * Practice surface — Tauri repositories only (Fastify removed).
 */

import {
  listReadingAssets,
  getReadingAssetPayload,
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
  exportHistory,
  mapHistoryDetailToSubmission
} from '@/api/history-repository.js'
import {
  ensureCoachThread,
  listCoachMessages
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
    const normalizedAssetId = String(assetId || '').trim()
    const { items } = await listReadingAssets()
    const meta = (items || []).find((item) => String(item.id) === normalizedAssetId) || null
    if (!meta) {
      const err = new Error(`asset not found: ${assetId}`)
      err.code = 'not_found'
      throw err
    }
    const payload = await getReadingAssetPayload(normalizedAssetId)
    return {
      ...meta,
      activity: activity || 'reading',
      refresh: !!options.refresh,
      payload
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
    if (!detail) {
      return { id: sessionId, activity, status: 'unknown' }
    }
    const submission = mapHistoryDetailToSubmission(detail)
    return {
      ...detail,
      id: sessionId,
      activity: activity || detail?.summary?.activity || 'reading',
      status: detail?.attempt?.status || detail?.summary?.status || 'completed',
      submission
    }
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
    if (!String(userText).trim()) {
      const error = new Error('阅读教练问题不能为空')
      error.code = 'coach.empty_question'
      throw error
    }

    const notify = (event, data = {}) => {
      if (typeof options.onEvent !== 'function') return
      options.onEvent({ event, data })
    }

    notify('start', { threadId })
    try {
      notify('generation_start', { threadId })
      const response = await invokeCommand('coach_run', {
        cmd: {
          threadId,
          content: String(userText).trim(),
          questionContext: {
            ...payload,
            activity: activity || 'reading',
            sessionId: sessionId || null
          }
        }
      })
      const result = unwrapCommandResponse(response, 'coach_run') || {}
      const answer = String(result.assistantMessage?.content || '').trim()
      if (!answer) {
        const error = new Error('阅读教练未返回有效回答')
        error.code = 'invalid_response_format'
        throw error
      }
      const { items } = await listCoachMessages(threadId, 0, 100)
      notify('generation_complete', { threadId })
      notify('complete', { ...result, threadId, messages: items, answer })
      return { ...result, threadId, messages: items, answer, degraded: false }
    } catch (error) {
      notify('error', {
        threadId,
        code: error?.code || 'coach.error',
        message: error?.message || '阅读教练请求失败'
      })
      throw error
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
