import type { PracticeAssetV2, AttemptRecord } from '@/types/generated/domain'

export interface ReadingListResult { source: 'tauri'; items: PracticeAssetV2[] }
export interface ReadingPayload { [key: string]: unknown }
export interface ReadingDraftInput {
  attemptId: string; assetId: string; answers?: Record<string, unknown>; markedQuestions?: string[]
  titleSnapshot?: string | null; idempotencyKey?: string
}
export function newKey(prefix?: string): string
export function listReadingAssets(): Promise<ReadingListResult>
export function getReadingAssetPayload(assetId: string): Promise<ReadingPayload>
export function saveReadingDraft(payload: ReadingDraftInput): Promise<{ source: 'tauri'; attempt: AttemptRecord }>
export function patchReadingAnswer(attemptId: string, questionId: string, answer: unknown, marked?: boolean): Promise<boolean>
export function submitReadingAttempt(payload: ReadingDraftInput & { assetPayload?: unknown; payload?: unknown; durationMs?: number | null }): Promise<{ source: 'tauri'; result: unknown }>
export const readingRepository: { listReadingAssets: typeof listReadingAssets; getReadingAssetPayload: typeof getReadingAssetPayload; saveReadingDraft: typeof saveReadingDraft; patchReadingAnswer: typeof patchReadingAnswer; submitReadingAttempt: typeof submitReadingAttempt; newKey: typeof newKey }
export function isTauriRuntime(): boolean
