import { computed, nextTick, ref, watch, type Ref } from 'vue'
import { useTauriPreferences } from '@/composables/useTauriPreferences.js'

const NOTES_STORAGE_PREFIX = 'practice_reading_notes_'
const SUITE_AUTO_ADVANCE_STORAGE_KEY = 'suite_auto_advance_after_submit'
const FONT_KEY = 'reading_font_size'
const THEME_KEY = 'reading_theme_mode'

export const readingFontSizeOptions = [
  { value: 'normal', label: 'A' },
  { value: 'large', label: 'A', style: { fontSize: '1.1rem' } },
  { value: 'xlarge', label: 'A', style: { fontSize: '1.25rem' } }
] as const

export const readingThemeModeOptions = [
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' }
] as const

export type ReadingFontSize = (typeof readingFontSizeOptions)[number]['value']
export type ReadingThemeMode = (typeof readingThemeModeOptions)[number]['value']

type AssetLike = { id?: string | null } | null | undefined

type ReadingUiPreferencesOptions = {
  assetSource: () => AssetLike
  onThemeChanged?: () => void
}

function takeLocal(key: string): string | null {
  try {
    const value = window.localStorage?.getItem(key) ?? null
    if (value != null) window.localStorage?.removeItem(key)
    return value
  } catch {
    return null
  }
}

function isFontSize(value: string | null | undefined): value is ReadingFontSize {
  return readingFontSizeOptions.some((option) => option.value === value)
}

function isThemeMode(value: string | null | undefined): value is ReadingThemeMode {
  return readingThemeModeOptions.some((option) => option.value === value)
}

/**
 * Reading chrome preferences — Tauri SQLite settings only.
 * One-shot migrates leftover localStorage keys into frontend-preferences.
 */
export function useReadingUiPreferences(options: ReadingUiPreferencesOptions) {
  const preferences = useTauriPreferences()
  const settingsPanelOpen = ref(false)
  const notesPanelOpen = ref(false)
  const notesTextarea = ref<HTMLTextAreaElement | null>(null)
  const notesText = ref('')
  const readingFontSize = ref<ReadingFontSize>('normal')
  const readingThemeMode = ref<ReadingThemeMode>('light')
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

  function notesKey(assetId: string) {
    return `${NOTES_STORAGE_PREFIX}${assetId}`
  }

  function migrateLocalIfMissing(key: string): string {
    const current = preferences.get(key, '')
    if (current) return current
    const legacy = takeLocal(key)
    if (legacy != null && legacy !== '') {
      preferences.set(key, legacy)
      return legacy
    }
    return current
  }

  async function initializeReadingPreferences() {
    await preferences.hydrate()
    const storedFont = migrateLocalIfMissing(FONT_KEY)
    if (isFontSize(storedFont)) readingFontSize.value = storedFont
    const storedTheme = migrateLocalIfMissing(THEME_KEY)
    if (isThemeMode(storedTheme)) readingThemeMode.value = storedTheme
    const storedSuiteFlow = migrateLocalIfMissing(SUITE_AUTO_ADVANCE_STORAGE_KEY)
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
    if (nextVisible) void nextTick(() => notesTextarea.value?.focus?.())
  }

  function selectReadingFont(value: string) {
    if (!isFontSize(value)) return
    readingFontSize.value = value
    preferences.set(FONT_KEY, value)
  }

  function selectReadingTheme(value: string) {
    if (!isThemeMode(value)) return
    readingThemeMode.value = value
    options.onThemeChanged?.()
    void nextTick(() => options.onThemeChanged?.())
    preferences.set(THEME_KEY, value)
  }

  function setSuiteAutoAdvance(value: unknown) {
    suiteAutoAdvance.value = Boolean(value)
    preferences.set(SUITE_AUTO_ADVANCE_STORAGE_KEY, String(suiteAutoAdvance.value))
  }

  function loadReadingNotes() {
    suppressNotesPersist = true
    const assetId = options.assetSource()?.id
    if (!assetId) {
      notesText.value = ''
      suppressNotesPersist = false
      return
    }
    const key = notesKey(String(assetId))
    const stored = migrateLocalIfMissing(key)
    notesText.value = stored || ''
    suppressNotesPersist = false
  }

  function clearReadingNotesDraft() {
    suppressNotesPersist = true
    notesText.value = ''
    suppressNotesPersist = false
  }

  watch(notesText, () => {
    if (suppressNotesPersist) return
    const assetId = options.assetSource()?.id
    if (assetId) preferences.set(notesKey(String(assetId)), notesText.value || '')
  })

  return {
    settingsPanelOpen,
    notesPanelOpen,
    notesTextarea: notesTextarea as Ref<HTMLTextAreaElement | null>,
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

export default useReadingUiPreferences
