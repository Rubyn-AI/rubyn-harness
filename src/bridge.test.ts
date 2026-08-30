// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const openDialog = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialog }));

import { harnessBridge, isDesktop } from "./bridge";

describe("desktop bridge contract", () => {
  beforeEach(() => {
    invoke.mockReset();
    openDialog.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("opens the native project directory picker", async () => {
    openDialog.mockResolvedValue("/repo");
    await expect(harnessBridge.chooseProjectFolder()).resolves.toBe("/repo");
    expect(openDialog).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: "Choose a Ruby or Rails project",
    });
  });

  it("normalizes native image and text attachment selections", async () => {
    openDialog.mockResolvedValue(["/tmp/screen.PNG", "/tmp/model.rb"]);
    await expect(harnessBridge.chooseAttachments()).resolves.toEqual([
      { path: "/tmp/screen.PNG", name: "screen.PNG", kind: "image" },
      { path: "/tmp/model.rb", name: "model.rb", kind: "text" },
    ]);
    expect(openDialog).toHaveBeenCalledWith(expect.objectContaining({ multiple: true, directory: false }));
  });

  it("rejects desktop-only operations in browser mode before invoking Tauri", async () => {
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");

    expect(isDesktop()).toBe(false);
    await expect(harnessBridge.inspectProject("/tmp/app")).rejects.toThrow(
      "Rubyn Harness requires the desktop runtime.",
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it("maps provider discovery, encrypted setup, revocation, and Codex login", async () => {
    invoke.mockResolvedValue({ models: [], activeProvider: "anthropic", activeModel: "claude-sonnet-5", modelMode: "auto" });

    await harnessBridge.listModels();
    await harnessBridge.upsertProvider({
      name: "minimax",
      baseUrl: "https://api.minimax.io/v1",
      apiFormat: "openai",
      envKey: "MINIMAX_API_KEY",
      apiKey: "secret",
      models: ["MiniMax-M2.5"],
    });
    await harnessBridge.revokeProvider("minimax");
    await harnessBridge.startCodexLogin();

    expect(invoke.mock.calls).toEqual([
      ["list_models", undefined],
      ["upsert_provider", { request: { name: "minimax", baseUrl: "https://api.minimax.io/v1", apiFormat: "openai", envKey: "MINIMAX_API_KEY", apiKey: "secret", models: ["MiniMax-M2.5"] } }],
      ["revoke_provider", { provider: "minimax" }],
      ["start_codex_login", undefined],
    ]);
  });

  it("creates diagnostics through the native allowlisted report command", async () => {
    invoke.mockResolvedValue({ path: "/app-data/diagnostics/report.json", createdAt: 42 });
    await expect(harnessBridge.createSanitizedDiagnostics()).resolves.toEqual({ path: "/app-data/diagnostics/report.json", createdAt: 42 });
    expect(invoke).toHaveBeenCalledWith("create_sanitized_diagnostics", undefined);
  });

  it("removes local data through one native fail-closed command", async () => {
    invoke.mockResolvedValue({ appState: { onboardingVersion: 0 }, cleanupPending: false, retainedPaths: [] });
    await harnessBridge.clearLocalData();
    expect(invoke).toHaveBeenCalledWith("clear_local_data", undefined);
  });

  it("keeps the selected model when continuing a completed conversation", async () => {
    invoke.mockResolvedValue({ id: 17, running: true, outcome: "running" });

    await harnessBridge.sendRunMessage(17, "Continue the work", [], {
      provider: "codex",
      model: "gpt-5.6-terra",
      tier: "top",
    });

    expect(invoke).toHaveBeenCalledWith("send_run_message", {
      request: {
        runId: 17,
        message: "Continue the work",
        provider: "codex",
        model: "gpt-5.6-terra",
      },
    });
  });

  it("maps the Rubyn Chisel toggle to native configuration commands", async () => {
    invoke.mockResolvedValueOnce("off").mockResolvedValueOnce("full");

    await expect(harnessBridge.getChiselMode()).resolves.toBe("off");
    await expect(harnessBridge.setChiselEnabled(true)).resolves.toBe("full");

    expect(invoke.mock.calls).toEqual([
      ["get_chisel_mode", undefined],
      ["set_chisel_enabled", { enabled: true }],
    ]);
  });

  it("maps durable conversation management", async () => {
    invoke.mockResolvedValue({ id: 17, title: "Billing export", pinned: true });

    await harnessBridge.updateConversation({ id: 17, title: "Billing export", pinned: true });

    expect(invoke).toHaveBeenCalledWith("update_conversation", {
      request: { id: 17, title: "Billing export", pinned: true },
    });
  });

  it("maps an explicit file-edit decision", async () => {
    invoke.mockResolvedValue({ id: 4, status: "approved" });

    await harnessBridge.resolveEditApproval(17, "edit-4", true);

    expect(invoke).toHaveBeenCalledWith("resolve_edit_approval", {
      request: { runId: 17, editId: "edit-4", accepted: true },
    });
  });

  it("maps agent profiles and column handoffs", async () => {
    invoke.mockResolvedValue({});

    await harnessBridge.createAgentProfile("/repo", "Security", "review", "Check tenant boundaries.");
    await harnessBridge.updateAgentProfile(9, "Security reviewer", "security", "Inspect authorization.");
    await harnessBridge.updateWorkflowColumn(4, undefined, undefined, 9);
    await harnessBridge.deleteAgentProfile(9);

    expect(invoke.mock.calls).toEqual([
      ["create_agent_profile", { request: { projectPath: "/repo", name: "Security", role: "review", instructions: "Check tenant boundaries." } }],
      ["update_agent_profile", { request: { id: 9, name: "Security reviewer", role: "security", instructions: "Inspect authorization." } }],
      ["update_workflow_column", { request: { id: 4, agentId: 9 } }],
      ["delete_agent_profile", { id: 9 }],
    ]);
  });

  it("maps the project, run, status, cancellation, output, and diff flow to typed commands", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "inspect_project") {
        return Promise.resolve({ path: "/repo", name: "repo", isRuby: true, isRails: true, hasRubynInstructions: false });
      }
      if (command === "launch_engine") {
        return Promise.resolve({ id: 17, projectPath: "/runs/17/workspace", mode: "prompt", running: true, outcome: "running" });
      }
      if (command === "get_project_data") return Promise.resolve({ project: { id: 3 }, tasks: [], todos: [], runs: [] });
      if (command === "create_project_todo") return Promise.resolve({ id: 7, status: "queued" });
      if (command === "update_project_todo") return Promise.resolve({ id: 7, status: "doing" });
      if (command === "create_project_task") return Promise.resolve({ id: 8, status: "queued" });
      if (command === "update_project_task") return Promise.resolve({ id: 8, status: "doing" });
      if (command === "list_runs") return Promise.resolve([]);
      if (command === "poll_run_events") return Promise.resolve({ run: { id: 17 }, events: [], nextEventId: 0 });
      if (command === "inspect_run_worktree") {
        return Promise.resolve({ run: { id: 17 }, status: { files: [] }, diff: { diff: "+change", truncated: false } });
      }
      return Promise.resolve(undefined);
    });

    const appState = {
      preferences: { defaultModel: "openai/gpt-5.4", parallelLimit: 3, autoCompaction: true, yoloEnabled: false },
      recentProjects: [{ path: "/repo", name: "repo" }],
    };
    await harnessBridge.engineHealth();
    await harnessBridge.appState();
    await harnessBridge.saveAppState(appState);
    await harnessBridge.inspectProject("/repo");
    await harnessBridge.trustProject("/repo");
    await harnessBridge.projectData("/repo");
    await harnessBridge.createTodo("/repo", "Confirm tenant boundary");
    await harnessBridge.updateTodo(7, "doing");
    await harnessBridge.createTask("/repo", "Add request specs", "Cover authorization", "The suite prevents regressions", [5, 6]);
    await harnessBridge.updateTask(8, "doing");
    await harnessBridge.gitStatus("/repo");
    await harnessBridge.launchPrompt("/repo", "Add request specs");
    await harnessBridge.sendRunMessage(17, "Now add the system spec");
    await harnessBridge.listRuns("/repo");
    await harnessBridge.pollRunEvents(17, 4);
    await harnessBridge.stop(17);
    await harnessBridge.inspectRunWorktree(17);
    await harnessBridge.integrateRun(17);
    await harnessBridge.discardRun(18);
    await harnessBridge.retryRunCleanup(19);
    await harnessBridge.listBundledSkills();
    await harnessBridge.listProjectSkills("/repo");
    await harnessBridge.createProjectSkill("/repo", "Tenant safety", "Scope every query.");
    await harnessBridge.readSkill("rails/security.md", "/repo");

    expect(invoke.mock.calls).toEqual([
      ["engine_health", undefined],
      ["get_app_state", undefined],
      ["save_app_state", { state: appState }],
      ["inspect_project", { projectPath: "/repo" }],
      ["trust_project", { projectPath: "/repo" }],
      ["get_project_data", { projectPath: "/repo" }],
      ["create_project_todo", { request: { projectPath: "/repo", title: "Confirm tenant boundary", owner: "You", status: "queued" } }],
      ["update_project_todo", { request: { id: 7, status: "doing" } }],
      ["create_project_task", { request: { projectPath: "/repo", title: "Add request specs", detail: "Cover authorization", outcome: "The suite prevents regressions", status: "queued", dependsOn: [5, 6] } }],
      ["update_project_task", { request: { id: 8, status: "doing" } }],
      ["get_git_status", { projectPath: "/repo" }],
      ["launch_engine", { request: { projectPath: "/repo", mode: { prompt: { prompt: "Add request specs" } }, yolo: false } }],
      ["send_run_message", { request: { runId: 17, message: "Now add the system spec" } }],
      ["list_runs", { projectPath: "/repo" }],
      ["poll_run_events", { runId: 17, afterEventId: 4 }],
      ["stop_engine", { sessionId: 17 }],
      ["inspect_run_worktree", { runId: 17 }],
      ["integrate_run", { runId: 17 }],
      ["discard_run", { runId: 18 }],
      ["retry_run_cleanup", { runId: 19 }],
      ["list_bundled_skills", undefined],
      ["list_project_skills", { projectPath: "/repo" }],
      ["create_project_skill", { request: { projectPath: "/repo", name: "Tenant safety", content: "Scope every query." } }],
      ["read_skill", { path: "rails/security.md", projectPath: "/repo" }],
    ]);
  });

  it("maps the complete Wayfinder and Grill approval contract", async () => {
    invoke.mockResolvedValue({});
    const ticket = {
      mapId: 4,
      title: "Choose tenancy boundary",
      question: "Where is isolation enforced?",
      information: "The app serves multiple firms.",
      outcome: "One boundary is approved.",
      ticketType: "grill" as const,
      dependsOn: [2],
      modelRole: "Sol",
      effort: "high",
      budgetCents: 125,
    };
    await harnessBridge.listWayfinderMaps("/repo");
    await harnessBridge.createWayfinderMap("/repo", "Map tenant isolation", "planning");
    await harnessBridge.getWayfinderMap(4);
    await harnessBridge.updateWayfinderMap(4, { destination: "Isolation is proved" });
    await harnessBridge.createWayfinderTicket(ticket);
    await harnessBridge.updateWayfinderTicket({ id: 8, title: "Choose tenant boundary" });
    await harnessBridge.submitWayfinderAnswers(8, [{ questionId: 11, answers: ["policy"], customAnswer: "" }]);
    await harnessBridge.activateWayfinderMap(4);
    await harnessBridge.resolveWayfinderTicket(8, "Use policy objects", [], [9]);
    await harnessBridge.completeWayfinderUserAction(10, "Security approved the boundary");
    await harnessBridge.linkWayfinderRun(8, 17);
    await harnessBridge.retireWayfinderTicket(9);
    await harnessBridge.archiveWayfinderMap(4);
    await harnessBridge.listWayfinderBlockers("/repo");
    await harnessBridge.answerEngineQuestion(17, "rpc-3", { questions: [{ id: "q1", selected: ["a"] }] });

    expect(invoke.mock.calls).toEqual([
      ["list_wayfinder_maps", { projectPath: "/repo" }],
      ["create_wayfinder_map", { request: { projectPath: "/repo", idea: "Map tenant isolation", codeTaskStatus: "planning" } }],
      ["get_wayfinder_map", { mapId: 4 }],
      ["update_wayfinder_map", { mapId: 4, destination: "Isolation is proved" }],
      ["create_wayfinder_ticket", { request: ticket }],
      ["update_wayfinder_ticket", { request: { id: 8, title: "Choose tenant boundary" } }],
      ["submit_wayfinder_answers", { ticketId: 8, answers: [{ questionId: 11, answers: ["policy"], customAnswer: "" }] }],
      ["activate_wayfinder_map", { mapId: 4 }],
      ["resolve_wayfinder_ticket", { request: { ticketId: 8, resolution: "Use policy objects", addTickets: [], retireTicketIds: [9] } }],
      ["complete_wayfinder_user_action", { ticketId: 10, resultNote: "Security approved the boundary" }],
      ["link_wayfinder_run", { ticketId: 8, runId: 17 }],
      ["retire_wayfinder_ticket", { ticketId: 9 }],
      ["archive_wayfinder_map", { mapId: 4 }],
      ["list_wayfinder_blockers", { projectPath: "/repo" }],
      ["answer_engine_question", { runId: 17, requestId: "rpc-3", answer: { questions: [{ id: "q1", selected: ["a"] }] } }],
    ]);
  });
});
