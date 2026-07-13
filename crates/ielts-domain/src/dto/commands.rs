//! Command request/response DTOs. Keep these thin; business rules live in domain/application.

use serde::{Deserialize, Serialize};

use crate::domain::{Activity, AttemptMode};
use crate::dto::{AttemptRecord, PracticeAssetV2, WritingEvaluationV4};
use crate::error::ErrorEnvelope;

#[cfg(feature = "ts-export")]
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct CommandResponse<T> {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorEnvelope>,
}

impl<T> CommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(error: ErrorEnvelope) -> CommandResponse<T> {
        CommandResponse {
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct ListHistoryQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default = "default_offset")]
    pub offset: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_score: Option<f64>,
}

fn default_limit() -> u32 {
    20
}

fn default_offset() -> u32 {
    0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct ListHistoryPage {
    pub items: Vec<crate::HistoryListItemVm>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct HistoryDetailResponse {
    pub summary: crate::HistoryListItemVm,
    pub attempt: AttemptRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<WritingEvaluationV4>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub enum HistoryExportFormat {
    Csv,
    Markdown,
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct ExportHistoryCommand {
    pub format: HistoryExportFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<ListHistoryQuery>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct ExportHistoryResult {
    pub format: HistoryExportFormat,
    pub body: String,
    pub record_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct SettingEntry {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct UpsertSettingCommand {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct SecretRef {
    /// Logical name, e.g. "writing.openai.api_key"
    pub name: String,
    /// Opaque OS keychain reference; never the secret itself.
    pub ref_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct SetSecretCommand {
    pub name: String,
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigDto {
    pub id: String,
    pub config_name: String,
    pub provider: String,
    pub base_url: String,
    pub default_model: String,
    pub is_default: bool,
    pub is_enabled: bool,
    pub has_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUpsertConfigCommand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub config_name: String,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub default_model: String,
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct BackupManifest {
    pub schema_version: u32,
    /// SQLite schema version the snapshot was created from. Legacy v1
    /// packages omit this field.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub database_schema_version: u32,
    pub created_at: String,
    pub app_version: String,
    pub includes_secrets: bool,
    pub attempt_count: u32,
    pub settings_count: u32,
    pub secret_ref_count: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub table_count: u32,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub row_count: u64,
    pub checksum_sha256: String,
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

/// Lossless representation of one SQLite value. Do not flatten values into
/// JSON scalars: doing so loses INTEGER/REAL/BLOB type information and makes a
/// database round-trip subtly lossy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub enum BackupSqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// A canonical SQLite table snapshot. Columns are stored explicitly so dry-run
/// can reject a package made for an incompatible schema before mutating data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct BackupTable {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<BackupSqlValue>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct BackupPackage {
    pub manifest: BackupManifest,
    pub attempts: Vec<AttemptRecord>,
    pub settings: Vec<SettingEntry>,
    /// Only refs; never plaintext secrets in ordinary backups.
    pub secret_refs: Vec<SecretRef>,
    /// Complete canonical database payload (schema v2+). Empty only for
    /// explicitly supported legacy v1 packages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub database: Vec<BackupTable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct ImportBackupReport {
    pub dry_run: bool,
    pub ok: bool,
    pub attempt_imported: u32,
    pub settings_imported: u32,
    pub secret_refs_imported: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub tables_imported: u32,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub rows_imported: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct SaveDraftCommand {
    pub attempt_id: String,
    pub activity: Activity,
    pub mode: AttemptMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_snapshot: Option<String>,
    /// Client-generated idempotency key.
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct SubmitAttemptCommand {
    pub attempt_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-export",
    derive(TS),
    ts(export, export_to = "../../../apps/writing-vue/src/types/generated/")
)]
pub struct GetAttemptResponse {
    pub attempt: AttemptRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<WritingEvaluationV4>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<PracticeAssetV2>,
}
