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
  const assetId = item.assetId || item.asset_id || null
  const sessionId = item.sessionId || item.session_id || item.id || null
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
    assetId,
    asset_id: assetId,
    examId: assetId,
    sessionId,
    session_id: sessionId,
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
 * Map history detail (AttemptRecord) → reading page submission shape.
 * Correct answers may be absent after field contraction; correctness flags remain.
 */
export function mapHistoryDetailToSubmission(detail) {
  const attempt = detail?.attempt
  if (!attempt) return null
  const activity = String(attempt.activity || detail?.summary?.activity || '').toLowerCase()
  if (activity && activity !== 'reading') return null

  const answers = {}
  const answerComparison = {}
  const markedQuestions = []
  for (const entry of attempt.answers || []) {
    const questionId = String(entry.questionId || entry.question_id || '').trim()
    if (!questionId) continue
    answers[questionId] = entry.answer
    answerComparison[questionId] = {
      questionId,
      userAnswer: entry.answer,
      correctAnswer: entry.correctAnswer ?? entry.correct_answer ?? null,
      isCorrect: entry.isCorrect ?? entry.is_correct ?? null,
      weight: entry.weight ?? 1,
      matchMode: 'single',
      questionKind: entry.questionKind ?? entry.question_kind ?? null
    }
    if (entry.marked) markedQuestions.push(questionId)
  }

  const accuracy = attempt.scoreValue ?? attempt.score_value ?? null
  const durationMs = attempt.durationMs ?? attempt.duration_ms ?? 0
  const assetId = attempt.assetId || attempt.asset_id || detail?.summary?.assetId || null
  const highlights = normalizeAnnotations(
    Array.isArray(attempt.annotations) ? attempt.annotations : []
  )
  return {
    sessionId: attempt.id,
    attemptId: attempt.id,
    assetId,
    examId: assetId,
    activity: 'reading',
    status: attempt.status || 'completed',
    answers,
    answerComparison,
    markedQuestions,
    score: accuracy,
    correctCount: attempt.correctCount ?? attempt.correct_count ?? 0,
    questionCount: attempt.questionCount ?? attempt.question_count ?? 0,
    percentage: accuracy != null ? Math.round(Number(accuracy) * 1000) / 10 : null,
    durationMs,
    duration: Math.round(Number(durationMs || 0) / 1000),
    submittedAt: attempt.submittedAt || attempt.submitted_at || attempt.completedAt || attempt.completed_at || null,
    title: attempt.titleSnapshot || attempt.title_snapshot || detail?.summary?.title || null,
    highlights,
    source: 'tauri-history'
  }
}

function normalizeAnnotations(items) {
  return (items || []).map((item) => {
    const anchor = item?.anchor || {}
    return {
      id: item.id,
      scope: item.scope || 'passage',
      text: anchor.text || item.text || '',
      kind: item.kind || 'highlight',
      questionId: item.questionId || item.question_id || null,
      startOffset: anchor.startOffset ?? anchor.start_offset ?? item.startOffset ?? null,
      endOffset: anchor.endOffset ?? anchor.end_offset ?? item.endOffset ?? null,
      before: anchor.before || item.before || '',
      after: anchor.after || item.after || '',
      occurrence: anchor.occurrence ?? item.occurrence ?? 0,
      createdAt: item.createdAt || item.created_at || null,
      noteText: item.noteText || item.note_text || null
    }
  }).filter((entry) => entry.text)
}

export const historyRepository = {
  listHistory,
  getHistoryDetail,
  exportHistory,
  deleteHistoryAttempt,
  mapHistoryDetailToSubmission,
  isTauriRuntime
}

export default historyRepository
