import { settings } from '@/api/client.js'
import { practiceAssets, practiceCoach, practiceHistory, practiceReadingSuite, practiceSessions } from '@/api/practice-client.js'
import {
  READING_ACTIVITY,
  READING_COACH_ENABLED_SETTING_KEY,
  normalizeReadingCoachEnabled,
  normalizeReadingRecordId,
  normalizeReadingSuitePayload
} from './contracts'

interface RefreshOptions { refresh?: boolean }
interface CoachOptions { onEvent?: (event: unknown) => void }

export const readingLibraryApi = {
  async listAssets(options: RefreshOptions = {}) {
    return practiceAssets.listAll({ activity: READING_ACTIVITY }, {
      refresh: Boolean(options.refresh)
    })
  },

  async getAsset(assetId: string, options: RefreshOptions = {}) {
    return practiceAssets.get(READING_ACTIVITY, assetId, options)
  }
}

export const readingHistoryApi = {
  async listAll() {
    return practiceHistory.listAll({ activity: READING_ACTIVITY })
  },

  async delete(recordId: string) {
    return practiceHistory.delete(READING_ACTIVITY, normalizeReadingRecordId(recordId))
  },

  async clear() {
    return practiceHistory.clear({ activity: READING_ACTIVITY })
  },

  async exportArchive() {
    return practiceHistory.exportArchive({ activity: READING_ACTIVITY })
  },

  async importArchive(payload: unknown) {
    return practiceHistory.importArchive(READING_ACTIVITY, payload)
  }
}

export const readingSuiteApi = {
  async create(payload = {}) {
    return practiceReadingSuite.create(normalizeReadingSuitePayload(payload))
  }
}

export const readingCoachApi = {
  async query(payload: Record<string, unknown>, sessionId: string | null, options: CoachOptions = {}) {
    const queryCoach = practiceCoach.query as (
      activity: string,
      request: Record<string, unknown>,
      attemptId: string | null,
      requestOptions: CoachOptions
    ) => Promise<unknown>
    return queryCoach(READING_ACTIVITY, payload, sessionId, options)
  }
}

export const readingCoachSettingsApi = {
  async getEnabled() {
    const value = await settings.get(READING_COACH_ENABLED_SETTING_KEY)
    return normalizeReadingCoachEnabled(value, true)
  },

  async updateEnabled(enabled: unknown) {
    const normalized = normalizeReadingCoachEnabled(enabled, true)
    await settings.update({
      [READING_COACH_ENABLED_SETTING_KEY]: normalized
    })
    return normalized
  }
}

export const readingSessionApi = {
  async getState(sessionId: string) {
    return practiceSessions.getState(READING_ACTIVITY, normalizeReadingRecordId(sessionId))
  }
}
