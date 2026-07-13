import type { Activity, HistoryListItemVm } from '@/types/generated/domain'
export interface HistoryQuery { activity?: Activity | null; limit?: number; offset?: number; search?: string | null; startDate?: string | null; endDate?: string | null; minScore?: number | null; maxScore?: number | null }
export function listHistory(query?: HistoryQuery): Promise<{ source: 'tauri'; items: HistoryListItemVm[]; total: number; limit: number; offset: number; nextCursor: string | null }>
export function getHistoryDetail(attemptId: string): Promise<{ source: 'tauri'; detail: unknown }>
export function exportHistory(format?: string, query?: HistoryQuery): Promise<{ source: 'tauri'; result: unknown }>
export function deleteHistoryAttempt(attemptId: string): Promise<unknown>
export function isTauriRuntime(): boolean
