/**
 * Reading attempt view-model surface (Phase 6).
 * Practice pages consume this instead of assembling submission blobs for Tauri.
 * Electron/Fastify path remains via practiceSessions until Phase 10 cutover.
 */

import {
  listReadingAssets,
  patchReadingAnswer,
  readingRepository,
  saveReadingDraft,
  submitReadingAttempt
} from '@/api/reading-repository.js'
import { isTauriRuntime } from '@/api/tauri-bridge.js'
import { READING_ACTIVITY, normalizeReadingRecordId } from './contracts'

function newAttemptId() {
  return readingRepository.newKey('attempt')
}

function comparisonsToMap(comparisons) {
  const out = {}
  if (!Array.isArray(comparisons)) return out
  for (const entry of comparisons) {
    const qid = String(entry?.questionId || entry?.question_id || '').trim()
    if (!qid) continue
    out[qid] = {
      questionId: qid,
      userAnswer: entry.userAnswer ?? entry.user_answer ?? null,
      correctAnswer: entry.correctAnswer ?? entry.correct_answer ?? null,
      isCorrect: entry.isCorrect ?? entry.is_correct ?? null,
      weight: entry.weight ?? 1,
      matchMode: entry.matchMode ?? entry.match_mode ?? 'single',
      questionKind: entry.questionKind ?? entry.question_kind ?? null
    }
  }
  return out
}

/**
 * Map Tauri ReadingSubmitResult → legacy submission shape used by PracticeReadingPage.
 */
export function mapSubmitResultToSubmission(result, extras = {}) {
  if (!result) return null
  const attempt = result.attempt || {}
  const score = result.score || {}
  const comparison = comparisonsToMap(result.comparisons)
  const correctCount = score.correct ?? attempt.correctCount ?? 0
  const questionCount = score.total ?? attempt.questionCount ?? 0
  const accuracy = score.accuracy ?? attempt.scoreValue ?? null
  return {
    sessionId: attempt.id || extras.attemptId || null,
    attemptId: attempt.id || extras.attemptId || null,
    assetId: attempt.assetId || extras.assetId || null,
    activity: READING_ACTIVITY,
    status: attempt.status || 'submitted',
    score: accuracy,
    correctCount,
    questionCount,
    percentage: score.percentage ?? (accuracy != null ? Math.round(Number(accuracy) * 1000) / 10 : null),
    duration: Math.round((attempt.durationMs || extras.durationMs || 0) / 1000),
    durationMs: attempt.durationMs || extras.durationMs || 0,
    answers: extras.answers || {},
    markedQuestions: extras.markedQuestions || [],
    answerComparison: comparison,
    scoreSummary: score,
    submittedAt: attempt.submittedAt || attempt.completedAt || null,
    title: attempt.titleSnapshot || extras.titleSnapshot || null,
    source: 'tauri',
    idempotentReplay: Boolean(result.idempotentReplay)
  }
}

export function useReadingAttempt(options = {}) {
  const deps = {
    listReadingAssets: options.listReadingAssets || listReadingAssets,
    saveReadingDraft: options.saveReadingDraft || saveReadingDraft,
    patchReadingAnswer: options.patchReadingAnswer || patchReadingAnswer,
    submitReadingAttempt: options.submitReadingAttempt || submitReadingAttempt,
    isTauri: options.isTauri || isTauriRuntime
  }

  function resolveReviewTarget(record) {
    const assetId = normalizeReadingRecordId(record?.assetId || record?.examId)
    const sessionId = normalizeReadingRecordId(record?.sessionId || record?.attemptId || record?.id)
    return {
      activity: READING_ACTIVITY,
      assetId,
      sessionId,
      ready: Boolean(assetId && sessionId)
    }
  }

  async function listAssets() {
    return deps.listReadingAssets()
  }

  /**
   * Persist draft answers. No-op on Electron (caller keeps sessionStorage).
   */
  async function persistDraft({
    attemptId,
    assetId,
    answers,
    markedQuestions,
    titleSnapshot,
    idempotencyKey
  }) {
    if (!deps.isTauri()) {
      return { source: 'electron', attempt: null, skipped: true }
    }
    const id = attemptId || newAttemptId()
    return deps.saveReadingDraft({
      attemptId: id,
      assetId,
      answers: answers || {},
      markedQuestions: markedQuestions || [],
      titleSnapshot: titleSnapshot || null,
      idempotencyKey: idempotencyKey || readingRepository.newKey('draft')
    })
  }

  async function patchAnswer(attemptId, questionId, answer, marked = false) {
    if (!deps.isTauri()) return false
    return deps.patchReadingAnswer(attemptId, questionId, answer, marked)
  }

  /**
   * Idempotent submit. Returns `{ source, submission, raw }` where `submission`
   * matches the view-model PracticeReadingPage already renders.
   */
  async function submit({
    attemptId,
    assetId,
    assetPayload,
    payload,
    answers,
    markedQuestions,
    durationMs,
    titleSnapshot,
    idempotencyKey
  }) {
    if (!deps.isTauri()) {
      return { source: 'electron', submission: null, raw: null, skipped: true }
    }
    const id = attemptId || newAttemptId()
    const { source, result } = await deps.submitReadingAttempt({
      attemptId: id,
      assetId,
      assetPayload: assetPayload || payload,
      payload: assetPayload || payload,
      answers: answers || {},
      markedQuestions: markedQuestions || [],
      durationMs: durationMs ?? null,
      titleSnapshot: titleSnapshot || null,
      idempotencyKey: idempotencyKey || readingRepository.newKey('submit')
    })
    return {
      source,
      raw: result,
      submission: mapSubmitResultToSubmission(result, {
        attemptId: id,
        assetId,
        answers,
        markedQuestions,
        durationMs,
        titleSnapshot
      })
    }
  }

  return {
    resolveReviewTarget,
    listAssets,
    persistDraft,
    patchAnswer,
    submit,
    newAttemptId,
    isTauriRuntime: deps.isTauri,
    mapSubmitResultToSubmission
  }
}

export default useReadingAttempt
