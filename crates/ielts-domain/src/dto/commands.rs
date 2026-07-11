//! Command request/response DTOs. Keep these thin; business rules live in domain/application.

use serde::{Deserialize, Serialize};

use crate::domain::{Activity, AttemptMode};
use crate::dto::{AttemptRecord, PracticeAssetV2, WritingEvaluationV4};
use crate::error::ErrorEnvelope;

#[cfg(feature = "ts-export")]
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-export", derive(TS), ts(export, export_to = "../../../apps/writing-vue/src/types/generated/"))]
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
#[cfg_attr(feature = "ts-export", derive(TS), ts(export, export_to = "../../../apps/writing-vue/src/types/generated/"))]
pub struct ListHistoryQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-export", derive(TS), ts(export, export_to = "../../../apps/writing-vue/src/types/generated/"))]
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
#[cfg_attr(feature = "ts-export", derive(TS), ts(export, export_to = "../../../apps/writing-vue/src/types/generated/"))]
pub struct SubmitAttemptCommand {
    pub attempt_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-export", derive(TS), ts(export, export_to = "../../../apps/writing-vue/src/types/generated/"))]
pub struct GetAttemptResponse {
    pub attempt: AttemptRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<WritingEvaluationV4>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<PracticeAssetV2>,
}
