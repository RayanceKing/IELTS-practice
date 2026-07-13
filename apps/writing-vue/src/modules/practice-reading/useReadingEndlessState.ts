import { useTauriPreferences } from '@/composables/useTauriPreferences.js'

export const ENDLESS_STATE_KEY = 'practice_reading_endless_state_v1'

export type EndlessPoolEntry = {
  id: string
  title?: string
  category?: string
}

export type EndlessState = {
  active?: boolean
  startedAt?: string
  updatedAt?: string
  currentAssetId?: string
  pool?: EndlessPoolEntry[]
  [key: string]: unknown
}

function parseState(raw: unknown): EndlessState {
  if (!raw) return {}
  try {
    const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw
    return parsed && typeof parsed === 'object' ? (parsed as EndlessState) : {}
  } catch {
    return {}
  }
}

function takeSessionLegacy(): string {
  try {
    const raw = window.sessionStorage?.getItem(ENDLESS_STATE_KEY) || ''
    if (raw) window.sessionStorage?.removeItem(ENDLESS_STATE_KEY)
    return raw
  } catch {
    return ''
  }
}

/**
 * Endless-mode continuity across Library → Reading.
 * Durable store: Tauri SQLite settings (frontend-preferences).
 * One-shot migrates leftover sessionStorage values.
 */
export function useReadingEndlessState() {
  const preferences = useTauriPreferences()

  async function ensureReady() {
    await preferences.hydrate()
    if (!preferences.get(ENDLESS_STATE_KEY, '')) {
      const legacy = takeSessionLegacy()
      if (legacy) preferences.set(ENDLESS_STATE_KEY, legacy)
    }
  }

  function readEndlessState(): EndlessState {
    return parseState(preferences.get(ENDLESS_STATE_KEY, ''))
  }

  function writeEndlessState(patch: EndlessState = {}): EndlessState {
    const nextState: EndlessState = {
      ...readEndlessState(),
      ...patch,
      active: patch.active !== undefined ? Boolean(patch.active) : true,
      updatedAt: new Date().toISOString()
    }
    preferences.set(ENDLESS_STATE_KEY, JSON.stringify(nextState))
    return nextState
  }

  function clearEndlessState() {
    preferences.set(ENDLESS_STATE_KEY, '')
    try {
      window.sessionStorage?.removeItem(ENDLESS_STATE_KEY)
    } catch {
      // ignore
    }
  }

  return {
    ensureReady,
    readEndlessState,
    writeEndlessState,
    clearEndlessState
  }
}

export default useReadingEndlessState
