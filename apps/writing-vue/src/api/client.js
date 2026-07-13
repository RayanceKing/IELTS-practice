/**
 * Product API facade — Tauri only.
 * Electron / Fastify / file:// paths removed.
 */

import {
  listHistory,
  listHistoryAll,
  getHistoryDetail,
  exportHistory,
  deleteHistoryAttempt
} from '@/api/history-repository.js'
import {
  saveDraft,
  submitAttempt,
  startEvaluation,
  listEvaluationEvents,
  cancelEvaluation,
  getEvaluationForAttempt,
  newIdempotencyKey
} from '@/api/writing-repository.js'
import {
  listSettings,
  upsertSetting,
  listAiConfigs,
  upsertAiConfig,
  deleteAiConfig,
  setDefaultAiConfig,
  testAiProvider
} from '@/api/settings-repository.js'
import { isTauriRuntime } from '@/api/tauri-bridge.js'
import { adaptWritingHistoryDetail } from '@/utils/evaluation-result.js'

const ERROR_MESSAGES = {
  invalid_api_key: 'API 密钥无效，请前往设置页面检查配置',
  insufficient_quota: 'API 余额不足，请充值后重试',
  rate_limit_exceeded: '请求频率超限，请稍后重试',
  rate_limited: '请求频率超限，请稍后重试',
  model_not_found: '模型不存在，请检查模型名称配置',
  timeout: '请求超时，请检查网络连接或稍后重试',
  network_error: '网络连接失败，请检查网络设置',
  server_error: '服务异常，请稍后重试',
  invalid_response_format: '评分数据解析失败，请点击"重试"按钮',
  start_failed: '启动评测失败，请重试',
  'ai.not_configured': '未配置 AI：请先在设置中添加并启用默认模型与 API Key。',
  not_implemented: '该功能尚未迁移到 Tauri 原生路径',
  tauri_required: '需要 Tauri 运行时（Electron/Fastify 已移除）',
  unknown_error: '未知错误，请重试'
}

const evaluationListeners = new Map()
let listenerSequence = 0
const activePolls = new Map()

export function getErrorMessage(code, fallbackMessage = '') {
  const mapped = ERROR_MESSAGES[code]
  if (mapped) return mapped
  const message = typeof fallbackMessage === 'string' ? fallbackMessage.trim() : ''
  if (message) return message
  return ERROR_MESSAGES.unknown_error
}

/** Prefer backend Chinese message (e.g. startEvaluation) over bare error code. */
export function resolveApiErrorMessage(error, fallbackCode = 'unknown_error') {
  const code = error?.code || fallbackCode
  // Known product codes win over sparse/technical messages so UI stays consistent.
  if (code && ERROR_MESSAGES[code] && (code === 'ai.not_configured' || !String(error?.message || '').trim())) {
    return ERROR_MESSAGES[code]
  }
  const message = typeof error?.message === 'string' ? error.message.trim() : ''
  if (message) return message
  return getErrorMessage(code)
}

export function isAPIAvailable() {
  return isTauriRuntime()
}

function createAiNotConfiguredError() {
  const error = new Error(ERROR_MESSAGES['ai.not_configured'])
  error.code = 'ai.not_configured'
  error.retryable = false
  return error
}

/**
 * Fail closed before draft/submit so unconfigured AI never leaves orphan submitted attempts.
 * Mirrors writing_start_evaluation: unconfigured → refuse; deterministic offline OK; else need default+key.
 */
async function assertAiConfiguredForWritingEvaluation() {
  const list = await configs.list()
  const defaultConfig = list.find((item) => item.is_default) || null
  if (defaultConfig?.is_enabled && defaultConfig?.has_secret) {
    const provider = String(defaultConfig.provider || '').trim().toLowerCase()
    if (provider && provider !== 'unconfigured') return
  }

  // Explicit offline scorer path (runtime provider only; not the product default).
  const { items } = await listSettings('ai')
  const providerEntry = (items || []).find((item) => item.key === 'provider')
  let provider = providerEntry?.value
  if (typeof provider === 'string') {
    try {
      const parsed = JSON.parse(provider)
      if (typeof parsed === 'string') provider = parsed
    } catch {
      // keep raw string
    }
  }
  if (String(provider || '').trim().toLowerCase() === 'deterministic') return

  throw createAiNotConfiguredError()
}

function notImplemented(feature) {
  const error = new Error(`${feature}: ${ERROR_MESSAGES.not_implemented}`)
  error.code = 'not_implemented'
  throw error
}

function newId(prefix = 'id') {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

async function readKvList(namespace) {
  const { items } = await listSettings(namespace)
  return (items || []).map((item) => {
    const raw = item.value ?? item
    if (typeof raw === 'string') {
      try {
        return JSON.parse(raw)
      } catch {
        return { key: item.key, value: raw }
      }
    }
    return raw && typeof raw === 'object' ? raw : { key: item.key, value: raw }
  })
}

async function writeKv(namespace, key, value) {
  await upsertSetting(namespace, key, value)
  return value
}

function normalizeAiConfig(item) {
  return {
    id: item.id,
    config_name: item.configName,
    provider: item.provider,
    base_url: item.baseUrl,
    default_model: item.defaultModel,
    is_default: !!item.isDefault,
    is_enabled: !!item.isEnabled,
    has_secret: !!item.hasSecret
  }
}

function toAiConfigCommand(data, id = null) {
  const cmd = {
    configName: data.config_name,
    provider: data.provider,
    baseUrl: data.base_url,
    defaultModel: data.default_model,
    isEnabled: data.is_enabled ?? data.enabled ?? true
  }
  if (id) cmd.id = id
  if (data.api_key) cmd.apiKey = data.api_key
  return cmd
}

export const configs = {
  async list() {
    return (await listAiConfigs()).map(normalizeAiConfig)
  },

  async getDefault() {
    const list = await this.list()
    return list.find((item) => item.is_default) || list[0] || null
  },

  async create(data) {
    const created = normalizeAiConfig(await upsertAiConfig(toAiConfigCommand(data)))
    if (data.is_default) await setDefaultAiConfig(created.id)
    return created
  },

  async update(id, updates) {
    const all = await this.list()
    const prev = all.find((item) => item.id === id)
    if (!prev) {
      const err = new Error(`config not found: ${id}`)
      err.code = 'not_found'
      throw err
    }
    const next = { ...prev, ...updates, id }
    const updated = normalizeAiConfig(await upsertAiConfig(toAiConfigCommand(next, id)))
    if (updates.is_default) await setDefaultAiConfig(id)
    return updated
  },

  async delete(id) {
    return deleteAiConfig(id)
  },

  async setDefault(id) {
    return setDefaultAiConfig(id)
  },

  async toggleEnabled(id) {
    const all = await this.list()
    const prev = all.find((item) => item.id === id)
    if (!prev) {
      const err = new Error(`config not found: ${id}`)
      err.code = 'not_found'
      throw err
    }
    return this.update(id, { is_enabled: !prev.is_enabled })
  },

  async test() {
    return testAiProvider()
  }
}

export const prompts = {
  async getActive(taskType) {
    const all = await this.listAll(taskType)
    return all.find((p) => p.active) || all[0] || null
  },

  async import(jsonData) {
    const list = Array.isArray(jsonData) ? jsonData : (jsonData?.prompts || [jsonData])
    const saved = []
    for (const item of list) {
      const id = item.id || newId('prompt')
      const entry = { ...item, id }
      await writeKv('prompts', id, entry)
      saved.push(entry)
    }
    return { imported: saved.length, items: saved }
  },

  async exportActive() {
    const all = await this.listAll()
    return { prompts: all.filter((p) => p.active) }
  },

  async listAll(taskType = null) {
    const items = (await readKvList('prompts')).filter((item) => item && item.id)
    if (!taskType) return items
    return items.filter(
      (item) => !item.taskType || item.taskType === taskType || item.task_type === taskType
    )
  },

  async activate(id) {
    const all = await this.listAll()
    for (const item of all) {
      const active = item.id === id
      if (!!item.active !== active) {
        await writeKv('prompts', item.id, { ...item, active })
      }
    }
    return true
  },

  async delete(id) {
    await deleteKv('prompts', id)
    return true
  }
}

function emitEvaluationEvent(event) {
  evaluationListeners.forEach((listener) => {
    try {
      listener(event)
    } catch (error) {
      console.warn('写作评测事件监听器执行失败:', error)
    }
  })
}

function mapEventToUi(raw) {
  const eventType = raw.eventType || raw.event_type || raw.type || 'log'
  const payload = raw.payload || raw.data || {}
  const typeMap = {
    stage: 'stage',
    completed: 'complete',
    complete: 'complete',
    error: 'error',
    failed: 'error',
    log: 'log'
  }
  const type = typeMap[eventType] || eventType
  const data =
    typeof payload === 'object' && payload !== null
      ? { ...payload }
      : { message: String(payload || '') }
  if (raw.stage && !data.stage) {
    data.stage = raw.stage
    data.key = typeof raw.stage === 'string' ? raw.stage.toLowerCase() : raw.stage
  }
  if (type === 'complete' && !data.score && data.evaluation?.score) {
    Object.assign(data, data.evaluation)
  }
  return {
    type,
    sessionId: raw.sessionId || raw.evaluationId || raw.evaluation_id,
    sequence: raw.sequence,
    data
  }
}

async function pollEvaluationEvents(attemptId, evaluationId) {
  if (!evaluationId || activePolls.has(evaluationId)) return
  let after = 0
  let stopped = false
  activePolls.set(evaluationId, () => {
    stopped = true
  })

  const tick = async () => {
    if (stopped) return
    try {
      const events = await listEvaluationEvents(evaluationId, after)
      for (const raw of events || []) {
        after = Math.max(after, Number(raw.sequence || 0))
        emitEvaluationEvent(mapEventToUi({ ...raw, sessionId: attemptId }))
        const t = String(raw.eventType || raw.event_type || '').toLowerCase()
        if (t === 'completed' || t === 'complete' || t === 'error' || t === 'failed') {
          stopped = true
        }
      }
      if (!stopped) {
        const evaluation = await getEvaluationForAttempt(attemptId)
        const status = String(
          evaluation?.evaluation?.status || evaluation?.status || ''
        ).toLowerCase()
        if (status === 'completed' || status === 'failed' || status === 'cancelled') {
          if (status === 'completed' && evaluation?.evaluation) {
            emitEvaluationEvent({
              type: 'complete',
              sessionId: attemptId,
              data: evaluation.evaluation
            })
          }
          stopped = true
        }
      }
    } catch (err) {
      console.warn('poll evaluation events failed', err)
    }
    if (!stopped) {
      setTimeout(tick, 1000)
    } else {
      activePolls.delete(evaluationId)
    }
  }
  void tick()
}

export const evaluate = {
  async start(payload) {
    const attemptId = payload.sessionId || payload.attemptId || newId('attempt')
    const content = payload.content || payload.contentText || ''
    const promptSnapshot =
      payload.topic_text || payload.topicText || payload.promptSnapshot || null
    const taskType = payload.task_type || payload.taskType || null

    // Gate before any durable attempt mutation — avoids submitted orphans when AI is missing.
    await assertAiConfiguredForWritingEvaluation()

    await saveDraft({
      attemptId,
      mode: payload.mode || 'bank',
      assetId:
        payload.topic_id != null
          ? String(payload.topic_id)
          : payload.assetId || null,
      contentText: content,
      promptSnapshot,
      idempotencyKey: newIdempotencyKey('draft')
    })
    await submitAttempt(attemptId, newIdempotencyKey('submit'))
    const { result } = await startEvaluation({
      attemptId,
      taskType,
      idempotencyKey: newIdempotencyKey('eval'),
      retryOf: payload.retryOf || null
    })

    const evaluationId =
      result?.session?.evaluationId ||
      result?.session?.evaluation_id ||
      result?.evaluation?.id ||
      null

    const events = result?.events || []
    for (const raw of events) {
      emitEvaluationEvent(mapEventToUi({ ...raw, sessionId: attemptId }))
    }
    if (evaluationId) {
      void pollEvaluationEvents(attemptId, evaluationId)
    } else if (result?.evaluation) {
      emitEvaluationEvent({
        type: 'complete',
        sessionId: attemptId,
        data: result.evaluation
      })
    }

    return {
      sessionId: attemptId,
      evaluationId,
      result
    }
  },

  async cancel(sessionId) {
    try {
      const { evaluation } = await getEvaluationForAttempt(sessionId)
      const evaluationId = evaluation?.id
      if (evaluationId) {
        const stop = activePolls.get(evaluationId)
        if (stop) stop()
        await cancelEvaluation(evaluationId)
      }
    } catch (_) {
      // ignore
    }
    return { cancelled: true, sessionId }
  },

  async getSessionState(sessionId) {
    const { evaluation } = await getEvaluationForAttempt(sessionId)
    let events = []
    if (evaluation?.id) {
      const rawEvents = await listEvaluationEvents(evaluation.id, 0)
      events = (rawEvents || []).map((raw) => mapEventToUi({ ...raw, sessionId }))
      void pollEvaluationEvents(sessionId, evaluation.id)
    }
    return {
      sessionId,
      evaluation,
      events,
      status: evaluation?.status || 'unknown'
    }
  },

  onEvent(callback) {
    if (typeof callback !== 'function') return null
    listenerSequence += 1
    const listenerId = `writing_eval_listener_${listenerSequence}`
    evaluationListeners.set(listenerId, callback)
    return listenerId
  },

  removeEventListener(listenerId) {
    if (!listenerId) return
    evaluationListeners.delete(listenerId)
  }
}

export const topics = {
  async list(filters = {}, pagination = { page: 1, limit: 20 }) {
    let items = (await readKvList('topics')).filter((item) => item && (item.id || item.title))
    if (filters.task_type || filters.taskType) {
      const t = filters.task_type || filters.taskType
      items = items.filter((item) => (item.task_type || item.taskType) === t)
    }
    if (filters.search) {
      const q = String(filters.search).toLowerCase()
      items = items.filter((item) =>
        String(item.title || item.prompt || '').toLowerCase().includes(q)
      )
    }
    const page = Number(pagination.page || 1)
    const limit = Number(pagination.limit || 20)
    const offset = (page - 1) * limit
    const slice = items.slice(offset, offset + limit)
    return { data: slice, total: items.length, page, limit }
  },

  async getById(id) {
    const { data } = await this.list({}, { page: 1, limit: 10000 })
    return data.find((item) => String(item.id) === String(id)) || null
  },

  async create(topicData) {
    const id = topicData.id || newId('topic')
    const entry = { ...topicData, id }
    await writeKv('topics', String(id), entry)
    return entry
  },

  async update(id, updates) {
    const prev = await this.getById(id)
    if (!prev) {
      const err = new Error(`topic not found: ${id}`)
      err.code = 'not_found'
      throw err
    }
    const next = { ...prev, ...updates, id }
    await writeKv('topics', String(id), next)
    return next
  },

  async delete(id) {
    await deleteKv('topics', String(id))
    return true
  },

  async batchImport(topicsArray) {
    const list = Array.isArray(topicsArray) ? topicsArray : []
    let count = 0
    for (const item of list) {
      await this.create(item)
      count += 1
    }
    return { imported: count }
  },

  async getStatistics() {
    const { data } = await this.list({}, { page: 1, limit: 10000 })
    const byTask = {}
    for (const item of data) {
      const t = item.task_type || item.taskType || 'unknown'
      byTask[t] = (byTask[t] || 0) + 1
    }
    return { total: data.length, byTask }
  }
}

function mapHistoryItemToEssay(item) {
  return {
    id: item.id,
    task_type: item.task_type || item.taskType || 'task2',
    topic_title: item.display_topic_title || item.topic_title || item.title || 'Untitled',
    content: item.content_text || '',
    total_score: item.total_score ?? item.score_value ?? 0,
    submitted_at: item.submitted_at || item.submittedAt || '',
    duration: item.duration ?? Math.round((item.duration_ms || 0) / 1000),
    status: item.status,
    source: 'tauri'
  }
}

export const essays = {
  async list(filters = {}, pagination = { page: 1, limit: 20 }) {
    const page = Number(pagination.page || 1)
    const limit = Number(pagination.limit || 20)
    const offset = (page - 1) * limit
    const result = await listHistory({
      activity: 'writing',
      limit,
      offset,
      search: filters.search || null,
      startDate: filters.startDate || filters.start_date || null,
      endDate: filters.endDate || filters.end_date || null,
      minScore: filters.minScore ?? filters.min_score ?? null,
      maxScore: filters.maxScore ?? filters.max_score ?? null
    })
    return {
      data: (result.items || []).map(mapHistoryItemToEssay),
      total: result.total,
      page,
      limit
    }
  },

  async getById(id) {
    const { detail } = await getHistoryDetail(id)
    if (!detail) return null
    // Shared V4 → UI adapter used by Result + History
    const adapted = adaptWritingHistoryDetail(detail)
    if (!adapted) return null
    return {
      ...adapted,
      id: adapted.id || id,
      source: 'tauri'
    }
  },

  async create() {
    notImplemented('essays.create (use evaluate.start / writing_save_draft)')
  },

  async delete(id) {
    await deleteHistoryAttempt(id)
    return true
  },

  async batchDelete(ids) {
    const list = Array.isArray(ids) ? ids : []
    for (const id of list) {
      await deleteHistoryAttempt(id)
    }
    return { deleted: list.length }
  },

  async deleteAll() {
    const result = await listHistoryAll({ activity: 'writing' })
    for (const item of result.items || []) {
      await deleteHistoryAttempt(item.id)
    }
    return { deleted: (result.items || []).length }
  },

  async getStatistics() {
    const result = await listHistoryAll({ activity: 'writing' })
    const items = result.items || []
    const scores = items
      .map((i) => Number(i.score_value ?? i.total_score ?? 0))
      .filter((n) => n > 0)
    const avg = scores.length ? scores.reduce((a, b) => a + b, 0) / scores.length : 0
    return {
      total: items.length,
      averageScore: Math.round(avg * 10) / 10,
      scored: scores.length
    }
  },

  async exportCSV(filters = {}) {
    const { result } = await exportHistory('csv', { activity: 'writing', ...filters })
    // Always return the CSV text body — callers historically did String(result)
    // which becomes "[object Object]" when given the full ExportHistoryResult.
    if (typeof result === 'string') return result
    return result?.body ?? ''
  }
}

export const settings = {
  async getAll() {
    const { items } = await listSettings('app')
    const out = {}
    for (const item of items || []) {
      const key = item.key
      let value = item.value
      if (typeof value === 'string') {
        try {
          value = JSON.parse(value)
        } catch {
          // keep string
        }
      }
      out[key] = value
    }
    return out
  },

  async get(key) {
    const all = await this.getAll()
    return all[key]
  },

  async update(updates) {
    const entries = Object.entries(updates || {})
    for (const [key, value] of entries) {
      await upsertSetting('app', key, value)
    }
    return true
  },

  async reset() {
    const all = await this.getAll()
    for (const key of Object.keys(all)) {
      await upsertSetting('app', key, null)
    }
    return true
  }
}

export const upload = {
  async uploadImage() {
    notImplemented('upload.uploadImage')
  },
  async deleteImage() {
    notImplemented('upload.deleteImage')
  },
  async getImagePath() {
    notImplemented('upload.getImagePath')
  }
}

export async function request() {
  notImplemented('request (Fastify HTTP removed)')
}

export async function requestEventStream() {
  notImplemented('requestEventStream (Fastify SSE removed)')
}

export default {
  configs,
  prompts,
  evaluate,
  topics,
  essays,
  settings,
  upload,
  getErrorMessage,
  resolveApiErrorMessage,
  isAPIAvailable
}
