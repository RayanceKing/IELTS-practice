<template>
  <div
    id="shui-three-bg"
    class="shui-gradient-bg"
    aria-hidden="true"
  ></div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useTauriPreferences } from '@/composables/useTauriPreferences.js'

const STORAGE_KEY = 'three_bg_theme'

const themes = {
  'misty-mountain': {
    label: 'misty-mountain',
    start: '#ffd89b',
    end: '#6accc7'
  },
  'teal-ocean': {
    label: 'teal-ocean',
    start: '#a7d8ff',
    end: '#42b9a8'
  },
  'floral-bloom': {
    label: 'floral-bloom',
    start: '#ffc4a3',
    end: '#87d8d0'
  }
}

const activeTheme = ref('misty-mountain')
const preferences = useTauriPreferences()

function normalizeThemeName(themeName) {
  return Object.prototype.hasOwnProperty.call(themes, themeName)
    ? themeName
    : 'misty-mountain'
}

function takeLocalLegacy() {
  try {
    const value = localStorage.getItem(STORAGE_KEY) || ''
    if (value) localStorage.removeItem(STORAGE_KEY)
    return value
  } catch (_) {
    return ''
  }
}

async function resolveStoredTheme() {
  await preferences.hydrate()
  let theme = preferences.get(STORAGE_KEY, '')
  if (!theme) {
    const legacy = takeLocalLegacy()
    if (legacy) {
      preferences.set(STORAGE_KEY, legacy)
      theme = legacy
    }
  }
  return normalizeThemeName(theme || 'misty-mountain')
}

function applyFallbackTheme(themeName, { persist = true } = {}) {
  const nextThemeName = normalizeThemeName(themeName)
  const theme = themes[nextThemeName]
  activeTheme.value = nextThemeName
  document.documentElement.style.setProperty('--shui-gradient-start', theme.start)
  document.documentElement.style.setProperty('--shui-gradient-end', theme.end)
  document.documentElement.dataset.shuiBgTheme = theme.label
  if (persist) {
    preferences.set(STORAGE_KEY, nextThemeName)
  }
}

function switchTheme(themeName) {
  applyFallbackTheme(themeName, { persist: true })
}

function handleThemeChange(event) {
  switchTheme(event?.detail?.theme || activeTheme.value)
}

function destroy() {
  window.removeEventListener('shui-bg-theme-change', handleThemeChange)
  document.body.classList.remove('hero-body', 'shui-gradient-active')
  delete document.documentElement.dataset.shuiBgTheme
  document.documentElement.style.removeProperty('--shui-gradient-start')
  document.documentElement.style.removeProperty('--shui-gradient-end')
}

onMounted(async () => {
  applyFallbackTheme(await resolveStoredTheme(), { persist: false })
  window.addEventListener('shui-bg-theme-change', handleThemeChange)
  document.body.classList.add('hero-body', 'shui-gradient-active')
})

onBeforeUnmount(destroy)
</script>

<style>
@property --body-gradient-angle {
  syntax: '<angle>';
  inherits: false;
  initial-value: 135deg;
}

#shui-three-bg {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  pointer-events: none;
  background:
    linear-gradient(
      var(--body-gradient-angle),
      var(--shui-gradient-start, #ffd89b) 0%,
      var(--shui-gradient-end, #6accc7) 100%
    );
  animation: bodyGradientRotation 120s ease-in-out infinite;
  transform: translateZ(0);
  backface-visibility: hidden;
}

@keyframes bodyGradientRotation {
  0% {
    --body-gradient-angle: 135deg;
  }

  50% {
    --body-gradient-angle: 225deg;
  }

  100% {
    --body-gradient-angle: 495deg;
  }
}

@media (prefers-reduced-motion: reduce) {
  #shui-three-bg {
    animation: none;
  }
}
</style>
