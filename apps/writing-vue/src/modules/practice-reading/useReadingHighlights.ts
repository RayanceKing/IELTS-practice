import { reactive, ref } from 'vue'
import { upsertAnnotation, listAnnotations, lookupDictionary, upsertVocab } from '@/api/enrichment-repository.js'
import { isTauriRuntime } from '@/api/tauri-bridge.js'

function normalizeComparableText(value) {
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
    parts: [],
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
  const highlightSnapshot = ref([])
  const dictionaryBubble = createDictionaryBubbleState()

  async function persistHighlightToStore(assetId, entry, attemptId = null) {
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
      })
      return annotation
    } catch (err) {
      console.warn('persist highlight failed', err)
      return null
    }
  }

  async function loadPersistedHighlights(assetId, attemptId = null) {
    if (!isTauriRuntime() || !assetId) return []
    try {
      const { items } = await listAnnotations(assetId, attemptId)
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
      return []
    }
  }

  async function lookupTermInDictionary(term) {
    if (!isTauriRuntime()) return null
    try {
      const { entry } = await lookupDictionary(term)
      return entry
    } catch (err) {
      console.warn('dictionary lookup failed', err)
      return null
    }
  }

  async function saveTermToVocab(entry, assetId = null, attemptId = null) {
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
      })
      return item
    } catch (err) {
      console.warn('save vocab failed', err)
      return null
    }
  }

  function normalizeHighlightSnapshot(value) {
    if (!Array.isArray(value)) {
      return []
    }
    return value
      .map((entry) => {
        if (!entry || typeof entry !== 'object') return null
        const text = normalizeComparableText(entry.text || entry.excerpt)
        if (!text) return null
        const scope = String(entry.scope || '').trim().toLowerCase()
        const startOffset = Number(entry.startOffset ?? entry.start)
        const endOffset = Number(entry.endOffset ?? entry.end)
        return {
          scope: scope === 'passage' || scope === 'questions' ? scope : 'unknown',
          text,
          kind: entry.kind === 'note' ? 'note' : 'highlight',
          questionId: entry.questionId ? String(entry.questionId).trim() : null,
          startOffset: Number.isFinite(startOffset) ? startOffset : null,
          endOffset: Number.isFinite(endOffset) ? endOffset : null,
          before: normalizeComparableText(entry.before),
          after: normalizeComparableText(entry.after),
          occurrence: Number.isFinite(Number(entry.occurrence)) ? Math.max(0, Number(entry.occurrence)) : 0,
          createdAt: entry.createdAt || new Date().toISOString()
        }
      })
      .filter(Boolean)
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
