// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  LocalAppState,
  ProjectData,
  ProjectSummary,
  RunRecord,
  TaskRecord,
  TaskStatus,
  TodoRecord,
  WorkflowStatus,
  WorkflowColumn,
} from "./bridge";

const native = vi.hoisted(() => ({
  desktop: true,
  chooseProjectFolder: vi.fn(),
  chooseAttachments: vi.fn(),
  engineHealth: vi.fn(),
  listModels: vi.fn(),
  upsertProvider: vi.fn(),
  startCodexLogin: vi.fn(),
  revokeProvider: vi.fn(),
  createSanitizedDiagnostics: vi.fn(),
  clearLocalData: vi.fn(),
  getChiselMode: vi.fn(),
  setChiselEnabled: vi.fn(),
  appState: vi.fn(),
  saveAppState: vi.fn(),
  inspectProject: vi.fn(),
  trustProject: vi.fn(),
  projectData: vi.fn(),
  gitStatus: vi.fn(),
  launchPrompt: vi.fn(),
  sendRunMessage: vi.fn(),
  listRuns: vi.fn(),
  updateConversation: vi.fn(),
  pollRunEvents: vi.fn(),
  inspectRunWorktree: vi.fn(),
  integrateRun: vi.fn(),
  discardRun: vi.fn(),
  retryRunCleanup: vi.fn(),
  stop: vi.fn(),
  resolveEditApproval: vi.fn(),
  createTask: vi.fn(),
  updateTask: vi.fn(),
  createWorkflowColumn: vi.fn(),
  updateWorkflowColumn: vi.fn(),
  deleteWorkflowColumn: vi.fn(),
  createAgentProfile: vi.fn(),
  updateAgentProfile: vi.fn(),
  deleteAgentProfile: vi.fn(),
  createTodo: vi.fn(),
  updateTodo: vi.fn(),
  listBundledSkills: vi.fn(),
  listProjectSkills: vi.fn(),
  createProjectSkill: vi.fn(),
  readSkill: vi.fn(),
}));

vi.mock("./bridge", async () => {
  const actual = await vi.importActual<typeof import("./bridge")>("./bridge");
  return {
    ...actual,
    isDesktop: () => native.desktop,
    harnessBridge: {
      chooseProjectFolder: native.chooseProjectFolder,
      chooseAttachments: native.chooseAttachments,
      engineHealth: native.engineHealth,
      listModels: native.listModels,
      upsertProvider: native.upsertProvider,
      startCodexLogin: native.startCodexLogin,
      revokeProvider: native.revokeProvider,
      createSanitizedDiagnostics: native.createSanitizedDiagnostics,
      clearLocalData: native.clearLocalData,
      getChiselMode: native.getChiselMode,
      setChiselEnabled: native.setChiselEnabled,
      appState: native.appState,
      saveAppState: native.saveAppState,
      inspectProject: native.inspectProject,
      trustProject: native.trustProject,
      projectData: native.projectData,
      gitStatus: native.gitStatus,
      launchPrompt: native.launchPrompt,
      sendRunMessage: native.sendRunMessage,
      listRuns: native.listRuns,
      updateConversation: native.updateConversation,
      pollRunEvents: native.pollRunEvents,
      inspectRunWorktree: native.inspectRunWorktree,
      integrateRun: native.integrateRun,
      discardRun: native.discardRun,
      retryRunCleanup: native.retryRunCleanup,
      stop: native.stop,
      resolveEditApproval: native.resolveEditApproval,
      createTask: native.createTask,
      updateTask: native.updateTask,
      createWorkflowColumn: native.createWorkflowColumn,
      updateWorkflowColumn: native.updateWorkflowColumn,
      deleteWorkflowColumn: native.deleteWorkflowColumn,
      createAgentProfile: native.createAgentProfile,
      updateAgentProfile: native.updateAgentProfile,
      deleteAgentProfile: native.deleteAgentProfile,
      createTodo: native.createTodo,
      updateTodo: native.updateTodo,
      listBundledSkills: native.listBundledSkills,
      listProjectSkills: native.listProjectSkills,
      createProjectSkill: native.createProjectSkill,
      readSkill: native.readSkill,
    },
  };
});

import { App } from "./App";
import { useHarnessStore } from "./store";

const project: ProjectSummary = {
  path: "/work/ledger",
  name: "ledger",
  gitRoot: "/work/ledger",
  isRuby: true,
  isRails: true,
  hasRubynInstructions: true,
};

const emptyState: LocalAppState = {
  preferences: {
    defaultModel: "openai/gpt-5.4",
    parallelLimit: 3,
    autoCompaction: true,
    yoloEnabled: false,
  },
  recentProjects: [],
  onboardingVersion: 1,
  trustedProjectPaths: [project.path],
};

const connectedModel = { provider: "minimax", model: "MiniMax-M3", tier: "top" };
const connectedCatalog = {
  models: [connectedModel],
  activeProvider: connectedModel.provider,
  activeModel: connectedModel.model,
  modelMode: "manual",
  connectedProviders: [connectedModel.provider],
};

let data: ProjectData;
let nextTaskId: number;
let nextTodoId: number;
const columns: WorkflowColumn[] = [
  { id: 1, projectId: 1, key: "backlog", name: "Backlog", position: 0, terminal: false },
  { id: 2, projectId: 1, key: "planning", name: "Planning", position: 1, terminal: false, agentId: 1 },
  { id: 3, projectId: 1, key: "implementing", name: "Implementing", position: 2, terminal: false, agentId: 2 },
  { id: 4, projectId: 1, key: "review", name: "Review", position: 3, terminal: false, agentId: 3 },
  { id: 5, projectId: 1, key: "done", name: "Done", position: 4, terminal: true },
];

function resetStore() {
  useHarnessStore.setState({
    view: "control",
    project: undefined,
    projectData: undefined,
    appState: undefined,
    modelCatalog: undefined,
    skills: [],
    globalRuns: [],
    selectedRunId: undefined,
    activeConversationId: undefined,
    newConversationDraft: "",
    newConversationTaskId: undefined,
    conversationDrafts: {},
    newConversationAttachments: [],
    conversationAttachments: {},
    runEvents: {},
    eventCursors: {},
    commandOpen: false,
    mobileOpen: false,
    reducedMotion: false,
    engineState: "checking",
    engineDetail: "Checking bundled Rubyn Code…",
    loading: true,
    notice: "",
  });
}

function makeTask(id: number, title: string): TaskRecord {
  return {
    id,
    projectId: 1,
    title,
    detail: "",
    outcome: "",
    status: "backlog",
    dependsOn: [],
    ready: true,
    createdAt: 1,
    updatedAt: 1,
  };
}

function makeTodo(id: number, title: string): TodoRecord {
  return { id, projectId: 1, title, owner: "You", status: "queued", createdAt: 1, updatedAt: 1 };
}

function cancelledRun(): RunRecord {
  return {
    id: 31,
    projectId: 1,
    sourceProjectPath: project.path,
    worktreePath: "/native/worktrees/run-31/workspace",
    baseCommit: "0123456789abcdef",
    prompt: "Add billing export",
    mode: "prompt",
    pid: undefined,
    running: false,
    outcome: "cancelled",
    lifecycle: "retained",
    stdout: "",
    stderr: "",
    createdAt: 1,
    updatedAt: 2,
    finishedAt: 2,
  };
}

function readyIntegration() {
  return {
    sourceHead: "0123456789abcdef",
    recordedBase: "0123456789abcdef",
    sourceClean: true,
    sourceMatchesBase: true,
    blockers: [],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  native.desktop = true;
  nextTaskId = 100;
  nextTodoId = 200;
  data = {
    project: { id: 1, path: project.path, name: project.name, createdAt: 1, updatedAt: 1 },
    agents: [
      { id: 1, projectId: 1, name: "Planner", role: "planning", instructions: "Plan it.", createdAt: 1, updatedAt: 1 },
      { id: 2, projectId: 1, name: "Builder", role: "implementation", instructions: "Build it.", createdAt: 1, updatedAt: 1 },
      { id: 3, projectId: 1, name: "Reviewer", role: "review", instructions: "Review it.", createdAt: 1, updatedAt: 1 },
    ],
    columns: columns.map((column) => ({ ...column })),
    tasks: [],
    todos: [],
    runs: [],
    approvals: [],
  };
  resetStore();

  native.engineHealth.mockResolvedValue({
    available: true,
    healthy: true,
    source: "bundled",
    executable: "/app/engine/rubyn-code",
    version: "0.1.0",
  });
  native.listModels.mockResolvedValue(connectedCatalog);
  native.revokeProvider.mockResolvedValue({ ...connectedCatalog, connectedProviders: [] });
  native.createSanitizedDiagnostics.mockResolvedValue({ path: "/app-data/diagnostics/rubyn-diagnostics.json", createdAt: 42 });
  native.clearLocalData.mockResolvedValue({ appState: { ...emptyState, onboardingVersion: 0, recentProjects: [], trustedProjectPaths: [] }, cleanupPending: false, retainedPaths: [] });
  native.getChiselMode.mockRejectedValue(new Error("Chisel unavailable in baseline tests"));
  native.appState.mockResolvedValue(emptyState);
  native.chooseProjectFolder.mockResolvedValue(null);
  native.chooseAttachments.mockResolvedValue([]);
  native.resolveEditApproval.mockResolvedValue({});
  native.saveAppState.mockImplementation(async (state: LocalAppState) => state);
  native.trustProject.mockImplementation(async (path: string) => ({
    ...emptyState,
    trustedProjectPaths: [path, ...(emptyState.trustedProjectPaths || []).filter((trusted) => trusted !== path)],
  }));
  native.listBundledSkills.mockResolvedValue([]);
  native.listProjectSkills.mockResolvedValue([]);
  native.readSkill.mockResolvedValue({ path: "rails.md", content: "# Rails\n" });
  native.listRuns.mockImplementation(async () => [...data.runs]);
  native.updateConversation.mockImplementation(async (request: { id: number; title?: string; pinned?: boolean; archived?: boolean }) => {
    const conversation = data.runs.find((run) => run.id === request.id)!;
    if (request.title) conversation.title = request.title;
    if (request.pinned !== undefined) conversation.pinned = request.pinned;
    if (request.archived !== undefined) {
      conversation.archivedAt = request.archived ? Date.now() : undefined;
      if (request.archived) conversation.pinned = false;
    }
    return { ...conversation };
  });
  native.inspectProject.mockResolvedValue(project);
  native.projectData.mockImplementation(async () => ({
    ...data,
    agents: [...data.agents],
    tasks: [...data.tasks],
    todos: [...data.todos],
    runs: [...data.runs],
    approvals: [...data.approvals],
  }));
  native.pollRunEvents.mockImplementation(async (runId: number, afterEventId?: number) => ({
    run: data.runs.find((run) => run.id === runId),
    events: [],
    nextEventId: afterEventId ?? 0,
  }));
  native.createTask.mockImplementation(async (_path: string, title: string, detail = "", outcome = "", dependsOn: number[] = []) => {
    const task = { ...makeTask(++nextTaskId, title), detail, outcome, dependsOn, ready: true };
    data.tasks.push(task);
    return task;
  });
  native.updateTask.mockImplementation(async (id: number, status?: TaskStatus, assignedRunId?: number | null) => {
    const task = data.tasks.find((candidate) => candidate.id === id)!;
    if (status) task.status = status;
    if (assignedRunId !== undefined) task.assignedRunId = assignedRunId ?? undefined;
    return task;
  });
  native.createWorkflowColumn.mockImplementation(async (_path: string, name: string) => {
    const column = { id: data.columns.length + 10, projectId: 1, key: name.toLowerCase(), name, position: data.columns.length, terminal: false };
    data.columns.push(column);
    return column;
  });
  native.updateWorkflowColumn.mockImplementation(async (id: number, name?: string, position?: number, agentId?: number | null) => {
    const column = data.columns.find((candidate) => candidate.id === id)!;
    if (name) column.name = name;
    if (agentId !== undefined) column.agentId = agentId ?? undefined;
    if (position !== undefined) column.position = position;
    data.columns.sort((left, right) => left.position - right.position);
    return column;
  });
  native.deleteWorkflowColumn.mockImplementation(async (id: number) => { data.columns = data.columns.filter((column) => column.id !== id); });
  native.createAgentProfile.mockImplementation(async (_path: string, name: string, role: string, instructions: string) => {
    const agent = { id: data.agents.length + 10, projectId: 1, name, role, instructions, createdAt: 1, updatedAt: 1 };
    data.agents.push(agent);
    return agent;
  });
  native.updateAgentProfile.mockImplementation(async (id: number, name?: string, role?: string, instructions?: string) => {
    const agent = data.agents.find((candidate) => candidate.id === id)!;
    if (name !== undefined) agent.name = name;
    if (role !== undefined) agent.role = role;
    if (instructions !== undefined) agent.instructions = instructions;
    return agent;
  });
  native.deleteAgentProfile.mockImplementation(async (id: number) => { data.agents = data.agents.filter((agent) => agent.id !== id); });
  native.createTodo.mockImplementation(async (_path: string, title: string) => {
    const todo = makeTodo(++nextTodoId, title);
    data.todos.push(todo);
    return todo;
  });
  native.updateTodo.mockImplementation(async (id: number, status?: WorkflowStatus, assignedRunId?: number | null) => {
    const todo = data.todos.find((candidate) => candidate.id === id)!;
    if (status) todo.status = status;
    if (assignedRunId !== undefined) todo.assignedRunId = assignedRunId ?? undefined;
    return todo;
  });
  native.createProjectSkill.mockImplementation(async (_path: string, name: string, content: string) => ({
    name,
    path: `${project.path}/.rubyn-code/skills/harness/${name.toLowerCase().replaceAll(" ", "-")}.md`,
    description: content.split("\n")[0],
  }));
});

afterEach(cleanup);

describe("native product flow", () => {
  it("gates browser builds without calling any native operation", () => {
    native.desktop = false;
    render(<App />);

    expect(screen.getByRole("heading", { name: /open the desktop app/i })).toBeInTheDocument();
    expect(screen.getByText(/no fixture data or simulated runtime/i)).toBeInTheDocument();
    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
    expect(native.engineHealth).not.toHaveBeenCalled();
    expect(native.appState).not.toHaveBeenCalled();
  });

  it("connects a familiar model service by asking only for its key", async () => {
    const catalog = {
      models: [
        { provider: "anthropic", model: "claude-sonnet-5", tier: "mid" },
        { provider: "openai", model: "gpt-5.4", tier: "top" },
      ],
      activeProvider: "anthropic",
      activeModel: "claude-sonnet-5",
      modelMode: "auto",
      connectedProviders: [],
    };
    native.listModels.mockResolvedValue(catalog);
    native.upsertProvider.mockResolvedValue({ ...catalog, connectedProviders: ["anthropic"] });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /Models & accounts/ }));
    expect(await screen.findByRole("heading", { name: "Models & accounts" })).toBeInTheDocument();
    fireEvent.click(screen.getByText("Anthropic", { exact: true }).closest("button")!);

    expect(screen.queryByLabelText("Web address")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Secret key"), { target: { value: "sk-ant-test" } });
    fireEvent.click(screen.getByRole("button", { name: "Save and connect" }));

    await waitFor(() => expect(native.upsertProvider).toHaveBeenCalledWith({
      name: "anthropic",
      baseUrl: "https://api.anthropic.com/v1",
      apiFormat: "anthropic",
      envKey: "",
      apiKey: "sk-ant-test",
      models: ["claude-fable-5", "claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"],
    }));
    expect(await screen.findByText("Connected")).toBeInTheDocument();
  });

  it("requires a named confirmation before revoking a connected provider", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /Models & accounts/ }));
    fireEvent.click(screen.getByRole("button", { name: "MiniMax" }));

    fireEvent.click(screen.getByRole("button", { name: "Revoke access" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Revoke minimax access now?");
    fireEvent.click(screen.getByRole("button", { name: "Revoke minimax" }));

    await waitFor(() => expect(native.revokeProvider).toHaveBeenCalledWith("minimax"));
    expect(await screen.findByText(/access was revoked/i)).toBeInTheDocument();
  });

  it("lists configured custom providers and revokes the selected provider name", async () => {
    native.listModels.mockResolvedValue({
      ...connectedCatalog,
      models: [{ provider: "rubyn-phase5-smoke", model: "smoke-model", tier: "custom" }],
      activeProvider: "rubyn-phase5-smoke",
      activeModel: "smoke-model",
      connectedProviders: ["rubyn-phase5-smoke"],
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /Models & accounts/ }));
    fireEvent.click(screen.getByRole("button", { name: "rubyn-phase5-smoke" }));

    expect(screen.getByRole("heading", { name: "rubyn-phase5-smoke" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Revoke access" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Revoke rubyn-phase5-smoke access now?");
    fireEvent.click(screen.getByRole("button", { name: "Revoke rubyn-phase5-smoke" }));

    await waitFor(() => expect(native.revokeProvider).toHaveBeenCalledWith("rubyn-phase5-smoke"));
  });

  it("creates a sanitized diagnostic report and shows its exact path", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Projects" }));
    fireEvent.click(await screen.findByRole("button", { name: "Create diagnostic report" }));

    await waitFor(() => expect(native.createSanitizedDiagnostics).toHaveBeenCalledTimes(1));
    expect(screen.getByText("/app-data/diagnostics/rubyn-diagnostics.json")).toBeInTheDocument();
    expect(screen.getByText(/not prompts, source, paths, or credentials/i)).toBeInTheDocument();
  });

  it("requires destructive confirmation before clearing local data and returns to onboarding", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Projects" }));
    fireEvent.click(await screen.findByRole("button", { name: "Remove local data…" }));

    expect(screen.getByRole("alert")).toHaveTextContent("Remove all Rubyn Harness local workspace data?");
    expect(screen.getByText(/Source repositories and provider accounts are not deleted/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Remove local data" }));

    await waitFor(() => expect(native.clearLocalData).toHaveBeenCalledTimes(1));
    expect(await screen.findByRole("heading", { name: /Your repository stays under your control/i })).toBeInTheDocument();
  });

  it("migrates a saved OpenAI 5.4 choice to the matching 5.6 tier", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      preferences: { ...emptyState.preferences, defaultModel: "openai/gpt-5.4" },
      recentProjects: [{ path: project.path, name: project.name }],
    });
    native.listModels.mockResolvedValue({
      models: [
        { provider: "anthropic", model: "claude-sonnet-5", tier: "mid" },
        { provider: "openai", model: "gpt-5.6-sol", tier: "top" },
        { provider: "openai", model: "gpt-5.6-terra", tier: "mid" },
      ],
      activeProvider: "anthropic",
      activeModel: "claude-sonnet-5",
      modelMode: "auto",
      connectedProviders: ["anthropic", "openai"],
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    expect(await screen.findByRole("combobox", { name: "Model for new conversations" })).toHaveValue("openai::gpt-5.6-sol");
  });

  it("falls back to a connected provider and disables disconnected models", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      preferences: { ...emptyState.preferences, defaultModel: "openai/gpt-5.4" },
      recentProjects: [{ path: project.path, name: project.name }],
    });
    native.listModels.mockResolvedValue({
      ...connectedCatalog,
      models: [
        connectedModel,
        { provider: "openai", model: "gpt-5.6-sol", tier: "top" },
      ],
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    expect(await screen.findByRole("combobox", { name: "Model for new conversations" })).toHaveValue("minimax::MiniMax-M3");
    expect(screen.getByRole("option", { name: "openai / gpt-5.6-sol · connect first" })).toBeDisabled();
  });

  it("turns Rubyn Chisel on from project runtime settings", async () => {
    native.getChiselMode.mockResolvedValue("off");
    native.setChiselEnabled.mockResolvedValue("full");

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Projects" }));
    const chisel = await screen.findByRole("switch", { name: "Rubyn Chisel" });
    expect(chisel).toHaveAttribute("aria-checked", "false");

    fireEvent.click(chisel);

    await waitFor(() => expect(native.setChiselEnabled).toHaveBeenCalledWith(true));
    expect(chisel).toHaveAttribute("aria-checked", "true");
    expect(screen.getByText(/smallest change that works/i)).toBeInTheDocument();
  });

  it("renames, pins, archives, and restores durable conversations", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    data.runs = [{ ...cancelledRun(), title: "Billing export", pinned: false, background: false }];

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByLabelText("Actions for Billing export"));
    fireEvent.click(screen.getByRole("button", { name: "Pin" }));
    await waitFor(() => expect(native.updateConversation).toHaveBeenCalledWith({ id: 31, pinned: true }));

    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Conversation name" }), { target: { value: "Quarterly export" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Quarterly export")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Archive" }));
    expect(await screen.findByText("Archived (1)")).toBeInTheDocument();
    expect(screen.queryByLabelText("Actions for Quarterly export")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Archived (1)"));
    fireEvent.click(screen.getByRole("button", { name: "Restore" }));
    expect(await screen.findByLabelText("Actions for Quarterly export")).toBeInTheDocument();
  });

  it("counts only task-linked background execution as runs", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    data.runs = [
      { ...cancelledRun(), id: 31, title: "General chat", background: false },
      { ...cancelledRun(), id: 32, title: "Implement export", background: true },
    ];

    render(<App />);

    expect(await screen.findByText("1 recorded run")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Recent runs" }).closest("article")).toHaveTextContent("Implement export");
    expect(screen.getByRole("heading", { name: "Recent runs" }).closest("article")).not.toHaveTextContent("General chat");
  });

  it("renders a truthful empty project state and provides keyboard navigation", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" })).toBeInTheDocument();
    expect(screen.getByText(/no sample repository or fixture state is loaded/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Wayfinder" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Workgraph" })).not.toBeInTheDocument();

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(screen.getByRole("dialog", { name: "Command palette" })).toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: "Search commands" }), { target: { value: "projects" } });
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Search commands" }), { key: "Enter" });

    expect(await screen.findByRole("heading", { name: "Projects" })).toBeInTheDocument();
    expect(screen.getByText("No recent projects.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose project folder" })).toBeEnabled();

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.getByRole("dialog", { name: "Command palette" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Command palette" })).not.toBeInTheDocument();
  });

  it("blocks the empty-project call to action when the native runtime is unavailable", async () => {
    native.engineHealth.mockResolvedValue({
      available: true,
      healthy: false,
      source: "bundled",
      executable: "/app/engine/rubyn-code",
      detail: "Bundled Rubyn Code requires Ruby 4.0.2+ with its runtime gems.",
    });

    render(<App />);

    expect(await screen.findByRole("heading", { name: "Finish Rubyn runtime setup" })).toBeInTheDocument();
    expect(screen.getByText(/requires Ruby 4\.0\.2\+/i)).toBeInTheDocument();
    expect(screen.getByText("gem install rubyn-code")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Choose project/ })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Open project runtime" }));
    expect(await screen.findByRole("heading", { name: "Projects" })).toBeInTheDocument();
  });

  it("disables new conversations when a remembered project has no runtime", async () => {
    native.engineHealth.mockResolvedValue({
      available: true,
      healthy: false,
      source: "bundled",
      executable: "/app/engine/rubyn-code",
      detail: "Bundled Rubyn Code requires Ruby 4.0.2+ with its runtime gems.",
    });
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Talk to Rubyn" }));
    expect(await screen.findByRole("heading", { name: "What should Rubyn work on?" })).toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Make a change" } });
    expect(screen.getByRole("button", { name: "Start conversation" })).toBeDisabled();
  });

  it("treats cleanup-pending worktree dispositions as terminal", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [{ ...cancelledRun(), lifecycle: "discard_cleanup_pending" }];
    native.retryRunCleanup.mockImplementation(async () => {
      const run = { ...cancelledRun(), lifecycle: "discarded" };
      data.runs = [run];
      return { run, cleanupPending: false };
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Review" }));

    expect(await screen.findByText("This worktree is discarded · cleanup pending.")).toBeInTheDocument();
    expect(native.inspectRunWorktree).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Integrate" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Discard" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry cleanup" }));
    await waitFor(() => expect(native.retryRunCleanup).toHaveBeenCalledWith(31));
    expect(await screen.findByText("This worktree is discarded.")).toBeInTheDocument();
  });

  it("confirms destructive review cleanup inline", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    data.runs = [cancelledRun()];
    native.inspectRunWorktree.mockResolvedValue({
      run: cancelledRun(),
      status: { branch: "HEAD", files: [] },
      diff: { diff: "", truncated: false },
      readiness: readyIntegration(),
    });
    native.discardRun.mockResolvedValue({ runId: 31, disposition: "discarded", cleanupPending: false });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Review" }));
    expect(await screen.findByText(/no changes to integrate/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Integrate" })).toBeDisabled();
    fireEvent.click(await screen.findByRole("button", { name: "Discard" }));

    expect(screen.getByText("Discard this worktree?")).toBeInTheDocument();
    expect(screen.getAllByText(/0 changed files/)).toHaveLength(2);
    expect(native.discardRun).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Keep worktree" }));
    expect(screen.queryByText("Discard this worktree?")).not.toBeInTheDocument();
  });

  it("shows source drift and blocks integration before confirmation", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    data.runs = [cancelledRun()];
    native.inspectRunWorktree.mockResolvedValue({
      run: cancelledRun(),
      status: { branch: "HEAD", files: [{ path: "app/models/account.rb", indexStatus: " ", worktreeStatus: "M" }] },
      diff: { diff: "diff --git a/app/models/account.rb b/app/models/account.rb\n+validates :name", truncated: false },
      readiness: {
        ...readyIntegration(),
        sourceHead: "fedcba9876543210",
        sourceMatchesBase: false,
        blockers: ["The source repository moved after this worktree was created. Start a fresh run from the current source revision."],
      },
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Review" }));

    expect(await screen.findByText("Integration blocked")).toBeInTheDocument();
    expect(screen.getByText(/source repository moved/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Integrate" })).toBeDisabled();
    expect(native.integrateRun).not.toHaveBeenCalled();
  });

  it("persists project work, launches from a task, cancels, reconciles status, and inspects the diff", async () => {
    native.launchPrompt.mockImplementation(async (_path: string, prompt: string) => {
      const run: RunRecord = {
        ...cancelledRun(),
        prompt,
        pid: 9876,
        running: true,
        outcome: "running",
        finishedAt: undefined,
      };
      data.runs = [run];
      return {
        id: run.id,
        projectPath: run.worktreePath,
        sourceProjectPath: project.path,
        worktreePath: run.worktreePath,
        mode: "prompt",
        pid: run.pid,
        running: true,
        outcome: "running",
      };
    });
    native.stop.mockImplementation(async (runId: number) => {
      expect(runId).toBe(31);
      data.runs = [cancelledRun()];
    });
    native.inspectRunWorktree.mockResolvedValue({
      run: cancelledRun(),
      status: { branch: "HEAD", files: [{ path: "app/services/billing_export.rb", indexStatus: " ", worktreeStatus: "M" }] },
      diff: { diff: "diff --git a/app/services/billing_export.rb b/app/services/billing_export.rb\n+exports invoices safely", truncated: false },
      readiness: readyIntegration(),
    });

    render(<App />);
    await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" });
    fireEvent.click(screen.getByRole("button", { name: /^choose project$/i }));

    fireEvent.click(screen.getByText("Enter a path manually"));
    fireEvent.change(screen.getByRole("textbox", { name: "Local project path" }), { target: { value: project.path } });
    fireEvent.click(screen.getByRole("button", { name: "Open path" }));
    expect(await screen.findByRole("heading", { name: "What should Rubyn work on?" })).toBeInTheDocument();
    expect(native.saveAppState).toHaveBeenCalledWith(expect.objectContaining({
      recentProjects: [{ path: project.path, name: project.name }],
    }));

    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));
    fireEvent.click(screen.getByRole("button", { name: "Create task" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Task title" }), { target: { value: "Add billing export" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Task information" }), { target: { value: "Export only the active tenant." } });
    fireEvent.change(screen.getByRole("textbox", { name: "Task outcome" }), { target: { value: "Authorized users can download a scoped CSV." } });
    fireEvent.click(within(screen.getByRole("dialog", { name: "Create an engineering task" })).getByRole("button", { name: "Create task" }));
    expect((await screen.findAllByText("Add billing export")).length).toBeGreaterThan(0);

    fireEvent.change(screen.getByRole("textbox", { name: "New todo" }), { target: { value: "Verify tenant boundary" } });
    fireEvent.click(screen.getByRole("button", { name: "Add todo" }));
    expect(await screen.findByText("Verify tenant boundary")).toBeInTheDocument();
    expect(native.createTask).toHaveBeenCalledWith(project.path, "Add billing export", "Export only the active tenant.", "Authorized users can download a scoped CSV.", []);
    expect(native.createTodo).toHaveBeenCalledWith(project.path, "Verify tenant boundary");

    const todoStatus = screen.getByRole("combobox", { name: "Status for Verify tenant boundary" });
    fireEvent.change(todoStatus, { target: { value: "doing" } });
    await waitFor(() => expect(native.updateTodo).toHaveBeenLastCalledWith(201, "doing"));

    fireEvent.click(screen.getByRole("button", { name: "Start Rubyn" }));
    expect(screen.getByRole("heading", { name: "What should Rubyn work on?" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Attach a task" })).toHaveValue("101");
    expect(screen.getByRole("textbox", { name: "Prompt" })).toHaveValue("Add billing export\n\nInformation\nExport only the active tenant.\n\nExpected outcome\nAuthorized users can download a scoped CSV.");
    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    expect(screen.getByRole("combobox", { name: "Attach a task" })).toHaveValue("101");
    fireEvent.click(screen.getByRole("button", { name: "Start conversation" }));

    expect(await screen.findByRole("button", { name: "Stop turn" })).toBeInTheDocument();
    expect(native.launchPrompt).toHaveBeenCalledWith(project.path, "Add billing export\n\nInformation\nExport only the active tenant.\n\nExpected outcome\nAuthorized users can download a scoped CSV.", [], connectedModel);
    expect(native.updateTask).toHaveBeenCalledWith(101, "implementing", 31);
    fireEvent.click(screen.getByRole("button", { name: "Stop turn" }));

    await waitFor(() => expect(native.stop).toHaveBeenCalledWith(31));
    fireEvent.click(await screen.findByRole("button", { name: "Review changes" }));

    expect(await screen.findByText(/exports invoices safely/)).toBeInTheDocument();
    expect(screen.getByText("1 changed file")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Integrate" })).toBeEnabled();
    expect(native.inspectRunWorktree).toHaveBeenCalledWith(31);
    fireEvent.click(screen.getByRole("button", { name: "Integrate" }));
    expect(screen.getByText("Integrate this reviewed worktree?")).toBeInTheDocument();
    expect(screen.getByText(/1 changed file will be committed/)).toBeInTheDocument();
    expect(native.integrateRun).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByText("Integrate this reviewed worktree?")).not.toBeInTheDocument();
  });

  it("finishes a waiting conversation before making its worktree reviewable", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    data.runs = [{ ...cancelledRun(), running: true, outcome: "waiting", finishedAt: undefined }];
    native.stop.mockImplementation(async () => {
      data.runs = [{ ...cancelledRun(), outcome: "completed" }];
    });
    native.inspectRunWorktree.mockResolvedValue({
      run: { ...cancelledRun(), outcome: "completed" },
      status: { branch: "HEAD", files: [{ path: "app/controllers/posts_controller.rb", indexStatus: " ", worktreeStatus: "M" }] },
      diff: { diff: "diff --git a/app/controllers/posts_controller.rb b/app/controllers/posts_controller.rb\n+safe search", truncated: false },
      readiness: readyIntegration(),
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    fireEvent.click(await screen.findByRole("button", { name: "Finish conversation" }));

    await waitFor(() => expect(native.stop).toHaveBeenCalledWith(31));
    fireEvent.click(await screen.findByRole("button", { name: "Review changes" }));
    expect(await screen.findByText("1 changed file")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Integrate" })).toBeEnabled();
  });

  it("opens a native folder picker instead of requiring a typed path", async () => {
    native.chooseProjectFolder.mockResolvedValue(project.path);
    render(<App />);
    await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" });
    fireEvent.click(screen.getByRole("button", { name: /^choose project$/i }));
    fireEvent.click(await screen.findByRole("button", { name: "Choose project folder" }));

    await waitFor(() => expect(native.chooseProjectFolder).toHaveBeenCalledOnce());
    expect(native.inspectProject).toHaveBeenCalledWith(project.path);
    expect(await screen.findByRole("heading", { name: "What should Rubyn work on?" })).toBeInTheDocument();
  });

  it("requires and persists the versioned first-launch trust disclosure", async () => {
    native.appState.mockResolvedValue({ ...emptyState, onboardingVersion: 0, trustedProjectPaths: [] });
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Your repository stays under your control." })).toBeInTheDocument();
    expect(screen.getByText("Isolated by default")).toBeInTheDocument();
    expect(screen.getByText("You approve repository edits")).toBeInTheDocument();
    expect(screen.getByText("Models require your account")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /I understand/ }));

    await waitFor(() => expect(native.saveAppState).toHaveBeenCalledWith(expect.objectContaining({ onboardingVersion: 1 })));
    expect(await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" })).toBeInTheDocument();
  });

  it("inspects an untrusted repository without opening it when confirmation is cancelled", async () => {
    native.appState.mockResolvedValue({ ...emptyState, trustedProjectPaths: [] });
    native.chooseProjectFolder.mockResolvedValue(project.path);
    render(<App />);
    await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" });
    fireEvent.click(screen.getByRole("button", { name: /^choose project$/i }));
    fireEvent.click(await screen.findByRole("button", { name: "Choose project folder" }));

    expect(await screen.findByRole("heading", { name: "Trust ledger?" })).toBeInTheDocument();
    expect(screen.getAllByText(project.path)).toHaveLength(2);
    expect(screen.getByText("RUBYN.md detected")).toBeInTheDocument();
    expect(native.projectData).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("heading", { name: "Trust ledger?" })).not.toBeInTheDocument();
    expect(native.trustProject).not.toHaveBeenCalled();
    expect(native.projectData).not.toHaveBeenCalled();
  });

  it("persists canonical repository trust before loading project state", async () => {
    native.appState.mockResolvedValue({ ...emptyState, trustedProjectPaths: [] });
    native.chooseProjectFolder.mockResolvedValue(project.path);
    render(<App />);
    await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" });
    fireEvent.click(screen.getByRole("button", { name: /^choose project$/i }));
    fireEvent.click(await screen.findByRole("button", { name: "Choose project folder" }));
    fireEvent.click(await screen.findByRole("button", { name: "Trust and open" }));

    await waitFor(() => expect(native.trustProject).toHaveBeenCalledWith(project.path));
    expect(native.trustProject.mock.invocationCallOrder[0]).toBeLessThan(native.projectData.mock.invocationCallOrder[0]);
    expect(await screen.findByRole("heading", { name: "What should Rubyn work on?" })).toBeInTheDocument();
  });

  it("rejects a selected folder that is not a Git repository before trust", async () => {
    native.appState.mockResolvedValue({ ...emptyState, trustedProjectPaths: [] });
    native.chooseProjectFolder.mockResolvedValue("/work/not-git");
    native.inspectProject.mockResolvedValue({ ...project, path: "/work/not-git", name: "not-git", gitRoot: undefined });
    render(<App />);
    await screen.findByRole("heading", { name: "Choose a Rails or Ruby project" });
    fireEvent.click(screen.getByRole("button", { name: /^choose project$/i }));
    fireEvent.click(await screen.findByRole("button", { name: "Choose project folder" }));

    expect(await screen.findByText(/not a Git repository/)).toBeInTheDocument();
    expect(native.trustProject).not.toHaveBeenCalled();
    expect(native.projectData).not.toHaveBeenCalled();
  });

  it("attaches a selected image to a new Rubyn conversation", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    native.chooseAttachments.mockResolvedValue([{ path: "/tmp/layout.png", name: "layout.png", kind: "image" }]);
    native.launchPrompt.mockResolvedValue({ id: 31, projectPath: "/runs/31", sourceProjectPath: project.path, worktreePath: "/runs/31", mode: "prompt", running: true, outcome: "running" });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    expect(await screen.findByRole("heading", { name: "What should Rubyn work on?" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Attach images or files" }));
    expect(await screen.findByText("layout.png")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Start conversation" }));

    await waitFor(() => expect(native.launchPrompt).toHaveBeenCalledWith(
      project.path,
      "Review the attached file or image.",
      [{ path: "/tmp/layout.png", name: "layout.png", kind: "image" }],
      connectedModel,
    ));
  });

  it("renames a workflow column inline without a native prompt", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));
    fireEvent.click(await screen.findByLabelText("Backlog column actions"));
    fireEvent.click(screen.getByRole("button", { name: "Rename Backlog" }));
    const input = screen.getByRole("textbox", { name: "New name for Backlog" });
    fireEvent.change(input, { target: { value: "Inbox" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(native.updateWorkflowColumn).toHaveBeenCalledWith(1, "Inbox"));
  });

  it("closes a column menu before showing delete confirmation", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));

    const menuButton = await screen.findByLabelText("Planning column actions");
    const menu = menuButton.closest("details");
    fireEvent.click(menuButton);
    expect(menu).toHaveAttribute("open");
    fireEvent.click(screen.getByRole("button", { name: "Delete Planning" }));

    expect(menu).not.toHaveAttribute("open");
    expect(screen.getByRole("alert")).toHaveTextContent("Delete Planning?");
    expect(screen.getByRole("button", { name: "Delete column" })).toBeVisible();
  });

  it("creates an agent profile and assigns it through a column handoff", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    data.tasks = [{ ...makeTask(9, "Implement authorization"), status: "implementing", assignedAgentId: 2 }];
    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));
    expect(screen.queryByRole("textbox", { name: "Agent name" })).not.toBeInTheDocument();
    expect(screen.getByText("Rubyn instructions: Builder")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start Rubyn" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Agents" }));
    expect(screen.queryByRole("textbox", { name: "Agent name" })).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Create agent" }));
    const agentDialog = screen.getByRole("dialog", { name: "Create an agent" });

    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Agent name" }), { target: { value: "Security" } });
    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Agent role" }), { target: { value: "security review" } });
    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Mission" }), { target: { value: "Find security defects." } });
    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Starting context" }), { target: { value: "Read authorization rules first." } });
    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Working method" }), { target: { value: "Trace every tenant boundary." } });
    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Finish line" }), { target: { value: "Every finding has evidence." } });
    fireEvent.change(within(agentDialog).getByRole("textbox", { name: "Guardrails" }), { target: { value: "Do not change code; ask when intent is unclear." } });
    expect(within(agentDialog).getByText("5/5")).toBeInTheDocument();
    fireEvent.click(within(agentDialog).getByRole("button", { name: "Create agent" }));
    await waitFor(() => expect(native.createAgentProfile).toHaveBeenCalledWith(project.path, "Security", "security review", "Mission\nFind security defects.\n\nStarting context\nRead authorization rules first.\n\nWorking method\nTrace every tenant boundary.\n\nFinish line\nEvery finding has evidence.\n\nGuardrails\nDo not change code; ask when intent is unclear."));

    expect(screen.queryByText("Prepare")).not.toBeInTheDocument();
    fireEvent.change(screen.getByRole("combobox", { name: "Instructions for Review" }), { target: { value: "13" } });

    await waitFor(() => expect(native.updateWorkflowColumn).toHaveBeenCalledWith(4, undefined, undefined, 13));
    expect(native.launchPrompt).not.toHaveBeenCalled();
  });

  it("edits an existing agent and upgrades legacy instructions in the same modal", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Agents" }));
    fireEvent.click(await screen.findByRole("button", { name: "Edit Planner" }));
    const dialog = screen.getByRole("dialog", { name: "Edit Planner" });

    expect(within(dialog).getByRole("textbox", { name: "Mission" })).toHaveValue("Plan it.");
    expect(within(dialog).getByText("1/5")).toBeInTheDocument();
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Agent name" }), { target: { value: "Lead Planner" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Mission" }), { target: { value: "Turn work into a complete plan." } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Starting context" }), { target: { value: "Read the task and repository guidance." } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Working method" }), { target: { value: "Resolve uncertainty before sequencing work." } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Finish line" }), { target: { value: "The builder can execute without guessing." } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Guardrails" }), { target: { value: "Do not edit code; ask about product ambiguity." } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(native.updateAgentProfile).toHaveBeenCalledWith(1, "Lead Planner", "planning", "Mission\nTurn work into a complete plan.\n\nStarting context\nRead the task and repository guidance.\n\nWorking method\nResolve uncertainty before sequencing work.\n\nFinish line\nThe builder can execute without guessing.\n\nGuardrails\nDo not edit code; ask about product ambiguity."));
    expect(await screen.findByText("Lead Planner")).toBeInTheDocument();
    expect(native.createAgentProfile).not.toHaveBeenCalled();
  });

  it("opens the real source of a bundled skill and closes it with Escape", async () => {
    native.appState.mockResolvedValue({ ...emptyState, recentProjects: [{ path: project.path, name: project.name }] });
    native.listBundledSkills.mockResolvedValue([{ name: "Rails review", path: "rails/review.md", description: "Review Rails changes." }]);
    native.readSkill.mockResolvedValue({ path: "rails/review.md", content: "# Rails review\n\nInspect authorization." });
    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Rubyn skills" }));
    fireEvent.click(await screen.findByRole("button", { name: "Read skill" }));
    expect(await screen.findByRole("dialog", { name: "Rails review" })).toHaveTextContent("Inspect authorization");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Rails review" })).not.toBeInTheDocument();
  });

  it("fans out only ready tasks up to the global concurrency limit", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.tasks = [makeTask(1, "First ready task"), makeTask(2, "Second ready task")];
    let runId = 40;
    native.launchPrompt.mockImplementation(async (_path: string, prompt: string) => {
      runId += 1;
      data.runs.push({ ...cancelledRun(), id: runId, prompt, running: true, outcome: "running", finishedAt: undefined });
      return { id: runId, projectPath: `/runs/${runId}/workspace`, mode: "prompt", running: true, outcome: "running" };
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    fireEvent.click(await screen.findByRole("button", { name: "Plan fan-out" }));
    expect(screen.getByText("Choose parallel tasks")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByText("Choose parallel tasks")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Plan fan-out" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /First ready task/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Second ready task/ }));
    fireEvent.click(screen.getByRole("button", { name: "Launch 2" }));

    await waitFor(() => expect(native.launchPrompt).toHaveBeenCalledTimes(2));
    expect(native.launchPrompt).toHaveBeenCalledWith(project.path, "First ready task", [], connectedModel);
    expect(native.launchPrompt).toHaveBeenCalledWith(project.path, "Second ready task", [], connectedModel);
    expect(native.updateTask).toHaveBeenCalledWith(1, "implementing", 41);
    expect(native.updateTask).toHaveBeenCalledWith(2, "implementing", 42);
  });

  it("uses the configured parallel limit for launch and fan-out capacity", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      preferences: { ...emptyState.preferences, parallelLimit: 1 },
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.tasks = [makeTask(1, "Ready task")];
    data.runs = [{ ...cancelledRun(), running: true, outcome: "running", finishedAt: undefined }];

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    expect(await screen.findByText("1/1 live")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Plan fan-out" })).toBeDisabled();
  });

  it("configures workflow columns and assigns Rubyn conversations to tasks and todos", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.tasks = [makeTask(1, "Implement export")];
    data.todos = [makeTodo(2, "Verify export")];
    data.runs = [cancelledRun()];

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));

    fireEvent.change(screen.getByRole("textbox", { name: "New workflow column" }), { target: { value: "Deploying" } });
    fireEvent.click(screen.getByRole("button", { name: "Add column" }));
    await waitFor(() => expect(native.createWorkflowColumn).toHaveBeenCalledWith(project.path, "Deploying"));
    expect((await screen.findAllByText("Deploying")).length).toBeGreaterThan(0);

    fireEvent.change(screen.getByRole("combobox", { name: "Background run" }), { target: { value: "31" } });
    await waitFor(() => expect(native.updateTask).toHaveBeenCalledWith(1, undefined, 31));
    fireEvent.change(screen.getByRole("combobox", { name: "Agent for Verify export" }), { target: { value: "31" } });
    await waitFor(() => expect(native.updateTodo).toHaveBeenCalledWith(2, undefined, 31));
  });

  it("keeps a per-thread draft across navigation and queues it into a busy actor", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [{ ...cancelledRun(), running: true, outcome: "running", finishedAt: undefined }];
    native.sendRunMessage.mockResolvedValue({});

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    const composer = await screen.findByRole("textbox", { name: "Message Rubyn" });
    fireEvent.change(composer, { target: { value: "Now add the request spec" } });
    fireEvent.click(screen.getByRole("button", { name: "Tasks & todos" }));
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    expect(await screen.findByRole("textbox", { name: "Message Rubyn" })).toHaveValue("Now add the request spec");
    fireEvent.click(screen.getByRole("button", { name: "Queue" }));

    await waitFor(() => expect(native.sendRunMessage).toHaveBeenCalledWith(31, "Now add the request spec", [], connectedModel));
  });

  it("continues a completed turn in the same conversation", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [cancelledRun()];
    native.sendRunMessage.mockResolvedValue({ id: 31, running: true, outcome: "running" });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    const composer = await screen.findByRole("textbox", { name: "Message Rubyn" });
    expect(composer).toHaveAttribute("placeholder", "Continue this conversation…");
    expect(screen.queryByText(/conversation is closed/i)).not.toBeInTheDocument();
    fireEvent.change(composer, { target: { value: "Now add the request spec" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    await waitFor(() => expect(native.sendRunMessage).toHaveBeenCalledWith(31, "Now add the request spec", [], connectedModel));
  });

  it("shows a persisted edit proposal and requires an explicit decision", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [{ ...cancelledRun(), running: true, outcome: "running", finishedAt: undefined }];
    data.approvals = [{ id: 9, runId: 31, editId: "edit-9", path: "app/models/user.rb", content: "class User\nend\n", editType: "modify", approvalKind: "fileChange", status: "pending", requestedAt: 1 }];
    native.resolveEditApproval.mockImplementation(async (_runId: number, editId: string, accepted: boolean) => {
      data.approvals[0] = { ...data.approvals[0], status: accepted ? "approved" : "denied", decidedAt: 2 };
      return data.approvals[0];
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    const approvals = await screen.findByLabelText("Pending approvals");
    expect(approvals).toHaveTextContent("app/models/user.rb");
    expect(approvals).toHaveTextContent("class User end");
    fireEvent.click(screen.getByRole("button", { name: "Approve edit" }));

    await waitFor(() => expect(native.resolveEditApproval).toHaveBeenCalledWith(31, "edit-9", true));
    await waitFor(() => expect(screen.queryByLabelText("Pending approvals")).not.toBeInTheDocument());
  });

  it("shows exact command context and grants only the displayed request", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [{ ...cancelledRun(), running: true, outcome: "running", finishedAt: undefined }];
    data.approvals = [{ id: 10, runId: 31, editId: "command-10", path: "/tmp/example-app", content: "bundle exec rails test\n\nReason: Run the test suite", editType: "command", approvalKind: "commandExecution", status: "pending", requestedAt: 1 }];
    native.resolveEditApproval.mockImplementation(async (_runId: number, editId: string, accepted: boolean) => {
      data.approvals[0] = { ...data.approvals[0], status: accepted ? "approved" : "denied", decidedAt: 2 };
      return data.approvals[0];
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    const approvals = await screen.findByLabelText("Pending approvals");
    expect(approvals).toHaveTextContent("Working directory: /tmp/example-app");
    expect(approvals).toHaveTextContent("bundle exec rails test Reason: Run the test suite");
    expect(approvals).toHaveTextContent("Only this displayed command will be authorized.");
    fireEvent.click(screen.getByRole("button", { name: "Run command" }));

    await waitFor(() => expect(native.resolveEditApproval).toHaveBeenCalledWith(31, "command-10", true));
  });

  it("shows provider token usage and cached-input efficiency for the selected conversation", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [cancelledRun()];
    native.pollRunEvents.mockResolvedValue({
      run: data.runs[0],
      events: [{
        id: 11,
        runId: 31,
        protocolSequence: 11,
        kind: "token/usage",
        payload: { inputTokens: 40_000, cachedInputTokens: 30_000, outputTokens: 1_200, reasoningOutputTokens: 200, totalTokens: 41_200, source: "provider" },
        raw: "",
        createdAt: 11,
      }],
      nextEventId: 11,
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    expect(await screen.findByText(/Provider usage · 41.2K total · 40K input · 1.2K output · 200 reasoning/)).toBeInTheDocument();
    expect(screen.getByText(/Rubyn efficiency · 30K cached input tokens reused \(75%\)/)).toBeInTheDocument();
  });

  it("keeps a followed conversation scrolled to its newest message", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [cancelledRun()];

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    const viewport = await screen.findByLabelText("Conversation messages");
    Object.defineProperty(viewport, "scrollHeight", { configurable: true, value: 600 });
    Object.defineProperty(viewport, "clientHeight", { configurable: true, value: 200 });
    viewport.scrollTop = 400;
    fireEvent.scroll(viewport);

    act(() => useHarnessStore.getState().appendRunEvents(31, [{
      id: 12,
      runId: 31,
      protocolSequence: 12,
      kind: "stream/text",
      payload: { text: "Newest reply", final: true },
      raw: "",
      createdAt: 12,
    }], 12));

    expect(await screen.findByText("Newest reply")).toBeInTheDocument();
    expect(viewport.scrollTop).toBe(600);
  });

  it("shows live reasoning, tool activity, progress, and streaming response text", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [{ ...cancelledRun(), running: true, outcome: "running", finishedAt: undefined }];

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));
    act(() => useHarnessStore.getState().appendRunEvents(31, [
      { id: 20, runId: 31, protocolSequence: 20, kind: "reasoning/delta", payload: { itemId: "reason-1", text: "Inspecting the repository structure." }, raw: "", createdAt: 20 },
      { id: 21, runId: 31, protocolSequence: 21, kind: "tool/use", payload: { requestId: "tool-1", tool: "shell", args: { command: "bundle exec rspec" } }, raw: "", createdAt: 21 },
      { id: 22, runId: 31, protocolSequence: 22, kind: "tool/progress", payload: { requestId: "tool-1", text: "3 examples, 0 failures" }, raw: "", createdAt: 22 },
      { id: 23, runId: 31, protocolSequence: 23, kind: "tool/result", payload: { requestId: "tool-1", tool: "shell", success: true, summary: "Command exited with code 0" }, raw: "", createdAt: 23 },
      { id: 24, runId: 31, protocolSequence: 24, kind: "stream/text", payload: { text: "The tests are green", final: false }, raw: "", createdAt: 24 },
    ], 24));

    expect(await screen.findByText("Inspecting the repository structure.")).toBeInTheDocument();
    expect(screen.getByText("Run command")).toBeInTheDocument();
    expect(screen.getByText("Command exited with code 0")).toBeInTheDocument();
    expect(screen.getByText("3 examples, 0 failures")).toBeInTheDocument();
    expect(screen.getByText("The tests are green")).toBeInTheDocument();
    expect(screen.getAllByText("Live").length).toBeGreaterThan(0);
    const liveReply = screen.getByText("The tests are green").closest(".chat-bubble");
    const toolCard = screen.getByText("Run command").closest(".tool-activity");
    expect(toolCard?.compareDocumentPosition(liveReply!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);

    act(() => useHarnessStore.getState().appendRunEvents(31, [
      { id: 25, runId: 31, protocolSequence: 25, kind: "stream/text", payload: { text: "All tests pass.", final: true }, raw: "", createdAt: 25 },
    ], 25));
    expect(await screen.findByText("All tests pass.")).toBeInTheDocument();
    expect(screen.queryByText("The tests are green")).not.toBeInTheDocument();
    expect(screen.getByText("All tests pass.").closest(".chat-bubble")).toBe(liveReply);
  });

  it("shows a provider failure instead of leaving the conversation looking busy", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    data.runs = [{ ...cancelledRun(), running: true, outcome: "failed", finishedAt: undefined }];
    native.pollRunEvents.mockResolvedValue({
      run: data.runs[0],
      events: [{
        id: 9,
        runId: 31,
        protocolSequence: 9,
        kind: "agent/status",
        payload: { status: "error", error: "No OpenAI API key configured" },
        raw: "",
        createdAt: 9,
      }],
      nextEventId: 9,
    });

    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Talk to Rubyn" }));

    expect(await screen.findByText("Provider error: No OpenAI API key configured")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Stop turn" })).not.toBeInTheDocument();
  });

  it("creates a real project-local skill through the native bridge", async () => {
    native.appState.mockResolvedValue({
      ...emptyState,
      recentProjects: [{ path: project.path, name: project.name }],
    });
    render(<App />);
    await screen.findByText("ledger is the source of truth.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Rubyn skills" }));
    fireEvent.click(await screen.findByRole("button", { name: "Create project skill" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Tenant safety" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Guidance" }), { target: { value: "Check every query for tenant scope." } });
    fireEvent.click(screen.getByRole("button", { name: "Install project skill" }));

    expect(await screen.findByRole("heading", { name: "Tenant safety" })).toBeInTheDocument();
    expect(native.createProjectSkill).toHaveBeenCalledWith(
      project.path,
      "Tenant safety",
      "Check every query for tenant scope.",
    );
    expect(screen.getByText(/Commit it so detached Rubyn worktrees load it/i)).toBeInTheDocument();
  });
});
