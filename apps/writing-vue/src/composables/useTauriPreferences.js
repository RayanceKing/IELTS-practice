import { onMounted, ref } from 'vue'
import { listSettings, upsertSetting } from '@/api/settings-repository.js'

// Small synchronous cache over the async SQLite settings repository. Callers can
// keep their existing setup-time defaults while the persisted values hydrate.
const cache = new Map()
let hydrated = false
let hydrationPromise

async function hydrate() {
  if (hydrated) return
  if (!hydrationPromise) {
    hydrationPromise = listSettings('frontend-preferences')
      .then(({ items }) => {
        for (const item of items || []) {
          if (item?.key) cache.set(item.key, item.value)
        }
        hydrated = true
      })
      .catch(() => {
        hydrated = true
      })
  }
  await hydrationPromise
}

export function useTauriPreferences() {
  const ready = ref(hydrated)

  onMounted(async () => {
    await hydrate()
    ready.value = true
  })

  function get(key, fallback = '') {
    return cache.has(key) ? cache.get(key) : fallback
  }

  function set(key, value) {
    cache.set(key, value)
    void upsertSetting('frontend-preferences', key, value).catch(() => {})
  }

  return { ready, get, set, hydrate }
}

export default useTauriPreferences
