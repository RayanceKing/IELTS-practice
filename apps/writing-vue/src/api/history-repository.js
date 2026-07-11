/**
 * Unified history repository (Phase 4).
 *
 * UI must call this module only — never merge essays + practice history itself.
 *
 * - Tauri path: single `list_history` command against SQLite v2.
 * - Electron fallback: repository performs dual fetch + merge so the page stays single-source.
 */

import { essays as essaysApi } from '@/api/client.js'
import { practiceHistory } from '@/api/practice-client.js'
import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

/**
 * @typedef {Object} HistoryListQuery
 * @property {'reading'|'writing'|null} [activity]
 * @property {number} [limit]
 * @property {number} [offset]
 * @property {string} [search]
 * @property {string} [startDate]
 * @property {string} [endDate]
 * @property {number|null} [minScore]
 * @property {number|null} [maxScore]
 */

/**
 * @param {HistoryListQuery} query
 */
export async function listHistory(query = {}) {
  if (isTauriRuntime()) {
    const response = await invokeCommand('list_history', {
      query: {
        activity: query.activity || null,
        limit: query.limit ?? 20,
        offset: query.offset ?? 0,
        search: query.search || null,
        startDate: query.startDate || null,
        endDate: query.endDate || null,
        minScore: query.minScore ?? null,
        maxScore: query.maxScore ?? null
      }
    })
    const page = unwrapCommandResponse(response, 'list_history')
    return {
      source: 'tauri',
      items: (page?.items || []).map(normalizeUnifiedItem),
      total: Number(page?.total || 0),
      limit: Number(page?.limit || query.limit || 20),
      offset: Number(page?.offset || query.offset || 0),
      nextCursor: page?.nextCursor || null
    }
  }

  return listHistoryElectronFallback(query)
}

export async function getHistoryDetail(attemptId) {
  if (isTauriRuntime()) {
    const response = await invokeCommand('get_history_detail', { attemptId })
    return {
      source: 'tauri',
      detail: unwrapCommandResponse(response, 'get_history_detail')
    }
  }
  // Electron path remains split; callers that need detail keep existing pages for now.
  return { source: 'electron', detail: null, attemptId }
}

export async function exportHistory(format = 'csv', query = {}) {
  if (isTauriRuntime()) {
    const response = await invokeCommand('export_history', {
      cmd: {
        format,
        query: {
          activity: query.activity || null,
          limit: query.limit ?? 10000,
          offset: 0,
          search: query.search || null,
          startDate: query.startDate || null,
          endDate: query.endDate || null,
          minScore: query.minScore ?? null,
          maxScore: query.maxScore ?? null
        }
      }
    })
    return {
      source: 'tauri',
      result: unwrapCommandResponse(response, 'export_history')
    }
  }

  // Electron: keep writing CSV path; reading optional append.
  const csv = await essaysApi.exportCSV?.(buildElectronWritingFilters(query)).catch(() => '')
  return {
    source: 'electron',
    result: {
      format: 'csv',
      body: csv || '',
      recordCount: 0
    }
  }
}

export async function deleteHistoryAttempt(attemptId, activityHint) {
  if (isTauriRuntime()) {
    const response = await invokeCommand('delete_history_attempt', { attemptId })
    return unwrapCommandResponse(response, 'delete_history_attempt')
  }
  if (activityHint === 'reading') {
    await practiceHistory.remove?.(attemptId)
    return true
  }
  await essaysApi.remove?.(attemptId)
  return true
}

function normalizeUnifiedItem(item) {
  const activity = item.activity || 'writing'
  const scoreValue = item.scoreValue ?? item.score_value ?? null
  const scoreDisplay = item.scoreDisplay || item.score_display || '—'
  return {
    id: item.id,
    activity,
    title: item.title || 'Untitled',
    status: item.status,
    mode: item.mode,
    submitted_at: item.submittedAt || item.submitted_at || '',
    duration_ms: item.durationMs ?? item.duration_ms ?? 0,
    score_value: scoreValue,
    score_scale: item.scoreScale || item.score_scale || null,
    score_label: item.scoreLabel || item.score_label || (activity === 'reading' ? 'Accuracy' : 'Overall Band'),
    score_display: scoreDisplay,
    // Legacy HistoryPage fields
    task_type: activity === 'reading' ? 'reading' : item.taskType || 'task2',
    display_topic_title: item.title || 'Untitled',
    topic_title: item.title || 'Untitled',
    total_score: activity === 'writing'
      ? Number(scoreValue ?? 0)
      : Number(scoreValue ?? 0) * 10,
    reading_accuracy: activity === 'reading' && scoreValue != null
      ? Math.round(Number(scoreValue) * 100)
      : undefined,
    duration: Math.round((item.durationMs ?? item.duration_ms ?? 0) / 1000),
    source: 'unified'
  }
}

/**
 * Dual-fetch merge lives HERE so HistoryPage never merges two result sets.
 * Remove this path when Electron is deleted (Phase 10).
 */
async function listHistoryElectronFallback(query) {
  const limit = Math.max(1, query.limit ?? 20)
  const offset = Math.max(0, query.offset ?? 0)
  const activity = query.activity || null

  const loadWriting = activity !== 'reading'
  const loadReading = activity !== 'writing'
  const mergedLimit = Math.max(1, offset + limit)

  const [writingResult, readingResult] = await Promise.all([
    loadWriting
      ? essaysApi.list(buildElectronWritingFilters(query), { page: 1, limit: mergedLimit })
      : Promise.resolve({ data: [], total: 0 }),
    loadReading
      ? practiceHistory.listAll({ activity: 'reading' })
      : Promise.resolve({ data: [], total: 0 })
  ])

  const writingRecords = (Array.isArray(writingResult.data) ? writingResult.data : []).map((record) =>
    normalizeUnifiedItem({
      id: record.id,
      activity: 'writing',
      title: record.display_topic_title || record.topic_title || '写作练习',
      status: 'completed',
      mode: 'bank',
      submittedAt: record.submitted_at || record.created_at || '',
      durationMs: Number(record.duration || 0) * 1000,
      scoreValue: Number(record.total_score ?? record.overall_score ?? 0),
      scoreScale: 'band9',
      scoreLabel: 'Overall Band',
      scoreDisplay: Number(record.total_score ?? record.overall_score ?? 0).toFixed(1)
    })
  )

  const readingRecords = (Array.isArray(readingResult.data) ? readingResult.data : [])
    .map((record) => {
      const accuracy = Number(record.accuracy || 0)
      return normalizeUnifiedItem({
        id: record.id,
        activity: 'reading',
        title: record.title || record.examId || '阅读练习',
        status: 'completed',
        mode: 'single',
        submittedAt: record.submittedAt || record.endTime || '',
        durationMs: Number(record.duration || 0) * 1000,
        scoreValue: accuracy,
        scoreScale: 'ratio',
        scoreLabel: 'Accuracy',
        scoreDisplay: `${Math.round(accuracy * 100)}%`
      })
    })
    .filter((record) => matchElectronFilters(record, query))

  const combined = [...writingRecords, ...readingRecords].sort(
    (a, b) => new Date(b.submitted_at || 0).getTime() - new Date(a.submitted_at || 0).getTime()
  )
  const total =
    (loadWriting ? Number(writingResult.total || writingRecords.length || 0) : 0) +
    (loadReading ? readingRecords.length : 0)

  return {
    source: 'electron-fallback',
    items: combined.slice(offset, offset + limit),
    total,
    limit,
    offset,
    nextCursor: offset + limit < total ? String(offset + limit) : null
  }
}

function buildElectronWritingFilters(query) {
  const apiFilters = {}
  if (query.startDate) apiFilters.start_date = query.startDate
  if (query.endDate) apiFilters.end_date = query.endDate
  if (query.minScore != null && query.minScore !== '') apiFilters.min_score = query.minScore
  if (query.maxScore != null && query.maxScore !== '') apiFilters.max_score = query.maxScore
  if (query.search && String(query.search).trim()) apiFilters.search = String(query.search).trim()
  return apiFilters
}

function matchElectronFilters(record, query) {
  if (query.startDate && String(record.submitted_at || '').slice(0, 10) < query.startDate) return false
  if (query.endDate && String(record.submitted_at || '').slice(0, 10) > query.endDate) return false
  if (query.minScore != null && query.minScore !== '' && Number(record.total_score || 0) < Number(query.minScore)) {
    return false
  }
  if (query.maxScore != null && query.maxScore !== '' && Number(record.total_score || 0) > Number(query.maxScore)) {
    return false
  }
  const q = String(query.search || '').trim().toLowerCase()
  if (!q) return true
  return [record.display_topic_title, record.topic_title, record.id]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
    .includes(q)
}

export const historyRepository = {
  listHistory,
  getHistoryDetail,
  exportHistory,
  deleteHistoryAttempt,
  isTauriRuntime
}

export default historyRepository
