use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_RECENT_PROJECTS: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub default_model: String,
    pub parallel_limit: u8,
    pub auto_compaction: bool,
    pub yolo_enabled: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            default_model: "rubyn".into(),
            parallel_limit: 3,
            auto_compaction: true,
            yolo_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalAppState {
    pub preferences: AppPreferences,
    pub recent_projects: Vec<RecentProject>,
}

impl LocalAppState {
    pub fn normalized(mut self) -> Self {
        self.preferences.parallel_limit = self.preferences.parallel_limit.clamp(1, 12);
        self.preferences.default_model = self.preferences.default_model.trim().to_owned();
        if self.preferences.default_model.is_empty() {
            self.preferences.default_model = "rubyn".into();
        }
        self.recent_projects
            .retain(|project| !project.path.trim().is_empty());
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
        self
    }

    pub fn record_project(&mut self, path: &Path) {
        let canonical = path.to_string_lossy().into_owned();
        self.recent_projects
            .retain(|project| project.path != canonical);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Project")
            .to_owned();
        self.recent_projects.insert(
            0,
            RecentProject {
                path: canonical,
                name,
            },
        );
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub available: bool,
    pub healthy: bool,
    pub source: EngineSource,
    pub executable: String,
    pub version: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineSource {
    Bundled,
    Installed,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub path: String,
    pub name: String,
    pub git_root: Option<String>,
    pub is_ruby: bool,
    pub is_rails: bool,
    pub has_rubyn_instructions: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    pub name: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillContent {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillRequest {
    pub project_path: String,
    pub name: String,
    pub content: String,
}

impl ProjectSummary {
    pub fn from_path(path: PathBuf, git_root: Option<PathBuf>) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Project")
            .to_owned();
        let is_ruby = path.join("Gemfile").is_file() || path.join(".ruby-version").is_file();
        let is_rails = path.join("config/application.rb").is_file()
            || (is_ruby
                && path.join("app/models").is_dir()
                && path.join("config/routes.rb").is_file());
        Self {
            path: path.to_string_lossy().into_owned(),
            name,
            git_root: git_root.map(|root| root.to_string_lossy().into_owned()),
            is_ruby,
            is_rails,
            has_rubyn_instructions: path.join("RUBYN.md").is_file(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub worktree_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: Option<String>,
    pub files: Vec<GitFileStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiff {
    pub diff: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineLaunchMode {
    Ide,
    Prompt { prompt: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub provider: String,
    pub model: String,
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalog {
    pub models: Vec<ModelOption>,
    pub active_provider: String,
    pub active_model: String,
    pub model_mode: String,
    #[serde(default)]
    pub connected_providers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertProviderRequest {
    pub name: String,
    pub base_url: String,
    pub api_format: String,
    #[serde(default)]
    pub env_key: String,
    #[serde(default)]
    pub api_key: String,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchEngineRequest {
    pub project_path: String,
    pub mode: EngineLaunchMode,
    #[serde(default)]
    pub yolo: bool,
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub resume_session: bool,
    #[serde(default)]
    pub backend_thread_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRunMessageRequest {
    pub run_id: u64,
    pub message: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSession {
    pub id: u64,
    /// Kept as the worktree path for compatibility with the first UI bridge.
    pub project_path: String,
    pub source_project_path: String,
    pub worktree_path: String,
    pub mode: String,
    pub pid: Option<u32>,
    pub running: bool,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSessionOutput {
    pub session: EngineSession,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: u64,
    pub path: String,
    pub name: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowColumn {
    pub id: u64,
    pub project_id: u64,
    pub key: String,
    pub name: String,
    pub position: u32,
    pub terminal: bool,
    #[serde(default)]
    pub agent_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    pub id: u64,
    pub project_id: u64,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub instructions: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: u64,
    pub project_id: u64,
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub outcome: String,
    pub status: String,
    #[serde(default)]
    pub depends_on: Vec<u64>,
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub assigned_run_id: Option<u64>,
    #[serde(default)]
    pub assigned_agent_id: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TodoRecord {
    pub id: u64,
    pub project_id: u64,
    pub title: String,
    pub owner: String,
    pub status: String,
    #[serde(default)]
    pub assigned_run_id: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: u64,
    pub project_id: u64,
    pub source_project_path: String,
    pub worktree_path: String,
    pub base_commit: String,
    pub prompt: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived_at: Option<u64>,
    #[serde(default)]
    pub background: bool,
    pub mode: String,
    pub pid: Option<u32>,
    pub running: bool,
    pub outcome: String,
    pub lifecycle: String,
    pub stdout: String,
    pub stderr: String,
    pub integrated_commit: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub finished_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationRequest {
    pub id: u64,
    pub title: Option<String>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunEventRecord {
    pub id: u64,
    pub run_id: u64,
    pub protocol_sequence: u64,
    pub kind: String,
    pub payload: serde_json::Value,
    pub raw: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EditApprovalRecord {
    pub id: u64,
    pub run_id: u64,
    pub edit_id: String,
    pub path: String,
    pub content: String,
    pub edit_type: String,
    pub status: String,
    pub requested_at: u64,
    pub decided_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveEditApprovalRequest {
    pub run_id: u64,
    pub edit_id: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEventBatch {
    pub run: RunRecord,
    pub events: Vec<RunEventRecord>,
    pub next_event_id: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectData {
    pub project: ProjectRecord,
    pub agents: Vec<AgentProfile>,
    pub columns: Vec<WorkflowColumn>,
    pub tasks: Vec<TaskRecord>,
    pub todos: Vec<TodoRecord>,
    pub runs: Vec<RunRecord>,
    pub approvals: Vec<EditApprovalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderMap {
    pub id: u64,
    pub project_id: u64,
    pub title: String,
    pub idea: String,
    pub destination: String,
    pub notes: String,
    #[serde(default)]
    pub code_task_status: String,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderTicket {
    pub id: u64,
    pub map_id: u64,
    pub title: String,
    pub question: String,
    pub information: String,
    pub outcome: String,
    pub ticket_type: String,
    pub status: String,
    #[serde(default)]
    pub depends_on: Vec<u64>,
    #[serde(default)]
    pub linked_task_id: Option<u64>,
    #[serde(default)]
    pub linked_run_id: Option<u64>,
    #[serde(default = "default_brief_version")]
    pub brief_version: u32,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub result_note: String,
    pub model_role: String,
    pub effort: String,
    #[serde(default)]
    pub budget_cents: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

fn default_brief_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderQuestionOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub pros: String,
    pub cons: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderQuestion {
    pub id: u64,
    pub ticket_id: u64,
    pub round: u32,
    pub title: String,
    pub prompt: String,
    pub cardinality: String,
    pub options: Vec<WayfinderQuestionOption>,
    #[serde(default)]
    pub answers: Vec<String>,
    #[serde(default)]
    pub custom_answer: String,
    #[serde(default)]
    pub answered_at: Option<u64>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderEvent {
    pub id: u64,
    pub map_id: u64,
    #[serde(default)]
    pub ticket_id: Option<u64>,
    pub kind: String,
    pub actor: String,
    pub payload: serde_json::Value,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderMapData {
    pub map: WayfinderMap,
    pub tickets: Vec<WayfinderTicket>,
    pub questions: Vec<WayfinderQuestion>,
    pub events: Vec<WayfinderEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWayfinderMapRequest {
    pub project_path: String,
    pub idea: String,
    #[serde(default)]
    pub code_task_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWayfinderTicketRequest {
    pub map_id: u64,
    pub title: String,
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub information: String,
    #[serde(default)]
    pub outcome: String,
    pub ticket_type: String,
    #[serde(default)]
    pub depends_on: Vec<u64>,
    #[serde(default)]
    pub model_role: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub budget_cents: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWayfinderTicketRequest {
    pub id: u64,
    pub title: Option<String>,
    pub question: Option<String>,
    pub information: Option<String>,
    pub outcome: Option<String>,
    pub depends_on: Option<Vec<u64>>,
    pub model_role: Option<String>,
    pub effort: Option<String>,
    pub budget_cents: Option<Option<u64>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderAnswer {
    pub question_id: u64,
    #[serde(default)]
    pub answers: Vec<String>,
    #[serde(default)]
    pub custom_answer: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveWayfinderTicketRequest {
    pub ticket_id: u64,
    pub resolution: String,
    #[serde(default)]
    pub add_tickets: Vec<CreateWayfinderTicketRequest>,
    #[serde(default)]
    pub retire_ticket_ids: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectTaskRequest {
    pub project_path: String,
    pub title: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default = "default_queued_status")]
    pub status: String,
    #[serde(default)]
    pub depends_on: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectTaskRequest {
    pub id: u64,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub outcome: Option<String>,
    pub status: Option<String>,
    pub depends_on: Option<Vec<u64>>,
    pub assigned_run_id: Option<Option<u64>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowColumnRequest {
    pub project_path: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkflowColumnRequest {
    pub id: u64,
    pub name: Option<String>,
    pub position: Option<u32>,
    pub agent_id: Option<Option<u64>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentProfileRequest {
    pub project_path: String,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub instructions: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentProfileRequest {
    pub id: u64,
    pub name: Option<String>,
    pub role: Option<String>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkflowColumnRequest {
    pub id: u64,
    pub move_tasks_to: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectTodoRequest {
    pub project_path: String,
    pub title: String,
    #[serde(default = "default_owner")]
    pub owner: String,
    #[serde(default = "default_queued_status")]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectTodoRequest {
    pub id: u64,
    pub title: Option<String>,
    pub owner: Option<String>,
    pub status: Option<String>,
    pub assigned_run_id: Option<Option<u64>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorktreeInspection {
    pub run: RunRecord,
    pub status: GitStatus,
    pub diff: GitDiff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeActionResult {
    pub run: RunRecord,
    pub commit_oid: Option<String>,
    pub cleanup_pending: bool,
}

fn default_queued_status() -> String {
    "queued".into()
}

fn default_owner() -> String {
    "You".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_untrusted_preferences() {
        let state = LocalAppState {
            preferences: AppPreferences {
                default_model: "  ".into(),
                parallel_limit: 99,
                auto_compaction: true,
                yolo_enabled: false,
            },
            recent_projects: Vec::new(),
        }
        .normalized();

        assert_eq!(state.preferences.default_model, "rubyn");
        assert_eq!(state.preferences.parallel_limit, 12);
    }
}
