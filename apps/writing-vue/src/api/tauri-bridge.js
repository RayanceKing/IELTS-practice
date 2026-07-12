/**
 * Tauri invoke bridge — product path is Tauri only.
 * Electron / Fastify / file:// fallbacks are not supported.
 */

export function isTauriRuntime() {
  return typeof window !== 'undefined' && !!(window.__TAURI_INTERNALS__ || window.__TAURI__)
}

export function assertTauriRuntime(label = 'command') {
  if (!isTauriRuntime()) {
    const err = new Error(
      `${label}: requires Tauri runtime (Electron/Fastify removed; use cargo tauri dev)`
    )
    err.code = 'tauri.required'
    throw err
  }
}

export async function invokeCommand(cmd, args = {}) {
  assertTauriRuntime(cmd)
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke(cmd, args)
  } catch (err) {
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
  if (response == null) {
    const err = new Error(`${label}: empty response`)
    err.code = 'tauri.empty'
    throw err
  }
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
