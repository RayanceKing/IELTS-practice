export function upsertAnnotation(cmd: Record<string, unknown>): Promise<{ annotation?: unknown }>
export function listAnnotations(
  assetId: string,
  attemptId?: string | null
): Promise<{ items?: unknown[] }>
export function deleteAnnotation(id: string): Promise<unknown>
export function revalidateAnnotations(
  assetId: string,
  scope: string,
  document: unknown
): Promise<unknown>
export function lookupDictionary(term: string): Promise<{ entry?: Record<string, unknown> | null }>
export function upsertVocab(cmd: Record<string, unknown>): Promise<{ item?: unknown }>
export function listVocab(limit?: number, offset?: number): Promise<unknown>
export function reviewVocab(itemId: string, grade: unknown): Promise<unknown>
export function ensureCoachThread(cmd: Record<string, unknown>): Promise<unknown>
export function appendCoachMessage(cmd: Record<string, unknown>): Promise<unknown>
export function listCoachMessages(
  threadId: string,
  afterSequence?: number,
  limit?: number
): Promise<unknown>
export function recordCoachFailure(threadId: string, error: unknown): Promise<unknown>

export const enrichmentRepository: Record<string, unknown>
export default enrichmentRepository
