use ielts_application::{
    AgentLimits, AgentRunOutcome, AgentService, ApplicationError, RunAgentCommand,
};
use ielts_domain::dto::CommandResponse;
use ielts_domain::ErrorEnvelope;
use serde::Deserialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::agent::{WorkspaceFileTools, WorkspaceGrant, WorkspaceGrants};
use crate::ai::{load_runtime, load_runtime_for_config};
use crate::app::application_store::ApplicationStore;
use crate::app::state::{AppDb, AppVault};

const AGENT_SYSTEM_PROMPT: &str = "You are IELTS Atlas's local workspace assistant. Use only the provided tools, inspect a file before modifying an existing file, preserve unrelated content, and report exactly what changed. Never invent tool results or claim access outside the granted workspace.";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkspaceAgentRequest {
    pub grant_id: String,
    pub prompt: String,
    #[serde(default)]
    pub config_id: Option<String>,
}

#[tauri::command]
pub fn agent_pick_workspace(
    app: tauri::AppHandle,
    grants: State<'_, WorkspaceGrants>,
) -> CommandResponse<Option<WorkspaceGrant>> {
    let folder = app.dialog().file().blocking_pick_folder();
    let Some(folder) = folder else {
        return CommandResponse::success(None);
    };
    let path = match folder.into_path() {
        Ok(path) => path,
        Err(error) => return CommandResponse::failure(path_error(error.to_string())),
    };
    match grants.issue(&path) {
        Ok(grant) => CommandResponse::success(Some(grant)),
        Err(error) => CommandResponse::failure(path_error(error)),
    }
}

#[tauri::command]
pub async fn agent_run(
    db: State<'_, AppDb>,
    vault: State<'_, AppVault>,
    grants: State<'_, WorkspaceGrants>,
    request: RunWorkspaceAgentRequest,
) -> Result<CommandResponse<AgentRunOutcome>, ErrorEnvelope> {
    if request.prompt.trim().is_empty() {
        return Ok(CommandResponse::failure(ErrorEnvelope::new(
            "agent.invalid_request",
            "agent prompt is required",
            false,
        )));
    }
    let root = match grants.resolve(&request.grant_id) {
        Ok(root) => root,
        Err(error) => return Ok(CommandResponse::failure(path_error(error))),
    };
    let tools = match WorkspaceFileTools::new(root) {
        Ok(tools) => tools,
        Err(error) => return Ok(CommandResponse::failure(path_error(error))),
    };
    let runtime_result = match request
        .config_id
        .as_deref()
        .filter(|config_id| !config_id.trim().is_empty())
    {
        Some(config_id) => load_runtime_for_config(&db, &vault, config_id),
        None => load_runtime(&db, &vault),
    };
    let runtime = match runtime_result {
        Ok(runtime) => runtime,
        Err(error) => {
            return Ok(CommandResponse::failure(ErrorEnvelope::new(
                "agent.ai_not_configured",
                error.to_string(),
                false,
            )))
        }
    };
    let command = RunAgentCommand {
        run_id: uuid::Uuid::new_v4().to_string(),
        provider_id: runtime.config.provider.clone(),
        model: runtime.config.model.clone(),
        system_prompt: AGENT_SYSTEM_PROMPT.into(),
        user_prompt: request.prompt.trim().into(),
        temperature: 0.1,
        limits: AgentLimits::default(),
    };
    let store = ApplicationStore::new(&db);
    Ok(
        match AgentService::run(&store, &runtime, &tools, command).await {
            Ok(outcome) => CommandResponse::success(outcome),
            Err(error) => CommandResponse::failure(application_error(error)),
        },
    )
}

#[tauri::command]
pub fn agent_get_run(
    db: State<'_, AppDb>,
    run_id: String,
) -> CommandResponse<Option<ielts_db::AgentRunRecord>> {
    match db.with_conn(|conn| ielts_db::load_agent_run(conn, &run_id)) {
        Ok(run) => CommandResponse::success(run),
        Err(error) => CommandResponse::failure(ErrorEnvelope::new(
            "agent.persistence_failed",
            error.to_string(),
            false,
        )),
    }
}

fn path_error(message: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope::new("agent.workspace_grant", message, false)
}

fn application_error(error: ApplicationError) -> ErrorEnvelope {
    ErrorEnvelope::new(error.code, error.message, error.retryable)
}
