import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface EngineInfo {
  available: boolean;
  healthy: boolean;
  source: "bundled" | "installed" | "unavailable";
  executable: string;
  version?: string;
  detail?: string;
}

export interface ProjectSummary {
  path: string;
  name: string;
  gitRoot?: string;
  isRuby: boolean;
  isRails: boolean;
  hasRubynInstructions: boolean;
}

export interface RecentProject { path: string; name: string }

export interface LocalAppState {
  preferences: {
    defaultModel: string;
    parallelLimit: number;
    autoCompaction: boolean;
    yoloEnabled: boolean;
  };
  recentProjects: RecentProject[];
  onboardingVersion?: number;
  trustedProjectPaths?: string[];
}

export interface ProjectRecord {
  id: number;
  path: string;
  name: string;
  createdAt: number;
  updatedAt: number;
}

export type WorkflowStatus = "queued" | "doing" | "review" | "done";
export type TaskStatus = string;
export interface AgentProfile {
  id: number;
  projectId: number;
  name: string;
  role: string;
  instructions: string;
  createdAt: number;
  updatedAt: number;
}

export interface WorkflowColumn {
  id: number;
  projectId: number;
  key: string;
  name: string;
  position: number;
  terminal: boolean;
  agentId?: number;
}

export interface TaskRecord {
  id: number;
  projectId: number;
  title: string;
  detail: string;
  outcome: string;
  status: TaskStatus;
  dependsOn: number[];
  ready: boolean;
  assignedRunId?: number;
  assignedAgentId?: number;
  createdAt: number;
  updatedAt: number;
}

export interface TodoRecord {
  id: number;
  projectId: number;
  title: string;
  owner: string;
  status: WorkflowStatus;
  assignedRunId?: number;
  createdAt: number;
  updatedAt: number;
}

export interface RunRecord {
  id: number;
  projectId: number;
  sourceProjectPath: string;
  worktreePath: string;
  baseCommit: string;
  prompt: string;
  title?: string;
  pinned?: boolean;
  archivedAt?: number;
  background?: boolean;
  mode: string;
  pid?: number;
  running: boolean;
  outcome: string;
  lifecycle: string;
  stdout: string;
  stderr: string;
  integratedCommit?: string;
  createdAt: number;
  updatedAt: number;
  finishedAt?: number;
}

export interface ProjectData {
  project: ProjectRecord;
  agents: AgentProfile[];
  columns: WorkflowColumn[];
  tasks: TaskRecord[];
  todos: TodoRecord[];
  runs: RunRecord[];
  approvals: EditApprovalRecord[];
}

export interface EditApprovalRecord {
  id: number;
  runId: number;
  editId: string;
  path: string;
  content: string;
  editType: string;
  approvalKind: "fileChange" | "commandExecution";
  status: "pending" | "approved" | "denied" | "expired";
  requestedAt: number;
  decidedAt?: number;
}

export type WayfinderTicketType = "grill" | "research" | "prototype" | "code" | "user_action";
export type WayfinderTicketStatus = "blocked" | "frontier" | "active" | "resolved" | "retired";

export interface WayfinderMap {
  id: number;
  projectId: number;
  title: string;
  idea: string;
  destination: string;
  notes: string;
  codeTaskStatus: string;
  status: "draft" | "active" | "completing" | "archived";
  createdAt: number;
  updatedAt: number;
}

export interface WayfinderTicket {
  id: number;
  mapId: number;
  title: string;
  question: string;
  information: string;
  outcome: string;
  ticketType: WayfinderTicketType;
  status: WayfinderTicketStatus;
  dependsOn: number[];
  linkedTaskId?: number;
  linkedRunId?: number;
  briefVersion: number;
  resolution: string;
  resultNote: string;
  modelRole: string;
  effort: string;
  budgetCents?: number;
  createdAt: number;
  updatedAt: number;
}

export interface WayfinderQuestionOption {
  id: string;
  label: string;
  description: string;
  pros: string;
  cons: string;
  recommended: boolean;
}

export interface WayfinderQuestion {
  id: number;
  ticketId: number;
  round: number;
  title: string;
  prompt: string;
  cardinality: "single" | "multiple";
  options: WayfinderQuestionOption[];
  answers: string[];
  customAnswer: string;
  answeredAt?: number;
  createdAt: number;
}

export interface WayfinderEvent {
  id: number;
  mapId: number;
  ticketId?: number;
  kind: string;
  actor: string;
  payload: unknown;
  createdAt: number;
}

export interface WayfinderMapData {
  map: WayfinderMap;
  tickets: WayfinderTicket[];
  questions: WayfinderQuestion[];
  events: WayfinderEvent[];
}

export interface CreateWayfinderTicketInput {
  mapId: number;
  title: string;
  question?: string;
  information?: string;
  outcome?: string;
  ticketType: WayfinderTicketType;
  dependsOn?: number[];
  modelRole?: string;
  effort?: string;
  budgetCents?: number;
}

export interface GitFileStatus {
  path: string;
  indexStatus: string;
  worktreeStatus: string;
}

export interface GitStatus { branch?: string; files: GitFileStatus[] }
export interface GitDiff { diff: string; truncated: boolean }

export interface EngineSession {
  id: number;
  projectPath: string;
  sourceProjectPath: string;
  worktreePath: string;
  mode: string;
  pid?: number;
  running: boolean;
  outcome: string;
}

export interface RunEventRecord {
  id: number;
  runId: number;
  protocolSequence: number;
  kind: string;
  payload: unknown;
  raw: string;
  createdAt: number;
}

export interface RunEventBatch {
  run: RunRecord;
  events: RunEventRecord[];
  nextEventId: number;
}

export interface RunWorktreeInspection {
  run: RunRecord;
  status: GitStatus;
  diff: GitDiff;
}

export interface WorktreeActionResult {
  run: RunRecord;
  commitOid?: string;
  cleanupPending: boolean;
}

export interface SkillSummary { name: string; path: string; description: string }
export interface SkillContent { path: string; content: string }
export interface AttachmentSelection { path: string; name: string; kind: "image" | "text" }
export interface ModelOption { provider: string; model: string; tier: string }
export interface ModelCatalog {
  models: ModelOption[];
  activeProvider: string;
  activeModel: string;
  modelMode: string;
  connectedProviders: string[];
}
export interface UpsertProviderRequest {
  name: string;
  baseUrl: string;
  apiFormat: "openai" | "anthropic";
  envKey: string;
  apiKey: string;
  models: string[];
}

export const isDesktop = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function desktopInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop()) throw new Error("Rubyn Harness requires the desktop runtime.");
  return invoke<T>(command, args);
}

export const harnessBridge = {
  chooseProjectFolder: async () => {
    if (!isDesktop()) throw new Error("Rubyn Harness requires the desktop runtime.");
    const selected = await open({ directory: true, multiple: false, title: "Choose a Ruby or Rails project" });
    return typeof selected === "string" ? selected : null;
  },
  chooseAttachments: async (): Promise<AttachmentSelection[]> => {
    if (!isDesktop()) throw new Error("Rubyn Harness requires the desktop runtime.");
    const selected = await open({
      directory: false,
      multiple: true,
      title: "Attach images, code, or text files",
      filters: [{ name: "Images and text", extensions: ["png", "jpg", "jpeg", "gif", "webp", "rb", "rake", "erb", "haml", "slim", "js", "jsx", "ts", "tsx", "css", "scss", "html", "md", "txt", "json", "yml", "yaml", "xml", "sql", "sh", "toml", "lock", "env", "csv"] }],
    });
    const paths = Array.isArray(selected) ? selected : typeof selected === "string" ? [selected] : [];
    const imageExtensions = new Set(["png", "jpg", "jpeg", "gif", "webp"]);
    return paths.map((path) => {
      const name = path.split(/[\\/]/).pop() || "attachment";
      const extension = name.includes(".") ? name.split(".").pop()!.toLowerCase() : "";
      return { path, name, kind: imageExtensions.has(extension) ? "image" : "text" };
    });
  },
  engineHealth: () => desktopInvoke<EngineInfo>("engine_health"),
  listModels: () => desktopInvoke<ModelCatalog>("list_models"),
  upsertProvider: (request: UpsertProviderRequest) => desktopInvoke<ModelCatalog>("upsert_provider", { request }),
  startCodexLogin: () => desktopInvoke<void>("start_codex_login"),
  getChiselMode: () => desktopInvoke<"off" | "lite" | "full" | "ultra">("get_chisel_mode"),
  setChiselEnabled: (enabled: boolean) => desktopInvoke<"off" | "full">("set_chisel_enabled", { enabled }),
  appState: () => desktopInvoke<LocalAppState>("get_app_state"),
  saveAppState: (state: LocalAppState) => desktopInvoke<LocalAppState>("save_app_state", { state }),
  inspectProject: (projectPath: string) => desktopInvoke<ProjectSummary>("inspect_project", { projectPath }),
  trustProject: (projectPath: string) => desktopInvoke<LocalAppState>("trust_project", { projectPath }),
  projectData: (projectPath: string) => desktopInvoke<ProjectData>("get_project_data", { projectPath }),
  gitStatus: (projectPath: string) => desktopInvoke<GitStatus>("get_git_status", { projectPath }),
  launchPrompt: (projectPath: string, prompt: string, attachments: AttachmentSelection[] = [], model?: ModelOption) => desktopInvoke<EngineSession>("launch_engine", {
    request: { projectPath, mode: { prompt: { prompt } }, yolo: false, ...(attachments.length ? { attachments: attachments.map(({ path }) => ({ path })) } : {}), ...(model ? { provider: model.provider, model: model.model } : {}) },
  }),
  sendRunMessage: (runId: number, message: string, attachments: AttachmentSelection[] = [], model?: ModelOption) => desktopInvoke<EngineSession>("send_run_message", {
    request: { runId, message, ...(attachments.length ? { attachments: attachments.map(({ path }) => ({ path })) } : {}), ...(model ? { provider: model.provider, model: model.model } : {}) },
  }),
  listRuns: (projectPath?: string) => desktopInvoke<RunRecord[]>("list_runs", { projectPath }),
  updateConversation: (request: { id: number; title?: string; pinned?: boolean; archived?: boolean }) => desktopInvoke<RunRecord>("update_conversation", { request }),
  pollRunEvents: (runId: number, afterEventId?: number) => desktopInvoke<RunEventBatch>("poll_run_events", {
    runId,
    afterEventId,
  }),
  inspectRunWorktree: (runId: number) => desktopInvoke<RunWorktreeInspection>("inspect_run_worktree", { runId }),
  integrateRun: (runId: number) => desktopInvoke<WorktreeActionResult>("integrate_run", { runId }),
  discardRun: (runId: number) => desktopInvoke<WorktreeActionResult>("discard_run", { runId }),
  stop: (sessionId: number) => desktopInvoke<void>("stop_engine", { sessionId }),
  answerEngineQuestion: (runId: number, requestId: string | number, answer: unknown) => desktopInvoke<void>("answer_engine_question", { runId, requestId, answer }),
  resolveEditApproval: (runId: number, editId: string, accepted: boolean) => desktopInvoke<EditApprovalRecord>("resolve_edit_approval", { request: { runId, editId, accepted } }),
  createTask: (projectPath: string, title: string, detail = "", outcome = "", dependsOn: number[] = []) => desktopInvoke<TaskRecord>("create_project_task", {
    request: { projectPath, title, detail, outcome, status: "queued", dependsOn },
  }),
  updateTask: (id: number, status?: TaskStatus, assignedRunId?: number | null) => desktopInvoke<TaskRecord>("update_project_task", {
    request: { id, ...(status ? { status } : {}), ...(assignedRunId !== undefined ? { assignedRunId } : {}) },
  }),
  createWorkflowColumn: (projectPath: string, name: string) => desktopInvoke<WorkflowColumn>("create_workflow_column", { request: { projectPath, name } }),
  updateWorkflowColumn: (id: number, name?: string, position?: number, agentId?: number | null) => desktopInvoke<WorkflowColumn>("update_workflow_column", { request: { id, name, position, ...(agentId !== undefined ? { agentId } : {}) } }),
  deleteWorkflowColumn: (id: number, moveTasksTo: number) => desktopInvoke<void>("delete_workflow_column", { request: { id, moveTasksTo } }),
  createAgentProfile: (projectPath: string, name: string, role: string, instructions: string) => desktopInvoke<AgentProfile>("create_agent_profile", { request: { projectPath, name, role, instructions } }),
  updateAgentProfile: (id: number, name?: string, role?: string, instructions?: string) => desktopInvoke<AgentProfile>("update_agent_profile", { request: { id, name, role, instructions } }),
  deleteAgentProfile: (id: number) => desktopInvoke<void>("delete_agent_profile", { id }),
  createTodo: (projectPath: string, title: string) => desktopInvoke<TodoRecord>("create_project_todo", {
    request: { projectPath, title, owner: "You", status: "queued" },
  }),
  updateTodo: (id: number, status?: WorkflowStatus, assignedRunId?: number | null) => desktopInvoke<TodoRecord>("update_project_todo", {
    request: { id, ...(status ? { status } : {}), ...(assignedRunId !== undefined ? { assignedRunId } : {}) },
  }),
  listBundledSkills: () => desktopInvoke<SkillSummary[]>("list_bundled_skills"),
  createProjectSkill: (projectPath: string, name: string, content: string) => desktopInvoke<SkillSummary>("create_project_skill", {
    request: { projectPath, name, content },
  }),
  listProjectSkills: (projectPath: string) => desktopInvoke<SkillSummary[]>("list_project_skills", { projectPath }),
  readSkill: (path: string, projectPath?: string) => desktopInvoke<SkillContent>("read_skill", { path, projectPath }),
  listWayfinderMaps: (projectPath: string) => desktopInvoke<WayfinderMap[]>("list_wayfinder_maps", { projectPath }),
  getWayfinderMap: (mapId: number) => desktopInvoke<WayfinderMapData>("get_wayfinder_map", { mapId }),
  createWayfinderMap: (projectPath: string, idea: string, codeTaskStatus: string) => desktopInvoke<WayfinderMapData>("create_wayfinder_map", { request: { projectPath, idea, codeTaskStatus } }),
  updateWayfinderMap: (mapId: number, changes: { title?: string; destination?: string; notes?: string }) => desktopInvoke<WayfinderMapData>("update_wayfinder_map", { mapId, ...changes }),
  createWayfinderTicket: (request: CreateWayfinderTicketInput) => desktopInvoke<WayfinderTicket>("create_wayfinder_ticket", { request: { ...request, dependsOn: request.dependsOn || [] } }),
  updateWayfinderTicket: (request: Partial<CreateWayfinderTicketInput> & { id: number }) => desktopInvoke<WayfinderTicket>("update_wayfinder_ticket", { request }),
  submitWayfinderAnswers: (ticketId: number, answers: { questionId: number; answers: string[]; customAnswer: string }[]) => desktopInvoke<WayfinderMapData>("submit_wayfinder_answers", { ticketId, answers }),
  activateWayfinderMap: (mapId: number) => desktopInvoke<WayfinderMapData>("activate_wayfinder_map", { mapId }),
  resolveWayfinderTicket: (ticketId: number, resolution: string, addTickets: CreateWayfinderTicketInput[] = [], retireTicketIds: number[] = []) => desktopInvoke<WayfinderMapData>("resolve_wayfinder_ticket", { request: { ticketId, resolution, addTickets, retireTicketIds } }),
  completeWayfinderUserAction: (ticketId: number, resultNote: string) => desktopInvoke<WayfinderMapData>("complete_wayfinder_user_action", { ticketId, resultNote }),
  linkWayfinderRun: (ticketId: number, runId: number) => desktopInvoke<WayfinderTicket>("link_wayfinder_run", { ticketId, runId }),
  retireWayfinderTicket: (ticketId: number) => desktopInvoke<WayfinderMapData>("retire_wayfinder_ticket", { ticketId }),
  archiveWayfinderMap: (mapId: number) => desktopInvoke<WayfinderMapData>("archive_wayfinder_map", { mapId }),
  listWayfinderBlockers: (projectPath: string) => desktopInvoke<WayfinderTicket[]>("list_wayfinder_blockers", { projectPath }),
};
