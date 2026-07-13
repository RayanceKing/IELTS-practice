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
  listHistoryAll,
  getHistoryDetail,
  deleteHistoryAttempt,
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
    const loaded = await getReadingAssetPayload(normalizedAssetId)
    return {
      ...meta,
      ...(loaded.asset || {}),
      activity: activity || 'reading',
      refresh: !!options.refresh,
      payload: loaded.payload
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
      assetRevision: payload.assetRevision ?? payload.asset?.schemaVersion ?? null,
      assetFingerprint: payload.assetFingerprint || payload.asset?.fingerprint || null,
      answers: payload.answers || payload.attempt?.answers || {},
      markedQuestions: payload.markedQuestions || payload.attempt?.markedQuestions || [],
      questionTimeline: payload.questionTimeline || payload.attempt?.questionTimelineLite || [],
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
      assetRevision: payload.assetRevision ?? payload.asset?.schemaVersion ?? null,
      assetFingerprint: payload.assetFingerprint || payload.asset?.fingerprint || null,
      answers: payload.answers || payload.attempt?.answers || {},
      markedQuestions: payload.markedQuestions || payload.attempt?.markedQuestions || [],
      questionTimeline: payload.questionTimeline || payload.attempt?.questionTimelineLite || [],
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
    const result = await listHistoryAll({
      activity: filters.activity || null,
      search: filters.search || null
    })
    return {
      data: result.items || [],
      total: result.total,
      page: 1,
      limit: (result.items || []).length
    }
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
    const result = await listHistoryAll({
      activity: filters.activity || null
    })
    for (const item of result.items || []) {
      await deleteHistoryAttempt(item.id)
    }
    return { deleted: (result.items || []).length }
  },

  async exportArchive(filters = { activity: 'reading' }) {
    const listed = await listHistoryAll({
      activity: filters.activity || 'reading',
      search: filters.search || null
    })
    const submissions = []
    for (const item of listed.items || []) {
      try {
        const { detail } = await getHistoryDetail(item.id)
        const submission = mapHistoryDetailToSubmission(detail)
        if (submission) {
          submissions.push({
            ...submission,
            id: submission.sessionId || submission.attemptId || item.id,
            title: submission.title || item.title || null
          })
        } else if (detail?.attempt) {
          // Non-reading or partial: still include attempt snapshot for restore.
          const attempt = detail.attempt
          submissions.push({
            id: attempt.id,
            sessionId: attempt.id,
            assetId: attempt.assetId || attempt.asset_id || item.assetId || null,
            examId: attempt.assetId || attempt.asset_id || item.assetId || null,
            activity: attempt.activity || 'reading',
            status: attempt.status || 'completed',
            answers: Object.fromEntries(
              (attempt.answers || []).map((entry) => [
                entry.questionId || entry.question_id,
                entry.answer
              ]).filter(([qid]) => qid)
            ),
            markedQuestions: (attempt.answers || [])
              .filter((entry) => entry.marked)
              .map((entry) => entry.questionId || entry.question_id)
              .filter(Boolean),
            score: attempt.scoreValue ?? attempt.score_value ?? null,
            correctCount: attempt.correctCount ?? attempt.correct_count ?? 0,
            questionCount: attempt.questionCount ?? attempt.question_count ?? 0,
            durationMs: attempt.durationMs ?? attempt.duration_ms ?? 0,
            submittedAt: attempt.submittedAt || attempt.submitted_at || null,
            title: attempt.titleSnapshot || attempt.title_snapshot || item.title || null
          })
        }
      } catch (err) {
        console.warn('exportArchive skip item', item?.id, err)
      }
    }
    const exportedAt = new Date().toISOString()
    return {
      activity: filters.activity || 'reading',
      schemaVersion: 'practice-history-archive.v1',
      exportedAt,
      count: submissions.length,
      submissions
    }
  },

  async importArchive(activity, payload) {
    const archive = payload?.archive && typeof payload.archive === 'object'
      ? payload.archive
      : payload
    const response = await invokeCommand('import_reading_archive_value', {
      value: archive
    })
    const report = unwrapCommandResponse(response, 'import_reading_archive_value') || {}
    return {
      importedCount: Number(report.importedCount ?? report.imported ?? 0),
      skippedCount: Number(report.skippedCount ?? 0),
      failedCount: Number(report.failedCount ?? report.failed ?? 0),
      attemptIds: report.attemptIds || report.attempt_ids || [],
      errors: report.errors || []
    }
  }
}

export const practiceCoach = {
  async listMessages(activity, sessionId = null, payload = {}) {
    const { thread } = await ensureCoachThread({
      activity: activity || 'reading',
      attemptId: sessionId || payload?.attemptId || null,
      assetId: payload?.assetId || null,
      kind: payload?.kind || 'practice'
    })
    const threadId = thread?.id
    if (!threadId) {
      const error = new Error('coach thread missing id')
      error.code = 'coach.error'
      throw error
    }
    const { items } = await listCoachMessages(threadId, 0, 100)
    return { threadId, messages: items }
  },

  async query(activity, payload, sessionId = null, options = {}) {
    const threadCmd = {
      activity: activity || 'reading',
      attemptId: sessionId || payload?.attemptId || null,
      assetId: payload?.assetId || null,
      kind: payload?.kind || 'practice'
    }
    const { thread } = await ensureCoachThread(threadCmd)
    const threadId = thread?.id
    if (!threadId) {
      const err = new Error('coach thread missing id')
      err.code = 'coach.error'
      throw err
    }

    const userText = payload?.content || ''
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
            activity: activity || 'reading',
            sessionId: sessionId || null,
            assetId: payload?.assetId || payload?.examId || null,
            mode: payload?.mode || 'single',
            locale: payload?.locale || 'zh',
            surface: payload?.surface || 'chat_widget',
            action: payload?.action || 'chat',
            promptKind: payload?.promptKind || 'freeform',
            selectedText: payload?.selectedText || '',
            selectedContext: payload?.selectedContext || null,
            focusQuestionNumbers: payload?.focusQuestionNumbers || [],
            attemptContext: payload?.attemptContext || null
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
