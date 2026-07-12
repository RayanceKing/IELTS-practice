/**
 * Unified history repository — Tauri SQLite v2 only.
 */

import { invokeCommand, isTauriRuntime, unwrapCommandResponse } from '@/api/tauri-bridge.js'

/**
 * @param {object} query
 */
export async function listHistory(query = {}) {
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

export async function getHistoryDetail(attemptId) {
  const response = await invokeCommand('get_history_detail', { attemptId })
  return {
    source: 'tauri',
    detail: unwrapCommandResponse(response, 'get_history_detail')
  }
}

export async function exportHistory(format = 'csv', query = {}) {
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

export async function deleteHistoryAttempt(attemptId) {
  const response = await invokeCommand('delete_history_attempt', { attemptId })
  return unwrapCommandResponse(response, 'delete_history_attempt')
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

export const historyRepository = {
  listHistory,
  getHistoryDetail,
  exportHistory,
  deleteHistoryAttempt,
  isTauriRuntime
}

export default historyRepository
