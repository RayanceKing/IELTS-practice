/**
 * Thin Tauri invoke bridge.
 * When not running inside Tauri, all helpers return null so callers fall back.
 */

export function isTauriRuntime() {
  return typeof window !== 'undefined' && !!(window.__TAURI_INTERNALS__ || window.__TAURI__)
}

export async function invokeCommand(cmd, args = {}) {
  if (!isTauriRuntime()) return null
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke(cmd, args)
  } catch (err) {
    // Fallback for environments that inject invoke without the npm package.
    if (typeof window !== 'undefined' && window.__TAURI__?.core?.invoke) {
      return window.__TAURI__.core.invoke(cmd, args)
    }
    throw err
  }
}

/**
 * Unwrap CommandResponse envelope from Rust commands.
 */
export function unwrapCommandResponse(response, label = 'command') {
  if (response == null) return null
  if (typeof response === 'object' && 'ok' in response) {
    if (response.ok) return response.data
    const message = response.error?.message || `${label} failed`
    const error = new Error(message)
    error.code = response.error?.code
    error.retryable = !!response.error?.retryable
    throw error
  }
  return response
}
