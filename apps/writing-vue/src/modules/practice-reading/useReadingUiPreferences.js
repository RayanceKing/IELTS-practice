import { computed, nextTick, ref, watch } from 'vue'

const NOTES_STORAGE_PREFIX = 'practice_reading_notes_'
const SUITE_AUTO_ADVANCE_STORAGE_KEY = 'suite_auto_advance_after_submit'

export const readingFontSizeOptions = [
  { value: 'normal', label: 'A' },
  { value: 'large', label: 'A', style: { fontSize: '1.1rem' } },
  { value: 'xlarge', label: 'A', style: { fontSize: '1.25rem' } }
]

export const readingThemeModeOptions = [
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' }
]

function readStorage(key) {
  try {
    return window.localStorage?.getItem(key) ?? null
  } catch (_) {
    return null
  }
}

function writeStorage(key, value) {
  try {
    window.localStorage?.setItem(key, value)
  } catch (_) {}
}

export function useReadingUiPreferences({ assetSource, onThemeChanged }) {
  const settingsPanelOpen = ref(false)
  const notesPanelOpen = ref(false)
  const notesTextarea = ref(null)
  const notesText = ref('')
  const readingFontSize = ref('normal')
  const readingThemeMode = ref('light')
  const suiteAutoAdvance = ref(true)
  let suppressNotesPersist = false

  const readingPageClassList = computed(() => ({
    [`font-${readingFontSize.value}`]: true,
    'dark-mode': readingThemeMode.value === 'dark'
  }))
  const readingPageStyle = computed(() => ({
    '--reading-font-scale': readingFontSize.value === 'xlarge'
      ? '1.18'
      : (readingFontSize.value === 'large' ? '1.08' : '1')
  }))

  function initializeReadingPreferences() {
    const storedFont = readStorage('reading_font_size')
    if (readingFontSizeOptions.some((option) => option.value === storedFont)) {
      readingFontSize.value = storedFont
    }
    const storedTheme = readStorage('reading_theme_mode')
    if (readingThemeModeOptions.some((option) => option.value === storedTheme)) {
      readingThemeMode.value = storedTheme
    }
    const storedSuiteFlow = readStorage(SUITE_AUTO_ADVANCE_STORAGE_KEY)
    if (storedSuiteFlow === 'true' || storedSuiteFlow === 'false') {
      suiteAutoAdvance.value = storedSuiteFlow === 'true'
    }
  }

  function closeFloatingPanels() {
    settingsPanelOpen.value = false
    notesPanelOpen.value = false
  }

  function toggleSettingsPanel() {
    const nextVisible = !settingsPanelOpen.value
    closeFloatingPanels()
    settingsPanelOpen.value = nextVisible
  }

  function toggleNotesPanel() {
    const nextVisible = !notesPanelOpen.value
    closeFloatingPanels()
    notesPanelOpen.value = nextVisible
    if (nextVisible) nextTick(() => notesTextarea.value?.focus?.())
  }

  function selectReadingFont(value) {
    if (!readingFontSizeOptions.some((option) => option.value === value)) return
    readingFontSize.value = value
    writeStorage('reading_font_size', value)
  }

  function selectReadingTheme(value) {
    if (!readingThemeModeOptions.some((option) => option.value === value)) return
    readingThemeMode.value = value
    onThemeChanged?.()
    nextTick(() => onThemeChanged?.())
    writeStorage('reading_theme_mode', value)
  }

  function setSuiteAutoAdvance(value) {
    suiteAutoAdvance.value = Boolean(value)
    writeStorage(SUITE_AUTO_ADVANCE_STORAGE_KEY, String(suiteAutoAdvance.value))
  }

  function loadReadingNotes() {
    suppressNotesPersist = true
    const assetId = assetSource()?.id
    notesText.value = assetId ? (readStorage(`${NOTES_STORAGE_PREFIX}${assetId}`) || '') : ''
    suppressNotesPersist = false
  }

  function clearReadingNotesDraft() {
    suppressNotesPersist = true
    notesText.value = ''
    suppressNotesPersist = false
  }

  watch(notesText, () => {
    if (suppressNotesPersist) return
    const assetId = assetSource()?.id
    if (assetId) writeStorage(`${NOTES_STORAGE_PREFIX}${assetId}`, notesText.value || '')
  })

  return {
    settingsPanelOpen,
    notesPanelOpen,
    notesTextarea,
    notesText,
    readingFontSize,
    readingThemeMode,
    suiteAutoAdvance,
    readingPageClassList,
    readingPageStyle,
    initializeReadingPreferences,
    toggleSettingsPanel,
    toggleNotesPanel,
    closeFloatingPanels,
    selectReadingFont,
    selectReadingTheme,
    setSuiteAutoAdvance,
    loadReadingNotes,
    clearReadingNotesDraft
  }
}
