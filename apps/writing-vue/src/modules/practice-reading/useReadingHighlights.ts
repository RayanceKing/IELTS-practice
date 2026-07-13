import { reactive, ref } from 'vue'
import { upsertAnnotation, listAnnotations, lookupDictionary, upsertVocab } from '@/api/enrichment-repository.js'
import { isTauriRuntime } from '@/api/tauri-bridge.js'

export interface HighlightRecord {
  id?: string
  scope?: string
  text?: string
  excerpt?: string
  kind?: string
  questionId?: string | null
  startOffset?: number | null
  endOffset?: number | null
  start?: number | null
  end?: number | null
  before?: string | null
  after?: string | null
  occurrence?: number | null
  createdAt?: string
  noteText?: string | null
  contentFingerprint?: string | null
  mismatch?: unknown
  [key: string]: unknown
}

export interface NormalizedHighlight {
  scope: 'passage' | 'questions' | 'unknown'
  text: string
  kind: 'note' | 'highlight'
  questionId: string | null
  startOffset: number | null
  endOffset: number | null
  before: string
  after: string
  occurrence: number
  createdAt: string
}

export interface DictionaryEntry {
  term?: string
  meaning?: string
  definition?: string
  example?: string
  phonetic?: string
  partOfSpeech?: string
  [key: string]: unknown
}

function normalizeComparableText(value: unknown): string {
  return String(value || '').replace(/\s+/g, ' ').trim()
}

function createDictionaryBubbleState() {
  return reactive({
    visible: false,
    term: '',
    meaning: '',
    definition: '',
    example: '',
    meta: '',
    sourceLine: '',
    parts: [] as string[],
    phonetic: '',
    partOfSpeech: '',
    sourceLabel: '',
    license: '',
    found: false,
    saved: false,
    left: 0,
    top: 0
  })
}

export function useReadingHighlights() {
  const selectionToolbarVisible = ref(false)
  const selectionToolbarStyle = reactive({ top: '0px', left: '0px' })
  const keepSelectionToolbar = ref(false)
  const highlightSnapshot = ref<NormalizedHighlight[]>([])
  const dictionaryBubble = createDictionaryBubbleState()

  async function persistHighlightToStore(
    assetId: string | null | undefined,
    entry: HighlightRecord | null | undefined,
    attemptId: string | null = null
  ) {
    if (!isTauriRuntime() || !assetId || !entry?.text) return null
    try {
      const { annotation } = await upsertAnnotation({
        attemptId,
        assetId,
        scope: entry.scope || 'passage',
        questionId: entry.questionId || null,
        kind: entry.kind || 'highlight',
        noteText: entry.noteText || null,
        anchor: {
          text: entry.text,
          before: entry.before || null,
          after: entry.after || null,
          occurrence: entry.occurrence || 0,
          startOffset: entry.startOffset ?? null,
          endOffset: entry.endOffset ?? null,
          contentFingerprint: entry.contentFingerprint || null
        }
      }) as { annotation?: unknown }
      return annotation
    } catch (err) {
      console.warn('persist highlight failed', err)
      return null
    }
  }

  async function loadPersistedHighlights(
    assetId: string | null | undefined,
    attemptId: string | null = null
  ) {
    if (!isTauriRuntime() || !assetId) return [] as NormalizedHighlight[]
    try {
      const { items } = await listAnnotations(assetId, attemptId) as {
        items?: Array<{
          id?: string
          scope?: string
          kind?: string
          questionId?: string | null
          noteText?: string | null
          createdAt?: string
          mismatch?: unknown
          anchor?: {
            text?: string
            startOffset?: number | null
            endOffset?: number | null
            before?: string | null
            after?: string | null
            occurrence?: number | null
          }
        }>
      }
      return normalizeHighlightSnapshot(
        (items || []).map((item) => ({
          scope: item.scope,
          text: item.anchor?.text,
          kind: item.kind,
          questionId: item.questionId,
          startOffset: item.anchor?.startOffset,
          endOffset: item.anchor?.endOffset,
          before: item.anchor?.before,
          after: item.anchor?.after,
          occurrence: item.anchor?.occurrence,
          createdAt: item.createdAt,
          noteText: item.noteText,
          mismatch: item.mismatch || null,
          id: item.id
        }))
      )
    } catch (err) {
      console.warn('load highlights failed', err)
      return [] as NormalizedHighlight[]
    }
  }

  async function lookupTermInDictionary(term: string) {
    if (!isTauriRuntime()) return null
    try {
      const { entry } = await lookupDictionary(term) as { entry?: DictionaryEntry | null }
      return entry || null
    } catch (err) {
      console.warn('dictionary lookup failed', err)
      return null
    }
  }

  async function saveTermToVocab(
    entry: DictionaryEntry | null | undefined,
    assetId: string | null = null,
    attemptId: string | null = null
  ) {
    if (!isTauriRuntime() || !entry?.term) return null
    try {
      const { item } = await upsertVocab({
        term: entry.term,
        definition: entry.definition || entry.meaning || null,
        phonetic: entry.phonetic || null,
        partOfSpeech: entry.partOfSpeech || null,
        example: entry.example || null,
        sourceAssetId: assetId,
        sourceAttemptId: attemptId,
        tags: ['reading']
      }) as { item?: unknown }
      return item
    } catch (err) {
      console.warn('save vocab failed', err)
      return null
    }
  }

  function normalizeHighlightSnapshot(value: unknown): NormalizedHighlight[] {
    if (!Array.isArray(value)) {
      return []
    }
    return value
      .map((entry) => {
        if (!entry || typeof entry !== 'object') return null
        const record = entry as HighlightRecord
        const text = normalizeComparableText(record.text || record.excerpt)
        if (!text) return null
        const scope = String(record.scope || '').trim().toLowerCase()
        const startOffset = Number(record.startOffset ?? record.start)
        const endOffset = Number(record.endOffset ?? record.end)
        return {
          scope: (scope === 'passage' || scope === 'questions' ? scope : 'unknown') as NormalizedHighlight['scope'],
          text,
          kind: (record.kind === 'note' ? 'note' : 'highlight') as NormalizedHighlight['kind'],
          questionId: record.questionId ? String(record.questionId).trim() : null,
          startOffset: Number.isFinite(startOffset) ? startOffset : null,
          endOffset: Number.isFinite(endOffset) ? endOffset : null,
          before: normalizeComparableText(record.before),
          after: normalizeComparableText(record.after),
          occurrence: Number.isFinite(Number(record.occurrence)) ? Math.max(0, Number(record.occurrence)) : 0,
          createdAt: record.createdAt || new Date().toISOString()
        }
      })
      .filter((entry): entry is NormalizedHighlight => Boolean(entry))
  }

  function closeSelectionToolbar() {
    selectionToolbarVisible.value = false
    keepSelectionToolbar.value = false
  }

  function closeDictionaryBubble() {
    dictionaryBubble.visible = false
    dictionaryBubble.term = ''
    dictionaryBubble.meaning = ''
    dictionaryBubble.definition = ''
    dictionaryBubble.example = ''
    dictionaryBubble.meta = ''
    dictionaryBubble.sourceLine = ''
    dictionaryBubble.parts = []
    dictionaryBubble.phonetic = ''
    dictionaryBubble.partOfSpeech = ''
    dictionaryBubble.sourceLabel = ''
    dictionaryBubble.license = ''
    dictionaryBubble.found = false
    dictionaryBubble.saved = false
  }

  function resetHighlightUiState() {
    highlightSnapshot.value = []
    closeSelectionToolbar()
    closeDictionaryBubble()
  }

  return {
    selectionToolbarVisible,
    selectionToolbarStyle,
    keepSelectionToolbar,
    highlightSnapshot,
    dictionaryBubble,
    persistHighlightToStore,
    loadPersistedHighlights,
    lookupTermInDictionary,
    saveTermToVocab,
    normalizeHighlightSnapshot,
    closeSelectionToolbar,
    closeDictionaryBubble,
    resetHighlightUiState
  }
}
