mod domain;
mod engine;
mod services;
mod store;

use crate::{
    domain::{
        AgentProfile, CreateAgentProfileRequest, CreateProjectTaskRequest,
        CreateProjectTodoRequest, CreateSkillRequest, CreateWayfinderMapRequest,
        CreateWayfinderTicketRequest, CreateWorkflowColumnRequest, DeleteWorkflowColumnRequest,
        EditApprovalRecord, EngineInfo, EngineSession, EngineSessionOutput, GitDiff, GitStatus,
        LaunchEngineRequest, LocalAppState, ModelCatalog, ProjectData, ProjectRecord,
        ProjectSummary, ResolveEditApprovalRequest, ResolveWayfinderTicketRequest, RunEventBatch,
        RunRecord, RunWorktreeInspection, SendRunMessageRequest, SkillSummary, TaskRecord,
        TodoRecord, UpdateAgentProfileRequest, UpdateConversationRequest, UpdateProjectTaskRequest,
        UpdateProjectTodoRequest, UpdateWayfinderTicketRequest, UpdateWorkflowColumnRequest,
        UpsertProviderRequest, WayfinderAnswer, WayfinderMap, WayfinderMapData, WayfinderTicket,
        WorkflowColumn, WorktreeActionResult,
    },
    engine::{EngineError, EngineLocation, EngineSupervisor},
    store::StateRepository,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    thread,
};
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
struct AppRuntime {
    store: Mutex<Option<StateRepository>>,
    engines: Mutex<EngineSupervisor>,
}

type CommandResult<T> = Result<T, String>;

fn refresh_builtin_models(catalog: &mut ModelCatalog) {
    const BUILTIN_PROVIDERS: [&str; 3] = ["anthropic", "openai", "minimax"];
    const CURRENT: [(&str, &str, &str); 10] = [
        ("anthropic", "claude-fable-5", "frontier"),
        ("anthropic", "claude-opus-5", "top"),
        ("anthropic", "claude-sonnet-5", "mid"),
        ("anthropic", "claude-haiku-4-5", "cheap"),
        ("openai", "gpt-5.6-sol", "top"),
        ("openai", "gpt-5.6-terra", "mid"),
        ("openai", "gpt-5.6-luna", "cheap"),
        ("minimax", "MiniMax-M3", "top"),
        ("minimax", "MiniMax-M2.7-highspeed", "fast"),
        ("minimax", "MiniMax-M2.7", "mid"),
    ];
    catalog
        .models
        .retain(|model| !BUILTIN_PROVIDERS.contains(&model.provider.as_str()));
    catalog.models.extend(
        CURRENT
            .into_iter()
            .map(|(provider, model, tier)| domain::ModelOption {
                provider: provider.into(),
                model: model.into(),
                tier: tier.into(),
            }),
    );
    let active_is_available = catalog.models.iter().any(|model| {
        model.provider == catalog.active_provider && model.model == catalog.active_model
    });
    if !active_is_available {
        catalog.active_model = match catalog.active_provider.as_str() {
            "anthropic" => "claude-sonnet-5",
            "openai" => "gpt-5.6-terra",
            "minimax" => "MiniMax-M3",
            _ => return,
        }
        .into();
    }
}

#[tauri::command]
fn engine_health(app: AppHandle) -> CommandResult<EngineInfo> {
    Ok(services::engine_info(bundled_engine_root(&app)))
}

#[tauri::command]
fn list_models(app: AppHandle) -> CommandResult<ModelCatalog> {
    let location = engine_location(&app).ok_or_else(|| "Rubyn Code is not available".to_owned())?;
    let value = engine::one_shot_rpc(location, "models/list", serde_json::json!({}))
        .map_err(to_command_error)?;
    let mut catalog: ModelCatalog = serde_json::from_value(value).map_err(to_command_error)?;
    refresh_builtin_models(&mut catalog);
    if let Ok(account) =
        engine::codex_one_shot_rpc("account/read", serde_json::json!({"refreshToken":false}))
    {
        if account.get("account").is_some_and(|value| !value.is_null()) {
            catalog.connected_providers.push("codex".into());
            if let Ok(codex) = engine::codex_one_shot_rpc(
                "model/list",
                serde_json::json!({"limit":100,"includeHidden":false}),
            ) {
                if let Some(models) = codex.get("data").and_then(serde_json::Value::as_array) {
                    for item in models {
                        let Some(model) = item.get("model").and_then(serde_json::Value::as_str)
                        else {
                            continue;
                        };
                        catalog.models.push(domain::ModelOption {
                            provider: "codex".into(),
                            model: model.into(),
                            tier: "subscription".into(),
                        });
                    }
                }
            }
        }
    }
    catalog.models.sort_by(|left, right| {
        left.provider
            .cmp(&right.provider)
            .then(model_tier_rank(&left.tier).cmp(&model_tier_rank(&right.tier)))
            .then(left.model.cmp(&right.model))
    });
    catalog
        .models
        .dedup_by(|left, right| left.provider == right.provider && left.model == right.model);
    catalog.connected_providers.sort();
    catalog.connected_providers.dedup();
    Ok(catalog)
}

fn ensure_provider_connected(app: &AppHandle, provider: Option<&str>) -> CommandResult<()> {
    let Some(provider) = provider else {
        return Ok(());
    };
    let catalog = list_models(app.clone())?;
    if catalog
        .connected_providers
        .iter()
        .any(|name| name == provider)
    {
        Ok(())
    } else {
        Err(format!(
            "Connect {provider} in Models & accounts before using one of its models"
        ))
    }
}

fn model_tier_rank(tier: &str) -> u8 {
    match tier {
        "frontier" => 0,
        "top" => 1,
        "mid" => 2,
        "fast" | "subscription" => 3,
        "cheap" => 4,
        _ => 5,
    }
}

#[tauri::command]
fn upsert_provider(app: AppHandle, request: UpsertProviderRequest) -> CommandResult<ModelCatalog> {
    let location = engine_location(&app).ok_or_else(|| "Rubyn Code is not available".to_owned())?;
    let result = engine::one_shot_rpc(
        location,
        "providers/upsert",
        serde_json::json!({
            "name": request.name,
            "baseUrl": request.base_url,
            "apiFormat": request.api_format,
            "envKey": request.env_key,
            "apiKey": request.api_key,
            "models": request.models,
        }),
    )
    .map_err(to_command_error)?;
    if result.get("updated").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(result
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Provider was not saved")
            .to_owned());
    }
    list_models(app)
}

#[tauri::command]
fn get_chisel_mode(app: AppHandle) -> CommandResult<String> {
    let location = engine_location(&app).ok_or_else(|| "Rubyn Code is not available".to_owned())?;
    let result = engine::one_shot_rpc(
        location,
        "config/get",
        serde_json::json!({"key":"chisel_mode"}),
    )
    .map_err(to_command_error)?;
    result
        .get("value")
        .and_then(serde_json::Value::as_str)
        .filter(|mode| matches!(*mode, "off" | "lite" | "full" | "ultra"))
        .map(str::to_owned)
        .ok_or_else(|| "Rubyn Code returned an invalid Chisel mode".to_owned())
}

#[tauri::command]
fn set_chisel_enabled(app: AppHandle, enabled: bool) -> CommandResult<String> {
    let location = engine_location(&app).ok_or_else(|| "Rubyn Code is not available".to_owned())?;
    let mode = if enabled { "full" } else { "off" };
    let result = engine::one_shot_rpc(
        location,
        "config/set",
        serde_json::json!({"key":"chisel_mode","value":mode}),
    )
    .map_err(to_command_error)?;
    if result.get("updated").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(result
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Chisel mode was not saved")
            .to_owned());
    }
    Ok(mode.to_owned())
}

#[tauri::command]
fn start_codex_login() -> CommandResult<()> {
    let executable = engine::codex_executable()
        .map_err(|error| format!("Unable to start Codex login: {error}"))?;
    let mut child = Command::new(executable)
        .arg("login")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Unable to start Codex login: {error}"))?;
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[tauri::command]
fn scan_projects(roots: Vec<String>) -> CommandResult<Vec<ProjectSummary>> {
    services::scan_projects(&roots).map_err(to_command_error)
}

#[tauri::command]
fn inspect_project(project_path: String) -> CommandResult<ProjectSummary> {
    services::inspect_project(&project_path).map_err(to_command_error)
}

#[tauri::command]
fn trust_project(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    project_path: String,
) -> CommandResult<LocalAppState> {
    let project = services::canonical_project(&project_path).map_err(to_command_error)?;
    let inspected =
        services::inspect_project(project.to_string_lossy().as_ref()).map_err(to_command_error)?;
    if inspected.git_root.is_none() {
        return Err(format!(
            "{} is not a Git repository. Initialize Git or choose the repository root.",
            project.display()
        ));
    }
    let mut repository = state_repository(&app, &runtime)?;
    let repository = repository
        .as_mut()
        .expect("state repository is initialized");
    let mut state = repository.snapshot();
    state.trust_project(&project);
    repository.replace(state).map_err(to_command_error)
}

#[tauri::command]
fn get_git_status(project_path: String) -> CommandResult<GitStatus> {
    services::git_status(&project_path).map_err(to_command_error)
}

#[tauri::command]
fn get_git_diff(project_path: String, staged: bool) -> CommandResult<GitDiff> {
    services::git_diff(&project_path, staged).map_err(to_command_error)
}

#[tauri::command]
fn get_app_state(app: AppHandle, runtime: State<'_, AppRuntime>) -> CommandResult<LocalAppState> {
    let repository = state_repository(&app, &runtime)?;
    Ok(repository
        .as_ref()
        .expect("state repository is initialized")
        .snapshot())
}

#[tauri::command]
fn save_app_state(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    state: LocalAppState,
) -> CommandResult<LocalAppState> {
    let mut repository = state_repository(&app, &runtime)?;
    repository
        .as_mut()
        .expect("state repository is initialized")
        .replace(state)
        .map_err(to_command_error)
}

#[tauri::command]
fn launch_engine(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: LaunchEngineRequest,
) -> CommandResult<EngineSession> {
    if request.yolo {
        return Err(
            "Bypass mode is disabled in this release; use Rubyn's approval workflow.".to_owned(),
        );
    }
    ensure_provider_connected(&app, request.provider.as_deref())?;
    let attachment_summaries = attachment_summaries(&request.attachments);
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    ensure_project_trusted(&app, &runtime, &project)?;
    let parallel_limit = state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .snapshot()
        .preferences
        .parallel_limit as usize;
    let mut engines = runtime
        .engines
        .lock()
        .map_err(|_| "Engine supervisor is unavailable".to_owned())?;
    if engines.at_capacity(parallel_limit) {
        return Err("Parallel run limit reached; wait for an active run to finish".to_owned());
    }
    let runs_root = app
        .path()
        .app_data_dir()
        .map_err(to_command_error)?
        .join("worktrees");
    let allocation =
        services::create_isolated_worktree(&project, &runs_root).map_err(to_command_error)?;
    let location = match engine_location(&app) {
        Some(location) => location,
        None => {
            let _ = services::remove_isolated_worktree(&project, &allocation.path, &runs_root);
            return Err("Rubyn Code is not available".to_owned());
        }
    };
    let prompt = match &request.mode {
        domain::EngineLaunchMode::Prompt { prompt } => prompt.clone(),
        domain::EngineLaunchMode::Ide => String::new(),
    };
    let mode = match &request.mode {
        domain::EngineLaunchMode::Ide => "ide",
        domain::EngineLaunchMode::Prompt { .. } => "prompt",
    };
    let run = {
        let mut repository = state_repository(&app, &runtime)?;
        match repository
            .as_mut()
            .expect("state repository is initialized")
            .allocate_run(
                &project,
                &allocation.path,
                allocation.base_commit.clone(),
                prompt,
                mode.into(),
            ) {
            Ok(run) => run,
            Err(error) => {
                let _ = services::remove_isolated_worktree(&project, &allocation.path, &runs_root);
                return Err(to_command_error(error));
            }
        }
    };
    let session = match engines.launch(run.id, location, request, &allocation.path, parallel_limit)
    {
        Ok(session) => session,
        Err(error) => {
            let _ = services::remove_isolated_worktree(&project, &allocation.path, &runs_root);
            if let Ok(mut repository) = state_repository(&app, &runtime) {
                let _ = repository
                    .as_mut()
                    .expect("state repository is initialized")
                    .mark_run_launch_failed(run.id, &error.to_string());
            }
            return Err(to_command_error(error));
        }
    };
    if let Err(error) = state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .mark_run_started(run.id, session.pid)
    {
        let _ = engines.stop(run.id);
        let _ = services::remove_isolated_worktree(&project, &allocation.path, &runs_root);
        return Err(to_command_error(error));
    }
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .append_user_message(run.id, &run.prompt, attachment_summaries)
        .map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .refresh_run_control_snapshot(run.id)
        .map_err(to_command_error)?;
    Ok(session)
}

#[tauri::command]
fn list_engine_sessions(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
) -> CommandResult<Vec<EngineSession>> {
    sync_runtime_state(&app, &runtime)
}

#[tauri::command]
fn stop_engine(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    session_id: u64,
) -> CommandResult<()> {
    {
        runtime
            .engines
            .lock()
            .map_err(|_| "Engine supervisor is unavailable".to_owned())?
            .stop(session_id)
            .map_err(to_command_error)?;
    }
    sync_runtime_state(&app, &runtime).map(|_| ())
}

#[tauri::command]
fn answer_engine_question(
    runtime: State<'_, AppRuntime>,
    run_id: u64,
    request_id: serde_json::Value,
    answer: serde_json::Value,
) -> CommandResult<()> {
    runtime
        .engines
        .lock()
        .map_err(|_| "Engine supervisor is unavailable".to_owned())?
        .answer_question(run_id, request_id, answer)
        .map_err(to_command_error)
}

#[tauri::command]
fn resolve_edit_approval(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: ResolveEditApprovalRequest,
) -> CommandResult<EditApprovalRecord> {
    sync_runtime_state(&app, &runtime)?;
    state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .pending_edit_approval(request.run_id, &request.edit_id)
        .map_err(to_command_error)?;
    runtime
        .engines
        .lock()
        .map_err(|_| "Engine supervisor is unavailable".to_owned())?
        .resolve_edit(request.run_id, &request.edit_id, request.accepted)
        .map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .resolve_edit_approval(request.run_id, &request.edit_id, request.accepted)
        .map_err(to_command_error)
}

#[tauri::command]
fn send_run_message(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: SendRunMessageRequest,
) -> CommandResult<EngineSession> {
    sync_runtime_state(&app, &runtime)?;
    ensure_provider_connected(&app, request.provider.as_deref())?;
    let needs_restart = {
        let mut engines = runtime
            .engines
            .lock()
            .map_err(|_| "Engine supervisor is unavailable".to_owned())?;
        match engines.send_message(
            request.run_id,
            &request.message,
            &request.attachments,
            request.provider.as_deref(),
            request.model.as_deref(),
        ) {
            Ok(()) => false,
            Err(
                EngineError::UnknownSession(_)
                | EngineError::ConversationClosed(_)
                | EngineError::BackendChanged(_),
            ) => true,
            Err(error) => return Err(to_command_error(error)),
        }
    };
    if needs_restart {
        let (run, mut backend_thread_id, tools_enabled, prior_context) = {
            let repository = state_repository(&app, &runtime)?;
            let repository = repository
                .as_ref()
                .expect("state repository is initialized");
            (
                repository.run(request.run_id).map_err(to_command_error)?,
                repository
                    .backend_thread_id(request.run_id)
                    .map_err(to_command_error)?,
                repository
                    .codex_harness_tools_enabled(request.run_id)
                    .map_err(to_command_error)?,
                repository
                    .conversation_context(request.run_id)
                    .map_err(to_command_error)?,
            )
        };
        if run.archived_at.is_some() {
            return Err("Restore this conversation before continuing it".into());
        }
        if run.lifecycle != "retained" {
            return Err(format!(
                "This conversation's worktree is {}; it cannot be continued",
                run.lifecycle
            ));
        }
        let worktree = PathBuf::from(&run.worktree_path)
            .canonicalize()
            .map_err(|error| {
                format!("The retained conversation worktree is unavailable: {error}")
            })?;
        let managed_root = runs_root(&app)?
            .canonicalize()
            .map_err(|error| format!("The Harness worktree directory is unavailable: {error}"))?;
        if !worktree.starts_with(&managed_root) {
            return Err("The retained conversation worktree is outside Harness storage".into());
        }
        let location =
            engine_location(&app).ok_or_else(|| "Rubyn Code is not available".to_owned())?;
        let migrate_codex_thread = request.provider.as_deref() == Some("codex")
            && backend_thread_id.is_some()
            && !tools_enabled;
        if migrate_codex_thread {
            backend_thread_id = None;
        }
        let prompt = if migrate_codex_thread {
            format!(
                "This is a continuation of the same Rubyn Harness conversation. The Harness restarted the underlying Codex thread once so it could attach the wayfinder and harness_task tools. Continue naturally from this retained transcript; do not tell the user to start another conversation.\n\n<retained-conversation>\n{prior_context}\n</retained-conversation>\n\n<current-user-message>\n{}\n</current-user-message>",
                request.message
            )
        } else {
            request.message.clone()
        };
        let launch_request = LaunchEngineRequest {
            project_path: run.source_project_path.clone(),
            mode: domain::EngineLaunchMode::Prompt { prompt },
            yolo: false,
            attachments: request.attachments.clone(),
            provider: request.provider.clone(),
            model: request.model.clone(),
            resume_session: true,
            backend_thread_id,
        };
        let parallel_limit = state_repository(&app, &runtime)?
            .as_ref()
            .expect("state repository is initialized")
            .snapshot()
            .preferences
            .parallel_limit as usize;
        let session = runtime
            .engines
            .lock()
            .map_err(|_| "Engine supervisor is unavailable".to_owned())?
            .launch(
                request.run_id,
                location,
                launch_request,
                &worktree,
                parallel_limit,
            )
            .map_err(to_command_error)?;
        state_repository(&app, &runtime)?
            .as_mut()
            .expect("state repository is initialized")
            .mark_run_started(request.run_id, session.pid)
            .map_err(to_command_error)?;
    }
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .append_user_message(
            request.run_id,
            &request.message,
            attachment_summaries(&request.attachments),
        )
        .map_err(to_command_error)?;
    sync_runtime_state(&app, &runtime)?
        .into_iter()
        .find(|session| session.id == request.run_id)
        .ok_or_else(|| format!("Run {} is no longer active", request.run_id))
}

fn attachment_summaries(attachments: &[domain::AttachmentInput]) -> serde_json::Value {
    serde_json::Value::Array(
        attachments
            .iter()
            .map(|attachment| {
                let name = std::path::Path::new(&attachment.path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("attachment");
                let extension = std::path::Path::new(name)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let kind = if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp")
                {
                    "image"
                } else {
                    "text"
                };
                serde_json::json!({"name": name, "kind": kind})
            })
            .collect(),
    )
}

#[tauri::command]
fn get_engine_session_output(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    session_id: u64,
) -> CommandResult<EngineSessionOutput> {
    sync_runtime_state(&app, &runtime)?;
    runtime
        .engines
        .lock()
        .map_err(|_| "Engine supervisor is unavailable".to_owned())?
        .output(session_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn list_projects(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
) -> CommandResult<Vec<ProjectRecord>> {
    let repository = state_repository(&app, &runtime)?;
    Ok(repository
        .as_ref()
        .expect("state repository is initialized")
        .projects())
}

#[tauri::command]
fn get_project_data(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    project_path: String,
) -> CommandResult<ProjectData> {
    let project = services::canonical_project(&project_path).map_err(to_command_error)?;
    ensure_project_trusted(&app, &runtime, &project)?;
    let mut repository = state_repository(&app, &runtime)?;
    let repository = repository
        .as_mut()
        .expect("state repository is initialized");
    repository
        .record_project(&project)
        .map_err(to_command_error)?;
    repository.project_data(&project).map_err(to_command_error)
}

#[tauri::command]
fn create_project_task(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: CreateProjectTaskRequest,
) -> CommandResult<TaskRecord> {
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .create_task(
            &project,
            &request.title,
            &request.detail,
            &request.outcome,
            &request.status,
            request.depends_on,
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn update_project_task(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: UpdateProjectTaskRequest,
) -> CommandResult<TaskRecord> {
    let mut repository = state_repository(&app, &runtime)?;
    let repository = repository
        .as_mut()
        .expect("state repository is initialized");
    let task = repository
        .update_task(
            request.id,
            request.title.as_deref(),
            request.detail.as_deref(),
            request.outcome.as_deref(),
            request.status.as_deref(),
            request.depends_on,
        )
        .map_err(to_command_error)?;
    match request.assigned_run_id {
        Some(assignment) => repository
            .assign_task(request.id, assignment)
            .map_err(to_command_error),
        None => Ok(task),
    }
}

#[tauri::command]
fn create_workflow_column(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: CreateWorkflowColumnRequest,
) -> CommandResult<WorkflowColumn> {
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .create_workflow_column(&project, &request.name)
        .map_err(to_command_error)
}

#[tauri::command]
fn update_workflow_column(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: UpdateWorkflowColumnRequest,
) -> CommandResult<WorkflowColumn> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .update_workflow_column(
            request.id,
            request.name.as_deref(),
            request.position,
            request.agent_id,
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn create_agent_profile(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: CreateAgentProfileRequest,
) -> CommandResult<AgentProfile> {
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .create_agent_profile(
            &project,
            &request.name,
            &request.role,
            &request.instructions,
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn update_agent_profile(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: UpdateAgentProfileRequest,
) -> CommandResult<AgentProfile> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .update_agent_profile(
            request.id,
            request.name.as_deref(),
            request.role.as_deref(),
            request.instructions.as_deref(),
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn delete_agent_profile(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    id: u64,
) -> CommandResult<()> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .delete_agent_profile(id)
        .map_err(to_command_error)
}

#[tauri::command]
fn delete_workflow_column(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: DeleteWorkflowColumnRequest,
) -> CommandResult<()> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .delete_workflow_column(request.id, request.move_tasks_to)
        .map_err(to_command_error)
}

#[tauri::command]
fn create_project_todo(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: CreateProjectTodoRequest,
) -> CommandResult<TodoRecord> {
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .create_todo(&project, &request.title, &request.owner, &request.status)
        .map_err(to_command_error)
}

#[tauri::command]
fn update_project_todo(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: UpdateProjectTodoRequest,
) -> CommandResult<TodoRecord> {
    let mut repository = state_repository(&app, &runtime)?;
    let repository = repository
        .as_mut()
        .expect("state repository is initialized");
    let todo = repository
        .update_todo(
            request.id,
            request.title.as_deref(),
            request.owner.as_deref(),
            request.status.as_deref(),
        )
        .map_err(to_command_error)?;
    match request.assigned_run_id {
        Some(assignment) => repository
            .assign_todo(request.id, assignment)
            .map_err(to_command_error),
        None => Ok(todo),
    }
}

#[tauri::command]
fn list_runs(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    project_path: Option<String>,
) -> CommandResult<Vec<RunRecord>> {
    sync_runtime_state(&app, &runtime)?;
    let canonical = project_path
        .as_deref()
        .map(services::canonical_project)
        .transpose()
        .map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .runs(canonical.as_deref())
        .map_err(to_command_error)
}

#[tauri::command]
fn update_conversation(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: UpdateConversationRequest,
) -> CommandResult<RunRecord> {
    sync_runtime_state(&app, &runtime)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .update_conversation(
            request.id,
            request.title.as_deref(),
            request.pinned,
            request.archived,
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn poll_run_events(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    run_id: u64,
    after_event_id: Option<u64>,
) -> CommandResult<RunEventBatch> {
    sync_runtime_state(&app, &runtime)?;
    let repository = state_repository(&app, &runtime)?;
    let repository = repository
        .as_ref()
        .expect("state repository is initialized");
    let run = repository.run(run_id).map_err(to_command_error)?;
    let events = repository
        .events(run_id, after_event_id.unwrap_or(0))
        .map_err(to_command_error)?;
    let next_event_id = events
        .last()
        .map(|event| event.id)
        .unwrap_or(after_event_id.unwrap_or(0));
    Ok(RunEventBatch {
        run,
        events,
        next_event_id,
    })
}

#[tauri::command]
fn inspect_run_worktree(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    run_id: u64,
) -> CommandResult<RunWorktreeInspection> {
    sync_runtime_state(&app, &runtime)?;
    let run = state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .run(run_id)
        .map_err(to_command_error)?;
    let (status, diff) = services::inspect_isolated_worktree(
        Path::new(&run.worktree_path),
        &runs_root(&app)?,
        &run.base_commit,
    )
    .map_err(to_command_error)?;
    let readiness = services::inspect_integration_readiness(
        Path::new(&run.source_project_path),
        &run.base_commit,
    )
    .map_err(to_command_error)?;
    Ok(RunWorktreeInspection {
        run,
        status,
        diff,
        readiness,
    })
}

#[tauri::command]
fn integrate_run(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    run_id: u64,
) -> CommandResult<WorktreeActionResult> {
    sync_runtime_state(&app, &runtime)?;
    let run = state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .run(run_id)
        .map_err(to_command_error)?;
    if run.running {
        return Err("Stop the run before integrating its worktree".into());
    }
    if run.lifecycle != "retained" {
        return Err(format!(
            "Run {run_id} worktree is already {}",
            run.lifecycle
        ));
    }
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .mark_integration_started(run_id)
        .map_err(to_command_error)?;
    let root = runs_root(&app)?;
    let commit = match services::integrate_isolated_worktree(
        Path::new(&run.source_project_path),
        Path::new(&run.worktree_path),
        &root,
        &run.base_commit,
        run_id,
    ) {
        Ok(commit) => commit,
        Err(error) => {
            let _ = state_repository(&app, &runtime).and_then(|mut repository| {
                repository
                    .as_mut()
                    .expect("state repository is initialized")
                    .mark_integration_failed(run_id, &error.to_string())
                    .map_err(to_command_error)
            });
            return Err(to_command_error(error));
        }
    };
    let run = state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .mark_integrated(run_id, &commit, true)
        .map_err(to_command_error)?;
    let cleanup_pending = services::remove_isolated_worktree(
        Path::new(&run.source_project_path),
        Path::new(&run.worktree_path),
        &root,
    )
    .is_err();
    let run = if cleanup_pending {
        run
    } else {
        state_repository(&app, &runtime)?
            .as_mut()
            .expect("state repository is initialized")
            .mark_integrated(run_id, &commit, false)
            .map_err(to_command_error)?
    };
    Ok(WorktreeActionResult {
        run,
        commit_oid: Some(commit),
        cleanup_pending,
    })
}

#[tauri::command]
fn discard_run(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    run_id: u64,
) -> CommandResult<WorktreeActionResult> {
    sync_runtime_state(&app, &runtime)?;
    let run = state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .run(run_id)
        .map_err(to_command_error)?;
    if run.running {
        return Err("Stop the run before discarding its worktree".into());
    }
    if run.lifecycle != "retained" {
        return Err(format!(
            "Run {run_id} worktree is already {}",
            run.lifecycle
        ));
    }
    let cleanup_pending = services::remove_isolated_worktree(
        Path::new(&run.source_project_path),
        Path::new(&run.worktree_path),
        &runs_root(&app)?,
    )
    .is_err();
    let run = state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .mark_discarded(run_id, cleanup_pending)
        .map_err(to_command_error)?;
    Ok(WorktreeActionResult {
        run,
        commit_oid: None,
        cleanup_pending,
    })
}

#[tauri::command]
fn retry_run_cleanup(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    run_id: u64,
) -> CommandResult<WorktreeActionResult> {
    sync_runtime_state(&app, &runtime)?;
    let run = state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .run(run_id)
        .map_err(to_command_error)?;
    if run.lifecycle != "integrated_cleanup_pending" && run.lifecycle != "discard_cleanup_pending" {
        return Err(format!(
            "Run {run_id} does not have pending worktree cleanup"
        ));
    }
    let worktree = Path::new(&run.worktree_path);
    if worktree.exists() {
        services::remove_isolated_worktree(
            Path::new(&run.source_project_path),
            worktree,
            &runs_root(&app)?,
        )
        .map_err(to_command_error)?;
    }
    let run = state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .mark_cleanup_complete(run_id)
        .map_err(to_command_error)?;
    Ok(WorktreeActionResult {
        commit_oid: run.integrated_commit.clone(),
        run,
        cleanup_pending: false,
    })
}

#[tauri::command]
fn list_bundled_skills(app: AppHandle) -> CommandResult<Vec<SkillSummary>> {
    let root =
        bundled_engine_root(&app).ok_or_else(|| "Bundled Rubyn Code is unavailable".to_owned())?;
    services::list_skills(&root).map_err(to_command_error)
}

#[tauri::command]
fn create_project_skill(request: CreateSkillRequest) -> CommandResult<SkillSummary> {
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    services::create_project_skill(&project, &request.name, &request.content)
        .map_err(to_command_error)
}

#[tauri::command]
fn list_project_skills(project_path: String) -> CommandResult<Vec<SkillSummary>> {
    let project = services::canonical_project(&project_path).map_err(to_command_error)?;
    services::list_project_skills(&project).map_err(to_command_error)
}

#[tauri::command]
fn read_skill(
    app: AppHandle,
    path: String,
    project_path: Option<String>,
) -> CommandResult<domain::SkillContent> {
    let root = if let Some(project_path) = project_path {
        services::canonical_project(&project_path)
            .map_err(to_command_error)?
            .join(".rubyn-code/skills")
    } else {
        bundled_engine_root(&app)
            .ok_or_else(|| "Bundled Rubyn Code is unavailable".to_owned())?
            .join("skills")
    };
    let content = services::read_skill_file(&root, &path).map_err(to_command_error)?;
    Ok(domain::SkillContent { path, content })
}

fn sync_runtime_state(app: &AppHandle, runtime: &AppRuntime) -> CommandResult<Vec<EngineSession>> {
    let (sessions, snapshots, events) = {
        let mut engines = runtime
            .engines
            .lock()
            .map_err(|_| "Engine supervisor is unavailable".to_owned())?;
        let sessions = engines.sessions();
        let snapshots = engines.snapshots();
        let events = engines.pending_events();
        (sessions, snapshots, events)
    };
    {
        let mut repository = state_repository(app, runtime)?;
        let repository = repository
            .as_mut()
            .expect("state repository is initialized");
        repository
            .apply_harness_tool_events(&events)
            .map_err(to_command_error)?;
        repository
            .append_engine_events(&events)
            .map_err(to_command_error)?;
        for snapshot in &snapshots {
            repository
                .sync_run(
                    snapshot.session.id,
                    snapshot.session.running,
                    &snapshot.session.outcome,
                    snapshot.session.pid,
                    &snapshot.stdout,
                    &snapshot.stderr,
                )
                .map_err(to_command_error)?;
        }
    }
    runtime
        .engines
        .lock()
        .map_err(|_| "Engine supervisor is unavailable".to_owned())?
        .acknowledge_events(&events);
    Ok(sessions)
}

fn runs_root(app: &AppHandle) -> CommandResult<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(to_command_error)?
        .join("worktrees"))
}

fn state_repository<'a>(
    app: &AppHandle,
    runtime: &'a AppRuntime,
) -> CommandResult<std::sync::MutexGuard<'a, Option<StateRepository>>> {
    let mut guard = runtime
        .store
        .lock()
        .map_err(|_| "Local state is unavailable".to_owned())?;
    if guard.is_none() {
        let directory = app.path().app_data_dir().map_err(to_command_error)?;
        *guard = Some(StateRepository::open(directory).map_err(to_command_error)?);
    }
    Ok(guard)
}

fn ensure_project_trusted(
    app: &AppHandle,
    runtime: &AppRuntime,
    project: &Path,
) -> CommandResult<()> {
    let repository = state_repository(app, runtime)?;
    if repository
        .as_ref()
        .expect("state repository is initialized")
        .snapshot()
        .trusts_project(project)
    {
        Ok(())
    } else {
        Err(format!(
            "Project trust required for {}. Inspect and confirm this repository in Projects before opening or running work.",
            project.display()
        ))
    }
}

#[tauri::command]
fn list_wayfinder_maps(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    project_path: String,
) -> CommandResult<Vec<WayfinderMap>> {
    let project = services::canonical_project(&project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .wayfinder_maps(&project)
        .map_err(to_command_error)
}

#[tauri::command]
fn get_wayfinder_map(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    map_id: u64,
) -> CommandResult<WayfinderMapData> {
    sync_runtime_state(&app, &runtime)?;
    state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .wayfinder_map_data(map_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn create_wayfinder_map(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: CreateWayfinderMapRequest,
) -> CommandResult<WayfinderMapData> {
    let project = services::canonical_project(&request.project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .create_wayfinder_map(&project, &request.idea, request.code_task_status.as_deref())
        .map_err(to_command_error)
}

#[tauri::command]
fn update_wayfinder_map(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    map_id: u64,
    title: Option<String>,
    destination: Option<String>,
    notes: Option<String>,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .update_wayfinder_map(
            map_id,
            title.as_deref(),
            destination.as_deref(),
            notes.as_deref(),
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn create_wayfinder_ticket(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: CreateWayfinderTicketRequest,
) -> CommandResult<WayfinderTicket> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .create_wayfinder_ticket(&request, "user")
        .map_err(to_command_error)
}

#[tauri::command]
fn update_wayfinder_ticket(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: UpdateWayfinderTicketRequest,
) -> CommandResult<WayfinderTicket> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .update_wayfinder_ticket(&request)
        .map_err(to_command_error)
}

#[tauri::command]
fn submit_wayfinder_answers(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    ticket_id: u64,
    answers: Vec<WayfinderAnswer>,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .submit_wayfinder_answers(ticket_id, &answers)
        .map_err(to_command_error)
}

#[tauri::command]
fn activate_wayfinder_map(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    map_id: u64,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .activate_wayfinder_map(map_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn resolve_wayfinder_ticket(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    request: ResolveWayfinderTicketRequest,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .resolve_wayfinder_ticket(
            request.ticket_id,
            &request.resolution,
            &request.add_tickets,
            &request.retire_ticket_ids,
        )
        .map_err(to_command_error)
}

#[tauri::command]
fn complete_wayfinder_user_action(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    ticket_id: u64,
    result_note: String,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .complete_wayfinder_user_action(ticket_id, &result_note)
        .map_err(to_command_error)
}

#[tauri::command]
fn link_wayfinder_run(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    ticket_id: u64,
    run_id: u64,
) -> CommandResult<WayfinderTicket> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .link_wayfinder_run(ticket_id, run_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn retire_wayfinder_ticket(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    ticket_id: u64,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .retire_wayfinder_ticket(ticket_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn archive_wayfinder_map(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    map_id: u64,
) -> CommandResult<WayfinderMapData> {
    state_repository(&app, &runtime)?
        .as_mut()
        .expect("state repository is initialized")
        .archive_wayfinder_map(map_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn list_wayfinder_blockers(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    project_path: String,
) -> CommandResult<Vec<WayfinderTicket>> {
    let project = services::canonical_project(&project_path).map_err(to_command_error)?;
    state_repository(&app, &runtime)?
        .as_ref()
        .expect("state repository is initialized")
        .wayfinder_blockers(&project)
        .map_err(to_command_error)
}

fn bundled_engine_root(app: &AppHandle) -> Option<PathBuf> {
    let from_resources = app
        .path()
        .resource_dir()
        .ok()
        .map(|directory| directory.join("engine/rubyn-code"));
    let from_working_directory = std::env::current_dir().ok().and_then(|directory| {
        directory
            .ancestors()
            .map(|ancestor| ancestor.join("engine/rubyn-code"))
            .find(|candidate| candidate.join("exe/rubyn-code").is_file())
    });
    from_resources
        .filter(|candidate| candidate.join("exe/rubyn-code").is_file())
        .or(from_working_directory)
}

fn engine_location(app: &AppHandle) -> Option<EngineLocation> {
    if let Some(root) = bundled_engine_root(app) {
        return services::ruby_runtime_for(&root)
            .map(|ruby| EngineLocation::Bundled { root, ruby });
    }
    let info = services::engine_info(None);
    info.available.then_some(EngineLocation::Installed)
}

fn to_command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppRuntime::default())
        .invoke_handler(tauri::generate_handler![
            engine_health,
            list_models,
            upsert_provider,
            get_chisel_mode,
            set_chisel_enabled,
            start_codex_login,
            scan_projects,
            inspect_project,
            trust_project,
            get_git_status,
            get_git_diff,
            get_app_state,
            save_app_state,
            list_projects,
            get_project_data,
            create_project_task,
            update_project_task,
            create_workflow_column,
            update_workflow_column,
            delete_workflow_column,
            create_agent_profile,
            update_agent_profile,
            delete_agent_profile,
            create_project_todo,
            update_project_todo,
            launch_engine,
            list_engine_sessions,
            get_engine_session_output,
            stop_engine,
            answer_engine_question,
            resolve_edit_approval,
            send_run_message,
            list_runs,
            update_conversation,
            poll_run_events,
            inspect_run_worktree,
            integrate_run,
            discard_run,
            retry_run_cleanup,
            list_bundled_skills,
            create_project_skill,
            list_project_skills,
            read_skill,
            list_wayfinder_maps,
            get_wayfinder_map,
            create_wayfinder_map,
            update_wayfinder_map,
            create_wayfinder_ticket,
            update_wayfinder_ticket,
            submit_wayfinder_answers,
            activate_wayfinder_map,
            resolve_wayfinder_ticket,
            complete_wayfinder_user_action,
            link_wayfinder_run,
            retire_wayfinder_ticket,
            archive_wayfinder_map,
            list_wayfinder_blockers
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rubyn Harness");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_model_catalog_replaces_stale_defaults_and_preserves_custom_providers() {
        let mut catalog = ModelCatalog {
            models: vec![
                domain::ModelOption {
                    provider: "openai".into(),
                    model: "gpt-5.4".into(),
                    tier: "top".into(),
                },
                domain::ModelOption {
                    provider: "local".into(),
                    model: "my-model".into(),
                    tier: "custom".into(),
                },
            ],
            active_provider: "openai".into(),
            active_model: "gpt-5.4".into(),
            model_mode: "auto".into(),
            connected_providers: vec!["openai".into()],
        };

        refresh_builtin_models(&mut catalog);

        assert!(!catalog.models.iter().any(|model| model.model == "gpt-5.4"));
        assert!(catalog
            .models
            .iter()
            .any(|model| model.model == "gpt-5.6-sol"));
        assert!(catalog
            .models
            .iter()
            .any(|model| model.model == "claude-fable-5"));
        assert!(catalog
            .models
            .iter()
            .any(|model| model.model == "MiniMax-M3"));
        assert!(catalog.models.iter().any(|model| model.model == "my-model"));
        assert_eq!(catalog.active_model, "gpt-5.6-terra");
    }
}
