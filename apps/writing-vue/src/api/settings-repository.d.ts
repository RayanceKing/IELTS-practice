export interface AiProviderConfig {
  id: string
  configName: string
  provider: string
  baseUrl: string
  defaultModel: string
  isDefault: boolean
  isEnabled: boolean
  hasSecret: boolean
}

export interface UpsertAiProviderConfigCommand {
  id?: string
  configName: string
  provider: string
  baseUrl?: string
  defaultModel: string
  isEnabled?: boolean
  apiKey?: string
}

export interface AiProviderTestResult {
  provider: string
  model: string
  reachable: boolean
  authenticated: boolean
  latencyMs: number
}

export function listSettings(namespace?: string | null): Promise<{ source: 'tauri'; items: unknown[] }>
export function upsertSetting(namespace: string, key: string, value: unknown): Promise<unknown>
export function migrateLocalPreferences(prefs: unknown): Promise<{ source: 'tauri'; count: number }>
export function setSecret(name: string, secret: string): Promise<unknown>
export function listSecretRefs(): Promise<unknown[]>
export function listAiConfigs(): Promise<AiProviderConfig[]>
export function upsertAiConfig(cmd: UpsertAiProviderConfigCommand): Promise<AiProviderConfig>
export function deleteAiConfig(id: string): Promise<unknown>
export function setDefaultAiConfig(id: string): Promise<unknown>
export function testAiProvider(): Promise<AiProviderTestResult>
export function createBackup(appVersion?: string | null): Promise<unknown>
export function importBackupPath(path: string, dryRun?: boolean): Promise<unknown>

declare const settingsRepository: Record<string, unknown>
export default settingsRepository
