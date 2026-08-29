import { FormEvent, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  Activity,
  ArrowRight,
  ArrowUpRight,
  Check,
  ChevronRight,
  CircleStop,
  Command,
  FileCode2,
  FolderKanban,
  GitBranch,
  Layers3,
  KeyRound,
  MessageCircle,
  Menu,
  MoreHorizontal,
  Map as MapIcon,
  PanelLeftClose,
  Paperclip,
  Pin,
  Archive,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Sparkles,
  TerminalSquare,
  Trash2,
  UsersRound,
  Play,
  X,
} from "lucide-react";
import {
  harnessBridge,
  isDesktop,
  type LocalAppState,
  type RunRecord,
  type RunEventRecord,
  type RunWorktreeInspection,
  type TaskRecord,
  type TaskStatus,
  type TodoRecord,
  type WorkflowStatus,
  type AttachmentSelection,
  type AgentProfile,
  type EditApprovalRecord,
} from "./bridge";
import { type View, useHarnessStore } from "./store";
import { Wayfinder } from "./Wayfinder";

const primaryNavigation: { id: View; label: string; icon: typeof Activity }[] = [
  { id: "agents", label: "Talk to Rubyn", icon: MessageCircle },
  { id: "wayfinder", label: "Wayfinder", icon: MapIcon },
  { id: "workboard", label: "Tasks & todos", icon: Layers3 },
  { id: "team", label: "Agents", icon: UsersRound },
  { id: "review", label: "Review", icon: ShieldCheck },
];

const utilityNavigation: { id: View; label: string; icon: typeof Activity }[] = [
  { id: "control", label: "Control room", icon: Activity },
  { id: "skills", label: "Rubyn skills", icon: Sparkles },
  { id: "accounts", label: "Models & accounts", icon: KeyRound },
  { id: "projects", label: "Projects", icon: FolderKanban },
];
const navigation = [...primaryNavigation, ...utilityNavigation];

const labels: Record<View, string> = {
  control: "Control room",
  workboard: "Tasks & todos",
  agents: "Talk to Rubyn",
  team: "Agents",
  wayfinder: "Wayfinder",
  skills: "Bundled skills",
  review: "Worktree review",
  accounts: "Models & accounts",
  projects: "Projects",
};

const todoStatuses: { value: WorkflowStatus; label: string }[] = [
  { value: "queued", label: "Queued" },
  { value: "doing", label: "In progress" },
  { value: "review", label: "Review" },
  { value: "done", label: "Done" },
];

function Kicker({ children }: { children: string }) {
  return <span className="kicker">{children}</span>;
}

function EmptyState({ title, children, action }: { title: string; children: string; action?: React.ReactNode }) {
  return (
    <div className="empty-state-card">
      <span className="empty-gem" />
      <h2>{title}</h2>
      <p>{children}</p>
      {action}
    </div>
  );
}

function ProjectRequired() {
  const setView = useHarnessStore((state) => state.setView);
  return (
    <EmptyState
      title="Choose a Rails or Ruby project"
      action={<button className="button primary" onClick={() => setView("projects")}>Choose project <ArrowRight size={15} /></button>}
    >
      Rubyn Harness only works against a project you explicitly inspect. No sample repository or fixture state is loaded.
    </EmptyState>
  );
}

function statusOf(run: RunRecord) {
  if (run.lifecycle.startsWith("integrated")) return "integrated";
  if (run.lifecycle.startsWith("discard")) return "discarded";
  if (run.outcome === "cancelled") return "cancelled";
  if (run.outcome === "failed") return "failed";
  if (run.running && run.outcome === "waiting") return "waiting";
  if (run.running) return "running";
  return "review";
}

function runLabel(run: RunRecord) {
  const status = statusOf(run);
  if (run.lifecycle.endsWith("cleanup_pending")) return `${status} · cleanup pending`;
  return status === "review" ? "ready for review" : status;
}

function conversationTitle(run: RunRecord, maximum = 48) {
  return shortPrompt(run.title?.trim() || run.prompt, maximum);
}

function currentModelPreference(value: string | undefined) {
  const normalized = value?.includes("::") ? value : value?.replace("/", "::");
  const migrations: Record<string, string> = {
    "openai::gpt-5.4": "openai::gpt-5.6-sol",
    "openai::gpt-5.4-mini": "openai::gpt-5.6-terra",
    "openai::gpt-5.4-nano": "openai::gpt-5.6-luna",
    "minimax::MiniMax-M2.5": "minimax::MiniMax-M3",
    "minimax::MiniMax-M2.5-highspeed": "minimax::MiniMax-M2.7-highspeed",
  };
  return normalized ? migrations[normalized] || normalized : "";
}

function hasTerminalLifecycle(lifecycle: string) {
  return lifecycle.startsWith("integrated") || lifecycle.startsWith("discard");
}

function shortPrompt(prompt: string, length = 74) {
  return prompt.length > length ? `${prompt.slice(0, length - 1)}…` : prompt;
}

function taskPrompt(task: TaskRecord) {
  const sections = [task.title];
  if (task.detail) sections.push(`Information\n${task.detail}`);
  if (task.outcome) sections.push(`Expected outcome\n${task.outcome}`);
  return sections.join("\n\n");
}

function agentTaskPrompt(task: TaskRecord, agent?: AgentProfile) {
  const guidance = agent ? [`Rubyn instruction profile: ${agent.name}`, `Role: ${agent.role}`, agent.instructions && `Standing instructions: ${agent.instructions}`] : [];
  return [...guidance, taskPrompt(task)].filter(Boolean).join("\n\n");
}

function useProjectRefresh() {
  const project = useHarnessStore((state) => state.project);
  const setProjectData = useHarnessStore((state) => state.setProjectData);
  const setGlobalRuns = useHarnessStore((state) => state.setGlobalRuns);
  const setNotice = useHarnessStore((state) => state.setNotice);
  const setWayfinderMaps = useHarnessStore((state) => state.setWayfinderMaps);
  const setWayfinderBlockers = useHarnessStore((state) => state.setWayfinderBlockers);
  return useCallback(async () => {
    if (!project) return;
    try {
      const [data, runs] = await Promise.all([
        harnessBridge.projectData(project.path),
        harnessBridge.listRuns(),
      ]);
      setProjectData(data);
      setGlobalRuns(runs);
      try {
        const [maps, blockers] = await Promise.all([harnessBridge.listWayfinderMaps(project.path), harnessBridge.listWayfinderBlockers(project.path)]);
        setWayfinderMaps(maps); setWayfinderBlockers(blockers);
      } catch { /* Core task/run refresh must survive an older native host during migration. */ }
    } catch (error) {
      setNotice(String(error));
    }
  }, [project, setGlobalRuns, setNotice, setProjectData, setWayfinderBlockers, setWayfinderMaps]);
}

function Metric({ label, value, detail }: { label: string; value: number | string; detail: string }) {
  return (
    <article className="metric">
      <Kicker>{label}</Kicker>
      <strong>{value}</strong>
      <span>{detail}</span>
    </article>
  );
}

function ControlRoom() {
  const { project, projectData, globalRuns, setView, openConversation } = useHarnessStore();
  if (!project || !projectData) return <ProjectRequired />;
  const backgroundRuns = projectData.runs.filter((run) => run.background);
  const active = backgroundRuns.filter((run) => run.running);
  const activeEverywhere = globalRuns.filter((run) => run.background && run.running);
  const reviewable = projectData.runs.filter((run) => statusOf(run) === "review");
  const openTasks = projectData.tasks.filter((task) => task.status !== "done");
  const openTodos = projectData.todos.filter((todo) => todo.status !== "done");
  return (
    <>
      <section className="control-hero">
        <div className="hero-copy">
          <Kicker>{project.isRails ? "Rails project" : project.isRuby ? "Ruby project" : "Git project"}</Kicker>
          <h1>Keep the work <em>weaving.</em></h1>
          <p>{project.name} is the source of truth. Background runs execute Board Tasks in detached worktrees, while conversations remain durable workspaces.</p>
          <div className="hero-actions">
            <button className="button primary" onClick={() => setView("agents")}><Plus size={16} />Launch work</button>
            <button className="button quiet" onClick={() => setView("workboard")}><Layers3 size={16} />Shape the queue</button>
          </div>
        </div>
        <div className={`weave ${active.length ? "active" : "idle"}`} aria-label={`${active.length} active Rubyn runs`}>
          <div className="weave-label"><span className={active.length ? "live-dot" : "quiet-dot"} /> {active.length ? `${active.length} run${active.length === 1 ? "" : "s"} weaving` : "Loom is quiet"}</div>
          <svg viewBox="0 0 460 218" aria-hidden="true">
            <path className="weave-line ruby" d="M0 36 C75 36 78 144 151 144 S231 68 302 68 S373 174 460 174" />
            <path className="weave-line emerald" d="M0 108 C73 108 80 30 151 30 S228 190 302 190 S379 86 460 86" />
            <path className="weave-line amber" d="M0 179 C71 179 83 92 151 92 S229 126 302 126 S387 37 460 37" />
            <circle cx="151" cy="144" r="6" className="node ruby" />
            <circle cx="302" cy="68" r="6" className="node emerald" />
            <circle cx="151" cy="30" r="6" className="node blue" />
            <circle cx="302" cy="126" r="6" className="node amber" />
          </svg>
          <div className="weave-footer"><span>{backgroundRuns.length} recorded {backgroundRuns.length === 1 ? "run" : "runs"}</span><span>{reviewable.length} worktrees awaiting review</span></div>
        </div>
      </section>
      <section className="metrics-grid" aria-label="Project state">
        <Metric label="Background runs" value={activeEverywhere.length} detail={`${active.length} active in this project`} />
        <Metric label="Open tasks" value={openTasks.length} detail={`${projectData.tasks.length} persisted`} />
        <Metric label="Open todos" value={openTodos.length} detail={`${projectData.todos.length} persisted`} />
        <Metric label="Review queue" value={reviewable.length} detail="Retained worktrees" />
      </section>
      <section className="split-grid">
        <article className="panel agent-panel">
          <div className="panel-heading"><div><Kicker>Rubyn runtime</Kicker><h2>Recent runs</h2></div><button className="text-button" onClick={() => setView("agents")}>All runs <ChevronRight size={15} /></button></div>
          {backgroundRuns.length ? (
            <div className="agent-list">
              {backgroundRuns.slice(0, 4).map((run) => (
                <button className="run-row" key={run.id} onClick={() => openConversation(run.id)}>
                  <span className={`pulse ${statusOf(run)}`} />
                  <span><strong>{conversationTitle(run)}</strong><small>Background run {run.id} · {runLabel(run)}</small></span>
                  <ChevronRight size={15} />
                </button>
              ))}
            </div>
          ) : <p className="panel-empty">No background runs yet. Launch a ready Board Task.</p>}
        </article>
        <article className="panel signal-panel">
          <div className="panel-heading"><div><Kicker>Shared momentum</Kicker><h2>Next moves</h2></div><span className="signal-count">{openTodos.length}</span></div>
          {openTodos.slice(0, 4).map((todo) => <ReadOnlyTodo key={todo.id} todo={todo} />)}
          {!openTodos.length && <p className="panel-empty">The shared todo queue is clear.</p>}
          <button className="open-board" onClick={() => setView("workboard")}>Open board <ArrowUpRight size={15} /></button>
        </article>
      </section>
    </>
  );
}

function ReadOnlyTodo({ todo }: { todo: TodoRecord }) {
  return <div className="todo-line static"><span className={`todo-mark ${todo.status}`} /><span><strong>{todo.title}</strong><small>{todo.owner} · {todo.status}</small></span></div>;
}

const agentInstructionParts = [
  { key: "mission", title: "Mission", question: "What is this agent's one job?", placeholder: "Plan changes so the builder can work without guessing." },
  { key: "context", title: "Starting context", question: "What should Rubyn inspect or know first?", placeholder: "Read the ticket, project conventions, related code, and dependencies." },
  { key: "method", title: "Working method", question: "How should Rubyn approach the work?", placeholder: "Break uncertainty into decisions, cite relevant files, and keep the plan testable." },
  { key: "finishLine", title: "Finish line", question: "What proves the work is excellent?", placeholder: "A sequenced plan names files, risks, tests, and acceptance criteria." },
  { key: "guardrails", title: "Guardrails", question: "When should Rubyn stop or ask you?", placeholder: "Do not edit code. Ask when product intent or a destructive choice is unclear." },
] as const;

type AgentInstructionKey = typeof agentInstructionParts[number]["key"];
type AgentInstructionDraft = Record<AgentInstructionKey, string>;

const emptyAgentInstructions: AgentInstructionDraft = { mission: "", context: "", method: "", finishLine: "", guardrails: "" };

function compileAgentInstructions(draft: AgentInstructionDraft) {
  return agentInstructionParts.map((part) => `${part.title}\n${draft[part.key].trim()}`).join("\n\n");
}

function parseAgentInstructions(instructions: string): AgentInstructionDraft {
  const draft = { ...emptyAgentInstructions };
  const hasStructuredInstructions = agentInstructionParts.every((part) => instructions.includes(`${part.title}\n`));
  if (!hasStructuredInstructions) return { ...draft, mission: instructions.trim() };
  agentInstructionParts.forEach((part, index) => {
    const start = instructions.indexOf(`${part.title}\n`) + part.title.length + 1;
    const next = agentInstructionParts[index + 1];
    const end = next ? instructions.indexOf(`\n\n${next.title}\n`, start) : instructions.length;
    draft[part.key] = instructions.slice(start, end < 0 ? instructions.length : end).trim();
  });
  return draft;
}

function AgentTeam() {
  const { project, projectData, setNotice } = useHarnessStore();
  const refresh = useProjectRefresh();
  const [agentEditorOpen, setAgentEditorOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<AgentProfile>();
  const [agentName, setAgentName] = useState("");
  const [agentRole, setAgentRole] = useState("implementation");
  const [agentInstructions, setAgentInstructions] = useState<AgentInstructionDraft>(emptyAgentInstructions);
  const completedInstructionParts = agentInstructionParts.filter((part) => agentInstructions[part.key].trim()).length;
  const agentReady = Boolean(agentName.trim() && agentRole.trim() && completedInstructionParts === agentInstructionParts.length);
  useEffect(() => {
    if (!agentEditorOpen) return;
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") setAgentEditorOpen(false); };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [agentEditorOpen]);
  if (!project || !projectData) return <ProjectRequired />;

  const openCreateAgent = () => {
    setEditingAgent(undefined);
    setAgentName("");
    setAgentRole("implementation");
    setAgentInstructions({ ...emptyAgentInstructions });
    setAgentEditorOpen(true);
  };
  const openEditAgent = (agent: AgentProfile) => {
    setEditingAgent(agent);
    setAgentName(agent.name);
    setAgentRole(agent.role);
    setAgentInstructions(parseAgentInstructions(agent.instructions));
    setAgentEditorOpen(true);
  };
  const closeAgentEditor = () => setAgentEditorOpen(false);
  const saveAgent = async (event: FormEvent) => {
    event.preventDefault();
    if (!agentReady) return;
    try {
      const instructions = compileAgentInstructions(agentInstructions);
      if (editingAgent) await harnessBridge.updateAgentProfile(editingAgent.id, agentName.trim(), agentRole.trim(), instructions);
      else await harnessBridge.createAgentProfile(project.path, agentName.trim(), agentRole.trim(), instructions);
      setAgentName("");
      setAgentRole("implementation");
      setAgentInstructions({ ...emptyAgentInstructions });
      setEditingAgent(undefined);
      setAgentEditorOpen(false);
      await refresh();
    } catch (error) { setNotice(String(error)); }
  };
  const deleteAgent = async (agent: AgentProfile) => {
    if (!window.confirm(`Delete ${agent.name}? Column and task instruction selections will be cleared.`)) return;
    try { await harnessBridge.deleteAgentProfile(agent.id); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const setColumnAgent = async (columnId: number, value: string) => {
    try { await harnessBridge.updateWorkflowColumn(columnId, undefined, undefined, value ? Number(value) : null); await refresh(); } catch (error) { setNotice(String(error)); }
  };

  return (
    <section className="agents-page">
      <div className="section-title agents-title"><div><Kicker>Rubyn's playbook</Kicker><h1>Agents</h1><p>Give Rubyn named sets of standing instructions for different kinds of work.</p></div><div className="agents-title-actions"><div className="agent-count"><strong>{projectData.agents.length}</strong><span>{projectData.agents.length === 1 ? "profile" : "profiles"}</span></div><button className="button primary" onClick={openCreateAgent}><Plus size={15} />Create agent</button></div></div>
      <section className="agent-team-panel">
        <div className="board-section-heading"><div><h2>Instruction profiles</h2><span>Rubyn adopts the selected profile when you start work from that column.</span></div></div>
        <div className="agent-profile-grid">{projectData.agents.map((agent) => {
          const assignments = projectData.columns.filter((column) => column.agentId === agent.id);
          return <article key={agent.id}><span>{agent.name.slice(0, 1).toUpperCase()}</span><div><strong>{agent.name}</strong><small>{agent.role}</small><p>{agent.instructions || "No standing instructions."}</p><em>{assignments.length ? `Used in ${assignments.map((column) => column.name).join(" · ")}` : "Not used by a column"}</em></div><div className="agent-profile-actions"><button className="icon-button" aria-label={`Edit ${agent.name}`} onClick={() => openEditAgent(agent)}><Pencil size={13} /></button><button className="icon-button" aria-label={`Delete ${agent.name}`} onClick={() => void deleteAgent(agent)}><Trash2 size={13} /></button></div></article>;
        })}</div>
      </section>
      <section className="handoff-panel">
        <div className="board-section-heading"><div><h2>Column handoffs</h2><span>Choose the standing instructions Rubyn uses when a task enters each column. Starting Rubyn remains your choice.</span></div></div>
        <div className="handoff-rail">{projectData.columns.map((column, index) => <div className="handoff-stop" key={column.id}>
          <span className="handoff-index">{String(index + 1).padStart(2, "0")}</span>
          <div className="handoff-name"><small>When a task enters</small><strong>{column.name}</strong></div>
          <span className="handoff-arrow"><ArrowRight size={15} /></span>
          <label>Rubyn uses<select aria-label={`Instructions for ${column.name}`} value={column.agentId || ""} onChange={(event) => void setColumnAgent(column.id, event.target.value)}><option value="">Default instructions</option>{projectData.agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name} instructions</option>)}</select></label>
        </div>)}</div>
      </section>
      {agentEditorOpen && <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) closeAgentEditor(); }}><form className="agent-creator" role="dialog" aria-modal="true" aria-labelledby="agent-editor-title" onSubmit={saveAgent}>
        <header><div><Kicker>Five clear answers</Kicker><h2 id="agent-editor-title">{editingAgent ? `Edit ${editingAgent.name}` : "Create an agent"}</h2><p>Rubyn will carry these standing instructions into every task that uses this profile.</p></div><div className="agent-creator-progress"><strong>{completedInstructionParts}/5</strong><span>instructions ready</span></div><button type="button" className="close" onClick={closeAgentEditor} aria-label="Close agent editor"><X size={18} /></button></header>
        <div className="agent-identity"><label>Name<input autoFocus aria-label="Agent name" value={agentName} onChange={(event) => setAgentName(event.target.value)} placeholder="Planner" /></label><label>Role<input aria-label="Agent role" value={agentRole} onChange={(event) => setAgentRole(event.target.value)} placeholder="planning" /></label></div>
        <div className="agent-creator-body"><div className="instruction-builder">{agentInstructionParts.map((part, index) => <label className={agentInstructions[part.key].trim() ? "complete" : ""} key={part.key}><span>{String(index + 1).padStart(2, "0")}</span><div><strong>{part.title}</strong><small>{part.question}</small><textarea aria-label={part.title} rows={3} value={agentInstructions[part.key]} onChange={(event) => setAgentInstructions((current) => ({ ...current, [part.key]: event.target.value }))} placeholder={part.placeholder} /></div></label>)}</div><aside className="agent-prompt-preview"><span>Rubyn receives</span><strong>{agentName.trim() || "Untitled agent"}</strong><small>{agentRole.trim() || "Choose a role"}</small><pre>{agentInstructionParts.map((part) => `${part.title}\n${agentInstructions[part.key].trim() || "…"}`).join("\n\n")}</pre></aside></div>
        <footer><span>All five answers are required.</span><div><button type="button" className="button quiet" onClick={closeAgentEditor}>Cancel</button><button className="button primary" disabled={!agentReady}>{editingAgent ? "Save changes" : "Create agent"}</button></div></footer>
      </form></div>}
    </section>
  );
}

function Workboard() {
  const { project, projectData, setNotice, openConversation, setNewConversationDraft, setNewConversationTaskId } = useHarnessStore();
  const refresh = useProjectRefresh();
  const [taskDraft, setTaskDraft] = useState("");
  const [taskDependencies, setTaskDependencies] = useState<number[]>([]);
  const [taskDetail, setTaskDetail] = useState("");
  const [taskOutcome, setTaskOutcome] = useState("");
  const [taskComposerOpen, setTaskComposerOpen] = useState(false);
  const [todoDraft, setTodoDraft] = useState("");
  const [columnDraft, setColumnDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [renamingColumnId, setRenamingColumnId] = useState<number>();
  const [renameDraft, setRenameDraft] = useState("");
  const [deletingColumnId, setDeletingColumnId] = useState<number>();
  useEffect(() => {
    if (!taskComposerOpen) return;
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") setTaskComposerOpen(false); };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [taskComposerOpen]);
  if (!project || !projectData) return <ProjectRequired />;

  const createTask = async (event: FormEvent) => {
    event.preventDefault();
    if (!taskDraft.trim()) return;
    setSaving(true);
    try {
      await harnessBridge.createTask(project.path, taskDraft.trim(), taskDetail.trim(), taskOutcome.trim(), taskDependencies);
      setTaskDraft("");
      setTaskDetail("");
      setTaskOutcome("");
      setTaskDependencies([]);
      setTaskComposerOpen(false);
      await refresh();
    } catch (error) { setNotice(String(error)); } finally { setSaving(false); }
  };
  const createTodo = async (event: FormEvent) => {
    event.preventDefault();
    if (!todoDraft.trim()) return;
    setSaving(true);
    try {
      await harnessBridge.createTodo(project.path, todoDraft.trim());
      setTodoDraft("");
      await refresh();
    } catch (error) { setNotice(String(error)); } finally { setSaving(false); }
  };
  const setTaskStatus = async (task: TaskRecord, status: TaskStatus) => {
    try { await harnessBridge.updateTask(task.id, status); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const createColumn = async (event: FormEvent) => {
    event.preventDefault();
    if (!columnDraft.trim()) return;
    try { await harnessBridge.createWorkflowColumn(project.path, columnDraft.trim()); setColumnDraft(""); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const renameColumn = async (event: FormEvent, id: number) => {
    event.preventDefault();
    const name = renameDraft.trim();
    if (!name) return;
    try { await harnessBridge.updateWorkflowColumn(id, name); setRenamingColumnId(undefined); setRenameDraft(""); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const moveColumn = async (id: number, position: number) => {
    try { await harnessBridge.updateWorkflowColumn(id, undefined, position); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const deleteColumn = async (id: number) => {
    const index = projectData.columns.findIndex((column) => column.id === id);
    const target = projectData.columns[index - 1] || projectData.columns[index + 1];
    if (!target) return;
    try { await harnessBridge.deleteWorkflowColumn(id, target.id); setDeletingColumnId(undefined); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const closeColumnMenu = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.currentTarget.closest("details")?.removeAttribute("open");
  };
  const assignTask = async (task: TaskRecord, value: string) => {
    try { await harnessBridge.updateTask(task.id, undefined, value ? Number(value) : null); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const assignTodo = async (todo: TodoRecord, value: string) => {
    try { await harnessBridge.updateTodo(todo.id, undefined, value ? Number(value) : null); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const setTodoStatus = async (todo: TodoRecord, status: WorkflowStatus) => {
    try { await harnessBridge.updateTodo(todo.id, status); await refresh(); } catch (error) { setNotice(String(error)); }
  };
  const runTask = (task: TaskRecord) => {
    const agent = projectData.agents.find((item) => item.id === task.assignedAgentId);
    setNewConversationTaskId(task.id);
    setNewConversationDraft(agentTaskPrompt(task, agent));
    openConversation(0);
  };

  return (
    <section>
      <div className="section-title board-title"><div><Kicker>Way finder</Kicker><h1>Tasks & todos</h1><p>Both boards write through to the native project store.</p></div></div>
      <div className="board-section-heading">
        <div><h2>Engineering tasks</h2><span>Durable work units for the graph and agent prompts.</span></div>
        <button className="button primary compact" onClick={() => setTaskComposerOpen(true)}><Plus size={15} />Create task</button>
      </div>
      <form className="column-add" onSubmit={createColumn}><input aria-label="New workflow column" value={columnDraft} onChange={(event) => setColumnDraft(event.target.value)} placeholder="Add a workflow column" /><button className="button quiet" disabled={!columnDraft.trim()}><Plus size={14} />Add column</button></form>
      <div className="board">
        {projectData.columns.map((column, index) => (
          <div className="board-column" key={column.id}>
            <div className="column-header"><span>{column.name}<small>Rubyn uses {projectData.agents.find((agent) => agent.id === column.agentId)?.name || "default"} instructions</small></span><b>{projectData.tasks.filter((task) => task.status === column.key).length}</b><details className="column-menu"><summary role="button" aria-label={`${column.name} column actions`}><MoreHorizontal size={16} /></summary><div><button aria-label={`Move ${column.name} left`} disabled={index === 0} onClick={(event) => { closeColumnMenu(event); void moveColumn(column.id, index - 1); }}>Move left</button><button aria-label={`Move ${column.name} right`} disabled={index === projectData.columns.length - 1} onClick={(event) => { closeColumnMenu(event); void moveColumn(column.id, index + 1); }}>Move right</button><button aria-label={`Rename ${column.name}`} onClick={(event) => { closeColumnMenu(event); setRenamingColumnId(column.id); setRenameDraft(column.name); }}>Rename</button><button aria-label={`Delete ${column.name}`} className="danger-text" disabled={projectData.columns.length === 1} onClick={(event) => { closeColumnMenu(event); setDeletingColumnId(column.id); }}>Delete…</button></div></details></div>
            {renamingColumnId === column.id && <form className="inline-column-editor" onSubmit={(event) => void renameColumn(event, column.id)}><input autoFocus aria-label={`New name for ${column.name}`} value={renameDraft} onChange={(event) => setRenameDraft(event.target.value)} /><button className="button primary compact" disabled={!renameDraft.trim()}>Save</button><button type="button" className="button quiet compact" onClick={() => setRenamingColumnId(undefined)}>Cancel</button></form>}
            {deletingColumnId === column.id && <div className="inline-confirm" role="alert"><p>Delete <strong>{column.name}</strong>? Its tasks move to {(projectData.columns[index - 1] || projectData.columns[index + 1])?.name}.</p><div><button className="button danger compact" onClick={() => void deleteColumn(column.id)}>Delete column</button><button className="button quiet compact" onClick={() => setDeletingColumnId(undefined)}>Keep it</button></div></div>}
            {projectData.tasks.filter((task) => task.status === column.key).map((task) => (
              <article className="todo-card" key={task.id}>
                <span className="todo-owner">R</span><strong>{task.title}</strong><span className="card-detail">Rubyn instructions: {projectData.agents.find((agent) => agent.id === task.assignedAgentId)?.name || "Default"}</span>{task.detail && <span className="card-detail">{task.detail}</span>}{task.outcome && <span className="task-outcome"><b>Outcome</b>{task.outcome}</span>}{task.dependsOn.length > 0 && <span className="card-detail">{task.ready ? "Dependencies complete" : `${task.dependsOn.length} ${task.dependsOn.length === 1 ? "dependency" : "dependencies"} open`}</span>}
                <label className="card-control">Column<select value={task.status} onChange={(event) => void setTaskStatus(task, event.target.value)}>{projectData.columns.map((option) => <option key={option.id} value={option.key}>{option.name}</option>)}</select></label>
                <label className="card-control">Background run<select value={task.assignedRunId || ""} onChange={(event) => void assignTask(task, event.target.value)}><option value="">Not started</option>{projectData.runs.map((run) => <option key={run.id} value={run.id}>Rubyn #{run.id} · {runLabel(run)} · {shortPrompt(run.prompt, 24)}</option>)}</select></label>
                <button className="task-run" onClick={() => task.assignedRunId ? openConversation(task.assignedRunId) : runTask(task)}>{task.assignedRunId ? <><MessageCircle size={14} />Open Rubyn #{task.assignedRunId}</> : <><Play size={14} />Start Rubyn</>}</button>
              </article>
            ))}
            {!projectData.tasks.some((task) => task.status === column.key) && <p className="column-empty">No tasks in {column.name.toLowerCase()}.</p>}
          </div>
        ))}
      </div>
      <div className="todo-board">
        <div className="board-section-heading">
          <div><h2>Shared next moves</h2><span>A small queue for you and the agents to keep momentum visible.</span></div>
          <form className="quick-add" onSubmit={createTodo}><input value={todoDraft} onChange={(event) => setTodoDraft(event.target.value)} placeholder="Add a next move" aria-label="New todo" /><button className="icon-button" aria-label="Add todo" disabled={saving || !todoDraft.trim()}><Plus size={17} /></button></form>
        </div>
        <div className="todo-ledger">
          {projectData.todos.map((todo) => (
            <div className="ledger-row" key={todo.id}>
              <span className={`todo-mark ${todo.status}`} aria-hidden="true">{todo.status === "done" ? <Check size={11} /> : null}</span>
              <span><strong>{todo.title}</strong><small>{todo.owner}</small></span>
              <label className="todo-agent">Agent<select aria-label={`Agent for ${todo.title}`} value={todo.assignedRunId || ""} onChange={(event) => void assignTodo(todo, event.target.value)}><option value="">Unassigned</option>{projectData.runs.map((run) => <option key={run.id} value={run.id}>Rubyn #{run.id} · {runLabel(run)}</option>)}</select></label>
              <label className="todo-status">Status<select aria-label={`Status for ${todo.title}`} value={todo.status} onChange={(event) => void setTodoStatus(todo, event.target.value as WorkflowStatus)}>{todoStatuses.map((status) => <option key={status.value} value={status.value}>{status.label}</option>)}</select></label>
            </div>
          ))}
          {!projectData.todos.length && <p className="panel-empty">No shared todos yet.</p>}
        </div>
      </div>
      {taskComposerOpen && <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setTaskComposerOpen(false); }}><form className="task-composer" role="dialog" aria-modal="true" aria-labelledby="create-task-title" onSubmit={createTask}><header><div><Kicker>Thoughtful work unit</Kicker><h2 id="create-task-title">Create an engineering task</h2><p>Give Rubyn enough context to understand the work and a concrete outcome to aim for.</p></div><button type="button" className="close" onClick={() => setTaskComposerOpen(false)} aria-label="Close task editor"><X size={18} /></button></header><label>Title<input autoFocus value={taskDraft} onChange={(event) => setTaskDraft(event.target.value)} placeholder="Add tenant-safe billing export" aria-label="Task title" /></label><label>Information <span>Context, constraints, and relevant details</span><textarea value={taskDetail} onChange={(event) => setTaskDetail(event.target.value)} rows={6} placeholder="Explain what Rubyn should know before starting…" aria-label="Task information" /></label><label>Expected outcome <span>What must be true when this task is complete</span><textarea value={taskOutcome} onChange={(event) => setTaskOutcome(event.target.value)} rows={4} placeholder="Describe the observable result or acceptance criteria…" aria-label="Task outcome" /></label>{projectData.tasks.length > 0 && <fieldset className="task-dependencies"><legend>Blocked by</legend><p>Select only work that must finish first.</p><div>{projectData.tasks.map((task) => <label key={task.id}><input type="checkbox" checked={taskDependencies.includes(task.id)} onChange={() => setTaskDependencies((current) => current.includes(task.id) ? current.filter((id) => id !== task.id) : [...current, task.id])} />{task.title}</label>)}</div></fieldset>}<footer><button type="button" className="button quiet" onClick={() => setTaskComposerOpen(false)}>Cancel</button><button className="button primary" disabled={saving || !taskDraft.trim() || !taskOutcome.trim()}>{saving ? "Creating…" : "Create task"}</button></footer></form></div>}
    </section>
  );
}

function Runs() {
  const { project, projectData, globalRuns, runEvents, engineState, appState, modelCatalog, activeConversationId, newConversationDraft, newConversationTaskId, conversationDrafts, newConversationAttachments, conversationAttachments, openConversation, setAppState, setNewConversationDraft, setNewConversationTaskId, setConversationDraft, setNewConversationAttachments, setConversationAttachments, setNotice, selectRun } = useHarnessStore();
  const refresh = useProjectRefresh();
  const [launching, setLaunching] = useState(false);
  const [fanoutPlanning, setFanoutPlanning] = useState(false);
  const [fanoutSelected, setFanoutSelected] = useState<number[]>([]);
  const [resolvingApproval, setResolvingApproval] = useState<string>();
  const messageViewportRef = useRef<HTMLDivElement>(null);
  const followedConversationRef = useRef<number | undefined>(undefined);
  const followLatestMessageRef = useRef(true);
  const activeCount = globalRuns.filter((run) => run.running).length;
  const readyTasks = projectData?.tasks.filter((task) => task.status === projectData.columns[0]?.key && task.ready) || [];
  const conversations = useMemo(() => [...(projectData?.runs || [])]
    .filter((conversation) => !conversation.archivedAt)
    .sort((left, right) => Number(Boolean(right.pinned)) - Number(Boolean(left.pinned)) || right.updatedAt - left.updatedAt), [projectData?.runs]);
  const composingNew = activeConversationId === 0 || conversations.length === 0;
  const selected = composingNew ? undefined : conversations.find((run) => run.id === activeConversationId) || conversations[0];
  const selectedId = selected?.id;
  const selectedEvents = selected ? runEvents[selected.id] || [] : [];
  const pendingApprovals = selected ? projectData?.approvals.filter((approval) => approval.runId === selected.id && approval.status === "pending") || [] : [];
  const message = selected ? conversationDrafts[selected.id] || "" : "";
  const replyAttachments = selected ? conversationAttachments[selected.id] || [] : [];
  const fanoutCapacity = Math.max(0, 3 - activeCount);
  const hasPersistedInitialMessage = Boolean(selected && selectedEvents.some((item) => item.kind === "chat/user" && eventPayload(item, "text") === selected.prompt));
  const modelKey = currentModelPreference(appState?.preferences.defaultModel);
  const connectedProviders = new Set(modelCatalog?.connectedProviders || []);
  const connectedModels = modelCatalog?.models.filter((item) => connectedProviders.has(item.provider)) || [];
  const selectedModel = connectedModels.find((item) => `${item.provider}::${item.model}` === modelKey)
    || connectedModels.find((item) => item.provider === modelCatalog?.activeProvider && item.model === modelCatalog?.activeModel)
    || connectedModels.find((item) => item.tier === "mid")
    || connectedModels[0];
  const turnActive = Boolean(selected?.running && selected.outcome === "running");
  const latestTimelineEventId = selectedEvents.at(-1)?.id;

  useLayoutEffect(() => {
    const viewport = messageViewportRef.current;
    if (!viewport || !selectedId) return;
    const changedConversation = followedConversationRef.current !== selectedId;
    followedConversationRef.current = selectedId;
    if (!changedConversation && !followLatestMessageRef.current) return;
    followLatestMessageRef.current = true;
    viewport.scrollTop = viewport.scrollHeight;
  }, [latestTimelineEventId, selectedId, turnActive]);

  const chooseModel = async (value: string) => {
    if (!appState) return;
    const next = { ...appState, preferences: { ...appState.preferences, defaultModel: value } };
    setAppState(next);
    try { setAppState(await harnessBridge.saveAppState(next)); } catch (error) { setNotice(String(error)); }
  };
  const launchWithModel = (prompt: string, attachments: AttachmentSelection[] = []) => {
    if (selectedModel) return harnessBridge.launchPrompt(project!.path, prompt, attachments, selectedModel);
    return attachments.length
      ? harnessBridge.launchPrompt(project!.path, prompt, attachments)
      : harnessBridge.launchPrompt(project!.path, prompt);
  };

  useEffect(() => {
    if (activeConversationId === undefined && conversations[0]) openConversation(conversations[0].id);
  }, [activeConversationId, conversations, openConversation]);

  useEffect(() => {
    if (!selectedId) return;
    const cursor = useHarnessStore.getState().eventCursors[selectedId] || 0;
    harnessBridge.pollRunEvents(selectedId, cursor).then((batch) => {
      useHarnessStore.getState().appendRunEvents(selectedId, batch.events, batch.nextEventId);
    }).catch(() => undefined);
  }, [selectedId]);

  useEffect(() => {
    const closeFanout = (event: KeyboardEvent) => {
      if (event.key === "Escape") setFanoutPlanning(false);
    };
    window.addEventListener("keydown", closeFanout);
    return () => window.removeEventListener("keydown", closeFanout);
  }, []);

  if (!project || !projectData) return <ProjectRequired />;

  const launch = async (event: FormEvent) => {
    event.preventDefault();
    if ((!newConversationDraft.trim() && !newConversationAttachments.length) || activeCount >= 3 || engineState !== "ready" || !selectedModel) return;
    setLaunching(true);
    try {
      const attachedTaskId = newConversationTaskId;
      const prompt = newConversationDraft.trim() || "Review the attached file or image.";
      const session = await launchWithModel(prompt, newConversationAttachments);
      openConversation(session.id);
      setNewConversationDraft("");
      setNewConversationAttachments([]);
      setNewConversationTaskId(undefined);
      if (attachedTaskId) {
        const implementing = projectData.columns.find((column) => column.key === "implementing") || projectData.columns[1] || projectData.columns[0];
        try {
          await harnessBridge.updateTask(attachedTaskId, implementing?.key, session.id);
        } catch (linkError) {
          try {
            await harnessBridge.stop(session.id);
            setNotice(`Conversation ${session.id} was stopped because its task could not be linked: ${String(linkError)}`);
          } catch (stopError) {
            setNotice(`Conversation ${session.id} could not be linked, and termination was not confirmed. Use End conversation immediately. Link error: ${String(linkError)}. Stop error: ${String(stopError)}`);
          }
          await refresh();
          return;
        }
      }
      setNotice(`Conversation ${session.id} started in an isolated worktree.`);
      await refresh();
    } catch (error) { setNotice(String(error)); } finally { setLaunching(false); }
  };
  const cancel = async (run: RunRecord) => {
    const completedTurn = run.outcome === "waiting";
    try {
      await harnessBridge.stop(run.id);
      setNotice(completedTurn ? "Conversation finished and ready for review." : "Rubyn's current turn stopped. The conversation remains open.");
      await refresh();
    } catch (error) { setNotice(String(error)); }
  };
  const resolveApproval = async (approval: EditApprovalRecord, accepted: boolean) => {
    setResolvingApproval(approval.editId);
    try {
      await harnessBridge.resolveEditApproval(approval.runId, approval.editId, accepted);
      setNotice(accepted ? `Approved ${approval.path}. Rubyn can apply the edit.` : `Denied ${approval.path}. Rubyn will continue without it.`);
      await refresh();
    } catch (error) {
      setNotice(String(error));
    } finally {
      setResolvingApproval(undefined);
    }
  };
  const reply = async (event: FormEvent) => {
    event.preventDefault();
    if (!selected || !selectedModel || (!message.trim() && !replyAttachments.length) || (!selected.running && activeCount >= 3)) return;
    const next = message.trim() || "Review the attached file or image.";
    followLatestMessageRef.current = true;
    setConversationDraft(selected.id, "");
    setConversationAttachments(selected.id, []);
    try {
      await harnessBridge.sendRunMessage(selected.id, next, replyAttachments, selectedModel);
      await refresh();
    } catch (error) { setConversationDraft(selected.id, message); setConversationAttachments(selected.id, replyAttachments); setNotice(String(error)); }
  };
  const pickAttachments = async (current: AttachmentSelection[], commit: (attachments: AttachmentSelection[]) => void) => {
    try {
      const picked = await harnessBridge.chooseAttachments();
      const unique = new Map([...current, ...picked].map((attachment) => [attachment.path, attachment]));
      const next = [...unique.values()];
      if (next.length > 10) { setNotice("Attach at most 10 files to one message."); return; }
      commit(next);
    } catch (error) { setNotice(String(error)); }
  };
  const fanOut = async () => {
    const batch = readyTasks.filter((task) => fanoutSelected.includes(task.id));
    if (!batch.length || engineState !== "ready") return;
    setLaunching(true);
    const results = await Promise.allSettled(batch.map(async (task) => {
      const session = await launchWithModel(taskPrompt(task));
      const implementing = projectData.columns.find((column) => column.key === "implementing") || projectData.columns[1] || projectData.columns[0];
      try {
        await harnessBridge.updateTask(task.id, implementing?.key, session.id);
        return session;
      } catch (linkError) {
        try {
          await harnessBridge.stop(session.id);
        } catch (stopError) {
          throw new Error(`Conversation ${session.id} could not be linked, and termination was not confirmed. Link error: ${String(linkError)}. Stop error: ${String(stopError)}`);
        }
        throw new Error(`Conversation ${session.id} was stopped because task ${task.id} could not be linked: ${String(linkError)}`);
      }
    }));
    const launched = results.filter((result) => result.status === "fulfilled").length;
    const failures = results.filter((result): result is PromiseRejectedResult => result.status === "rejected");
    setNotice(failures.length ? `Launched ${launched} isolated run${launched === 1 ? "" : "s"}. ${failures.length} launch/link failure${failures.length === 1 ? "" : "s"}: ${failures.map((failure) => String(failure.reason)).join(" · ")}` : `Launched ${launched} isolated run${launched === 1 ? "" : "s"}.`);
    setFanoutSelected([]);
    setFanoutPlanning(false);
    await refresh();
    setLaunching(false);
  };

  return (
    <section>
      <div className="talk-utility">{modelCatalog?.models.length ? <label className="model-picker">Model<select aria-label="Model for new conversations" value={selectedModel ? `${selectedModel.provider}::${selectedModel.model}` : ""} onChange={(event) => void chooseModel(event.target.value)}><option value="" disabled>Connect a model provider</option>{modelCatalog.models.map((item) => <option key={`${item.provider}::${item.model}`} value={`${item.provider}::${item.model}`} disabled={!connectedProviders.has(item.provider)}>{item.provider} / {item.model}{connectedProviders.has(item.provider) ? "" : " · connect first"}</option>)}</select></label> : null}{readyTasks.length > 0 && <button className="button quiet compact" onClick={() => setFanoutPlanning((open) => !open)} disabled={launching || activeCount >= 3 || engineState !== "ready" || !selectedModel}><Layers3 size={15} />Plan fan-out</button>}<span>{activeCount}/3 live</span></div>
      {fanoutPlanning && <div className="fanout-preflight"><div><strong>Choose parallel tasks</strong><small>{fanoutCapacity} slot{fanoutCapacity === 1 ? "" : "s"} available · isolated worktrees · Rubyn Code · provider cost unavailable</small></div><div className="fanout-choices">{readyTasks.map((task) => { const checked = fanoutSelected.includes(task.id); const full = !checked && fanoutSelected.length >= fanoutCapacity; return <label key={task.id}><input type="checkbox" checked={checked} disabled={full} onChange={() => setFanoutSelected((current) => checked ? current.filter((id) => id !== task.id) : [...current, task.id])} /><span><strong>{task.title}</strong><small>{task.dependsOn.length ? "Dependencies satisfied" : "No dependencies"}</small></span></label>; })}</div><footer><span>No tasks launch until you confirm this exact selection.</span><div><button className="button quiet compact" onClick={() => setFanoutPlanning(false)}>Cancel</button><button className="button primary compact" onClick={() => void fanOut()} disabled={!fanoutSelected.length || launching}>Launch {fanoutSelected.length || "selected"}</button></div></footer></div>}
      <div className="chat-layout conversation-shell">
        {selected ? <article className="chat-thread">
          <header><div><span className={`pulse ${statusOf(selected)}`} /><strong>{conversationTitle(selected, 80)}</strong><small>Conversation · {runLabel(selected)}{selected.background ? " · background task" : ""}</small></div><div>{turnActive ? <button className="button danger compact" onClick={() => void cancel(selected)}><CircleStop size={14} />Stop turn</button> : selected.running ? <button className="button primary compact" onClick={() => void cancel(selected)}><Check size={14} />Finish conversation</button> : <button className="button quiet compact" onClick={() => selectRun(selected.id)}><ShieldCheck size={14} />Review changes</button>}</div></header>
          <div ref={messageViewportRef} className="messages" aria-label="Conversation messages" aria-live="polite" onScroll={(event) => { const viewport = event.currentTarget; followLatestMessageRef.current = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 80; }}>{!hasPersistedInitialMessage && <ChatBubble role="user" text={selected.prompt} />}<ConversationTimeline events={selectedEvents} runId={selected.id} turnActive={turnActive} onAnswered={() => setNotice("Answer delivered to Rubyn.")} /></div>
          {pendingApprovals.length > 0 && <div className="edit-approval-stack" aria-label="Pending edit approvals">{pendingApprovals.map((approval) => <section className="edit-approval" key={approval.id}><header><div><span>Approval required · {approval.editType}</span><strong>{approval.path}</strong></div><FileCode2 size={17} /></header><pre>{approval.content}</pre><footer><span>The worktree is unchanged until you approve.</span><div><button className="button quiet compact" disabled={Boolean(resolvingApproval)} onClick={() => void resolveApproval(approval, false)}>Deny</button><button className="button primary compact" disabled={Boolean(resolvingApproval)} onClick={() => void resolveApproval(approval, true)}><Check size={14} />Approve edit</button></div></footer></section>)}</div>}
          {!hasTerminalLifecycle(selected.lifecycle) ? <form className="chat-composer" onSubmit={reply}><AttachmentTray attachments={replyAttachments} remove={(path) => setConversationAttachments(selected.id, replyAttachments.filter((item) => item.path !== path))} /><textarea aria-label="Message Rubyn" value={message} onChange={(event) => setConversationDraft(selected.id, event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} placeholder={turnActive ? "Queue guidance while Rubyn works…" : selected.outcome === "failed" ? "Retry with a connected model…" : "Continue this conversation…"} rows={3} /><div className="composer-actions"><button type="button" className="attach-button" aria-label="Attach images or files" onClick={() => void pickAttachments(replyAttachments, (items) => setConversationAttachments(selected.id, items))}><Paperclip size={17} />Attach</button><button className="button primary" disabled={(!message.trim() && !replyAttachments.length) || (!selected.running && activeCount >= 3) || engineState !== "ready" || !selectedModel}><ArrowUpRight size={16} />{turnActive ? "Queue" : selected.outcome === "failed" ? "Retry" : "Continue"}</button></div></form> : <div className="chat-ended">This worktree has already been {statusOf(selected)}.</div>}
        </article> : <article className="chat-thread start-thread"><div className="start-canvas"><div className="rubyn-orb">R</div><Kicker>{`Rubyn Code · ${project.name}`}</Kicker><h2>What should Rubyn work on?</h2><p>Talk naturally. Add images, code, or text context when it helps.</p><form className="chat-start-composer" onSubmit={launch}>{projectData.tasks.filter((task) => task.ready).length > 0 && <label className="task-attachment">Attached task<select aria-label="Attach a task" value={newConversationTaskId || ""} onChange={(event) => { const task = projectData.tasks.find((candidate) => candidate.id === Number(event.target.value)); setNewConversationTaskId(task?.id); if (task) setNewConversationDraft(taskPrompt(task)); }}><option value="">No task attached</option>{projectData.tasks.filter((task) => task.ready).map((task) => <option key={task.id} value={task.id}>{task.title}</option>)}</select></label>}<AttachmentTray attachments={newConversationAttachments} remove={(path) => setNewConversationAttachments(newConversationAttachments.filter((item) => item.path !== path))} /><textarea autoFocus aria-label="Prompt" value={newConversationDraft} onChange={(event) => setNewConversationDraft(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} placeholder="Ask Rubyn to plan, investigate, build, test, or review…" rows={4} /><footer><div className="composer-facts"><button type="button" className="attach-button" aria-label="Attach images or files" onClick={() => void pickAttachments(newConversationAttachments, setNewConversationAttachments)}><Paperclip size={16} />Attach files</button><span>isolated worktree</span></div><button className="send-orb" aria-label="Start conversation" disabled={launching || (!newConversationDraft.trim() && !newConversationAttachments.length) || activeCount >= 3 || !selectedModel}><ArrowUpRight size={18} /></button></footer></form></div></article>}
      </div>
    </section>
  );
}

function AttachmentTray({ attachments, remove }: { attachments: AttachmentSelection[]; remove: (path: string) => void }) {
  if (!attachments.length) return null;
  return <div className="attachment-tray" aria-label="Message attachments">{attachments.map((attachment) => <span className={`attachment-chip ${attachment.kind}`} key={attachment.path}><Paperclip size={13} /><b>{attachment.name}</b><button type="button" aria-label={`Remove ${attachment.name}`} onClick={() => remove(attachment.path)}><X size={13} /></button></span>)}</div>;
}

function eventPayload(event: { payload: unknown }, key: string): unknown {
  return event.payload && typeof event.payload === "object" ? (event.payload as Record<string, unknown>)[key] : undefined;
}

function eventAttachmentSummaries(event: { payload: unknown }): { name: string; kind: string }[] {
  const value = eventPayload(event, "attachments");
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is { name: string; kind: string } => Boolean(item && typeof item === "object" && typeof item.name === "string" && typeof item.kind === "string"));
}

function ChatBubble({ role, text, attachments = [], streaming = false }: { role: "user" | "assistant"; text: string; attachments?: { name: string; kind: string }[]; streaming?: boolean }) {
  if (!text && !attachments.length) return null;
  return <div className={`chat-bubble ${role}${streaming ? " streaming" : ""}`}><span>{role === "user" ? "You" : streaming ? <>Rubyn · writing <b>Live</b></> : "Rubyn"}</span>{text && <p>{text}</p>}{attachments.length > 0 && <div className="sent-attachments">{attachments.map((attachment, index) => <b key={`${attachment.name}-${index}`}><Paperclip size={12} />{attachment.name}<small>{attachment.kind}</small></b>)}</div>}</div>;
}

function activityKey(event: RunEventRecord) {
  return String(eventPayload(event, "requestId") ?? eventPayload(event, "itemId") ?? event.id);
}

function toolLabel(tool: string) {
  const labels: Record<string, string> = {
    bash: "Run command",
    shell: "Run command",
    read_file: "Read file",
    write_file: "Write file",
    edit_file: "Edit file",
    file_change: "Change files",
    grep: "Search code",
    glob: "Find files",
    web_search: "Search the web",
    web_fetch: "Open web page",
    run_specs: "Run tests",
    spawnAgent: "Start agent",
    sendInput: "Guide agent",
    wait: "Wait for agent",
    wayfinder: "Update Wayfinder",
  };
  return labels[tool] || tool.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function compactActivityValue(value: unknown, maximum = 220) {
  if (value === undefined || value === null || value === "") return "";
  const text = typeof value === "string" ? value : JSON.stringify(value);
  return text.length > maximum ? `${text.slice(0, maximum - 1)}…` : text;
}

function toolIntent(event: RunEventRecord) {
  const args = eventPayload(event, "args");
  const values = args && typeof args === "object" ? args as Record<string, unknown> : {};
  const direct = values.command ?? values.path ?? values.pattern ?? values.query ?? values.url ?? values.prompt;
  if (direct) return compactActivityValue(direct);
  const changes = values.changes;
  if (Array.isArray(changes)) {
    const paths = changes.map((change) => change && typeof change === "object" ? String((change as Record<string, unknown>).path || "") : "").filter(Boolean);
    if (paths.length) return compactActivityValue(paths.join(", "));
  }
  return compactActivityValue(args);
}

function ToolActivity({ event, result, progress }: { event: RunEventRecord; result?: RunEventRecord; progress: string }) {
  const tool = String(eventPayload(event, "tool") || "tool");
  const success = result ? eventPayload(result, "success") !== false : undefined;
  const summary = result ? String(eventPayload(result, "summary") || (success ? "Finished" : "Failed")) : "Working…";
  const Icon = tool === "shell" || tool === "bash" ? TerminalSquare : tool.includes("file") ? FileCode2 : tool.includes("search") || tool === "grep" || tool === "glob" ? Search : Activity;
  return <details className={`tool-activity ${result ? success ? "success" : "failed" : "running"}`}>
    <summary><span className="tool-activity-icon"><Icon size={14} /></span><span><strong>{toolLabel(tool)}</strong><small>{result ? summary : toolIntent(event) || summary}</small></span><b>{result ? success ? "Done" : "Failed" : "Live"}</b></summary>
    <div className="tool-activity-detail">{toolIntent(event) && <code>{toolIntent(event)}</code>}{progress && <pre>{progress.slice(-2400)}</pre>}</div>
  </details>;
}

function ReasoningActivity({ text, active }: { text: string; active: boolean }) {
  return <div className={`reasoning-activity ${active ? "live" : ""}`}><span><Sparkles size={13} />Thinking{active && <b>Live</b>}</span><p>{text}</p></div>;
}

function AssistantTurn({ events, runId, active, onAnswered }: { events: RunEventRecord[]; runId: number; active: boolean; onAnswered: () => void }) {
  const results = new Map(events.filter((event) => event.kind === "tool/result").map((event) => [activityKey(event), event]));
  const progress = new Map<string, string>();
  for (const event of events.filter((candidate) => candidate.kind === "tool/progress")) {
    const key = activityKey(event);
    progress.set(key, `${progress.get(key) || ""}${String(eventPayload(event, "text") || "")}`);
  }
  const reasoning = new Map<string, { first: number; text: string }>();
  for (const event of events.filter((candidate) => candidate.kind === "reasoning/delta")) {
    const key = activityKey(event);
    const current = reasoning.get(key);
    reasoning.set(key, { first: current?.first ?? event.id, text: `${current?.text || ""}${String(eventPayload(event, "text") || "")}` });
  }
  const finalEvent = [...events].reverse().find((event) => event.kind === "stream/text" && eventPayload(event, "final") === true);
  const liveText = events.filter((event) => event.kind === "stream/text" && eventPayload(event, "final") !== true).map((event) => String(eventPayload(event, "text") || "")).join("");
  const responseText = finalEvent ? String(eventPayload(finalEvent, "text") || "") : liveText;
  const error = [...events].reverse().find((event) => event.kind === "agent/status" && eventPayload(event, "status") === "error");

  return <>
    {events.map((event) => {
      if (event.kind === "ide/askUser") return <EngineQuestionCard key={event.id} runId={runId} payload={event.payload} onAnswered={onAnswered} />;
      if (event.kind === "tool/use") {
        const key = activityKey(event);
        return <ToolActivity key={event.id} event={event} result={results.get(key)} progress={progress.get(key) || ""} />;
      }
      if (event.kind === "reasoning/delta") {
        const group = reasoning.get(activityKey(event));
        return group?.first === event.id && group.text.trim() ? <ReasoningActivity key={event.id} text={group.text} active={active} /> : null;
      }
      return null;
    })}
    {error ? <ChatBubble role="assistant" text={`Provider error: ${String(eventPayload(error, "error") || "The model request failed.")}`} /> : responseText ? <ChatBubble role="assistant" text={responseText} streaming={active && !finalEvent} /> : null}
    {active && <div className="rubyn-thinking"><span className="live-dot" />{responseText ? "Building the response…" : "Thinking through the next move…"}</div>}
  </>;
}

function ConversationTimeline({ events, runId, turnActive, onAnswered }: { events: RunEventRecord[]; runId: number; turnActive: boolean; onAnswered: () => void }) {
  const rendered: ReactNode[] = [];
  let turnEvents: RunEventRecord[] = [];
  let turnKey = "opening-turn";
  const flushTurn = (active: boolean) => {
    if (turnEvents.length || active) rendered.push(<AssistantTurn key={turnKey} events={turnEvents} runId={runId} active={active} onAnswered={onAnswered} />);
    turnEvents = [];
  };

  for (const event of events) {
    if (event.kind === "chat/user") {
      flushTurn(false);
      rendered.push(<ChatBubble key={`user-${event.id}`} role="user" text={String(eventPayload(event, "text") || "")} attachments={eventAttachmentSummaries(event)} />);
      turnKey = `turn-after-${event.id}`;
    } else {
      turnEvents.push(event);
    }
  }
  flushTurn(turnActive);
  return <>{rendered}</>;
}

function EngineQuestionCard({ runId, payload, onAnswered }: { runId: number; payload: unknown; onAnswered: () => void }) {
  const data = payload && typeof payload === "object" ? payload as Record<string, unknown> : {};
  const requestId = data.requestId;
  const questions = (Array.isArray(data.questions) && data.questions.length ? data.questions : [{ prompt: data.question, options: data.options }]).slice(0, 3).map((item, questionIndex) => {
    const question = item && typeof item === "object" ? item as Record<string, unknown> : {};
    return { id: String(question.id ?? questionIndex), prompt: String(question.prompt || question.question || data.question || "Rubyn needs a decision before continuing."), cardinality: question.cardinality === "multiple" ? "multiple" : "single", options: (Array.isArray(question.options) ? question.options : []).map((option, index) => typeof option === "string" ? { id: String(index), label: option, description: "" } : option as Record<string, unknown>) };
  });
  const [drafts, setDrafts] = useState<Record<string, { selected: string[]; custom: string }>>({});
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    if (requestId === undefined || !questions.every((question) => drafts[question.id]?.selected.length || drafts[question.id]?.custom.trim())) return;
    setBusy(true);
    try {
      await harnessBridge.answerEngineQuestion(runId, requestId as string | number, { questions: questions.map((question) => ({ id: question.id, selected: drafts[question.id]?.selected || [], custom: drafts[question.id]?.custom.trim() || "" })) });
      setSent(true); onAnswered();
    } finally { setBusy(false); }
  };
  return <section className="engine-question" aria-label="Rubyn question"><span>ASK USER · {questions.length} {questions.length === 1 ? "QUESTION" : "QUESTIONS"}</span>{questions.map((question) => { const draft = drafts[question.id] || { selected: [], custom: "" }; return <fieldset key={question.id}><legend>{question.prompt}</legend>{question.options.length > 0 && <div>{question.options.map((option, index) => { const id = String(option.id ?? index); const checked = draft.selected.includes(id); return <label className={checked ? "selected" : ""} key={id}><input type={question.cardinality === "multiple" ? "checkbox" : "radio"} name={`engine-question-${runId}-${String(requestId)}-${question.id}`} checked={checked} disabled={sent} onChange={() => setDrafts((current) => ({ ...current, [question.id]: { ...draft, selected: question.cardinality === "multiple" ? checked ? draft.selected.filter((value) => value !== id) : [...draft.selected, id] : [id] } }))} /><strong>{String(option.label || option.title || id)}{option.recommended === true && <b>Recommended</b>}</strong>{Boolean(option.description) && <small>{String(option.description)}</small>}{Boolean(option.pros) && <small>Pro: {String(option.pros)}</small>}{Boolean(option.cons) && <small>Con: {String(option.cons)}</small>}</label>; })}</div>}<label className="engine-question-freeform">Add context or answer freely<textarea rows={2} value={draft.custom} disabled={sent} onChange={(event) => setDrafts((current) => ({ ...current, [question.id]: { ...draft, custom: event.target.value } }))} /></label></fieldset>; })}<button className="button primary compact" disabled={sent || busy || !questions.every((question) => drafts[question.id]?.selected.length || drafts[question.id]?.custom.trim())} onClick={() => void submit()}>{sent ? "Answer sent" : busy ? "Sending…" : "Answer Rubyn"}</button></section>;
}

function Skills() {
  const bundledSkills = useHarnessStore((state) => state.skills);
  const project = useHarnessStore((state) => state.project);
  const setNotice = useHarnessStore((state) => state.setNotice);
  const [projectSkills, setProjectSkills] = useState<typeof bundledSkills>([]);
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");
  const [inspecting, setInspecting] = useState<{ name: string; path: string; content: string; project: boolean }>();
  const [loadingSkill, setLoadingSkill] = useState(false);
  const skills = [...projectSkills, ...bundledSkills];
  const filtered = skills.filter((skill) => `${skill.name} ${skill.description}`.toLowerCase().includes(search.toLowerCase()));
  useEffect(() => {
    if (!project) { setProjectSkills([]); return; }
    harnessBridge.listProjectSkills(project.path).then(setProjectSkills).catch(() => setProjectSkills([]));
  }, [project]);
  useEffect(() => {
    if (!creating && !inspecting) return;
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") { setCreating(false); setInspecting(undefined); } };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [creating, inspecting]);
  const create = async (event: FormEvent) => {
    event.preventDefault();
    if (!project || !name.trim() || !content.trim()) return;
    try {
      const skill = await harnessBridge.createProjectSkill(project.path, name.trim(), content.trim());
      setProjectSkills((current) => [skill, ...current]);
      setName(""); setContent(""); setCreating(false);
      setNotice(`${skill.name} was created in ${project.name}. Commit it so detached Rubyn worktrees load it.`);
    } catch (error) { setNotice(String(error)); }
  };
  const inspect = async (skill: (typeof skills)[number], isProjectSkill: boolean) => {
    setLoadingSkill(true);
    try {
      const detail = await harnessBridge.readSkill(skill.path, isProjectSkill ? project?.path : undefined);
      setInspecting({ name: skill.name, path: skill.path, content: detail.content, project: isProjectSkill });
    } catch (error) { setNotice(String(error)); } finally { setLoadingSkill(false); }
  };
  return (
    <section>
      <div className="section-title inline-title"><div><Kicker>Rubyn Code backbone</Kicker><h1>Skills</h1><p>Bundled guidance plus project-local Rubyn skills. Commit new project skills before launching detached runs.</p></div>{project && <button className="button primary" onClick={() => setCreating(true)}><FileCode2 size={15} />Create project skill</button>}</div>
      <div className="skill-toolbar"><Search size={17} /><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Find a bundled skill" aria-label="Search bundled skills" /><span>{skills.length} source{skills.length === 1 ? "" : "s"}</span></div>
      {filtered.length ? (
        <div className="skills-grid">
          {filtered.map((skill) => { const isProjectSkill = projectSkills.includes(skill); return <article className="skill-card" key={`${isProjectSkill ? "project" : "bundled"}-${skill.path}`}><span className="skill-source"><FileCode2 size={13} />{isProjectSkill ? "Project skill" : "Bundled skill"}</span><h2>{skill.name}</h2><p>{skill.description || "No description in the skill source."}</p><small>{skill.path}</small><button className="skill-inspect-button" onClick={() => void inspect(skill, isProjectSkill)} disabled={loadingSkill}>Read skill <ArrowRight size={13} /></button></article>; })}
        </div>
      ) : <EmptyState title={search ? "No matching bundled skill" : "No bundled skills found"}>{search ? "Try a broader search." : "This build did not expose a Rubyn skill source inventory."}</EmptyState>}
      {creating && <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setCreating(false); }}><form className="composer" role="dialog" aria-modal="true" aria-labelledby="skill-title" onSubmit={create}><button type="button" className="close" onClick={() => setCreating(false)} aria-label="Close"><X size={18} /></button><Kicker>Project skill</Kicker><h2 id="skill-title">Create reusable Rubyn guidance.</h2><label>Name<input autoFocus value={name} onChange={(event) => setName(event.target.value)} placeholder="Safe Rails migrations" /></label><label>Guidance<textarea value={content} onChange={(event) => setContent(event.target.value)} rows={10} placeholder="Describe when to use the skill, constraints, steps, and evidence required." /></label><button className="button primary" disabled={!name.trim() || !content.trim()}><FileCode2 size={15} />Install project skill</button></form></div>}
      {inspecting && <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setInspecting(undefined); }}><article className="skill-inspector" role="dialog" aria-modal="true" aria-labelledby="inspect-skill-title"><header><div><span>{inspecting.project ? "Project skill" : "Bundled skill"}</span><h2 id="inspect-skill-title">{inspecting.name}</h2><small>{inspecting.path}</small></div><button onClick={() => setInspecting(undefined)} aria-label="Close skill"><X size={18} /></button></header><pre>{inspecting.content}</pre></article></div>}
    </section>
  );
}

function Review() {
  const { project, projectData, selectedRunId, selectRun, setNotice } = useHarnessStore();
  const refresh = useProjectRefresh();
  const [inspection, setInspection] = useState<RunWorktreeInspection>();
  const [loading, setLoading] = useState(false);
  const [acting, setActing] = useState(false);
  const [confirmingDiscard, setConfirmingDiscard] = useState(false);
  const runs = projectData?.runs || [];
  const selected = runs.find((run) => run.id === selectedRunId) || runs[0];
  const selectedId = selected?.id;
  const selectedRunning = selected?.running;
  const selectedLifecycle = selected?.lifecycle;

  useEffect(() => {
    setInspection(undefined);
    setConfirmingDiscard(false);
    if (!selectedId || selectedRunning || (selectedLifecycle && hasTerminalLifecycle(selectedLifecycle))) return;
    let current = true;
    setLoading(true);
    harnessBridge.inspectRunWorktree(selectedId)
      .then((result) => { if (current) setInspection(result); })
      .catch((error) => { if (current) setNotice(String(error)); })
      .finally(() => { if (current) setLoading(false); });
    return () => { current = false; };
  }, [selectedId, selectedRunning, selectedLifecycle, setNotice]);

  if (!project || !projectData) return <ProjectRequired />;
  if (!selected) return <EmptyState title="Nothing to review">Conversations with retained worktrees will appear here after they finish.</EmptyState>;

  const integrate = async () => {
    setActing(true);
    try {
      const result = await harnessBridge.integrateRun(selected.id);
      setNotice(result.commitOid ? `Integrated commit ${result.commitOid.slice(0, 10)}.` : "Worktree integrated.");
      setInspection(undefined);
      await refresh();
    } catch (error) { setNotice(String(error)); } finally { setActing(false); }
  };
  const discard = async () => {
    setActing(true);
    try {
      const result = await harnessBridge.discardRun(selected.id);
      setNotice(result.cleanupPending ? "Worktree discarded; cleanup will retry." : "Worktree discarded and cleaned up.");
      setInspection(undefined);
      await refresh();
    } catch (error) { setNotice(String(error)); } finally { setActing(false); }
  };

  const actionable = !selected.running && !hasTerminalLifecycle(selected.lifecycle);
  return (
    <section>
      <div className="section-title inline-title"><div><Kicker>Real worktree evidence</Kicker><h1>Review</h1><p>Inspect a conversation’s Git status and unified diff before choosing what happens to its worktree.</p></div><label className="run-selector">Conversation<select value={selected.id} onChange={(event) => selectRun(Number(event.target.value))}>{runs.map((conversation) => <option key={conversation.id} value={conversation.id}>{conversationTitle(conversation, 48)} · {runLabel(conversation)}</option>)}</select></label></div>
      <div className="review-layout">
        <article className="diff-panel">
          <div className="diff-header"><span><GitBranch size={15} />{conversationTitle(selected, 48)} worktree</span><span>{inspection?.diff.truncated ? "Truncated at native limit" : inspection ? `${inspection.status.files.length} changed file${inspection.status.files.length === 1 ? "" : "s"}` : runLabel(selected)}</span></div>
          {loading ? <div className="diff-loading"><RefreshCw size={17} />Inspecting worktree…</div> : inspection?.diff.diff ? <DiffView diff={inspection.diff.diff} /> : <div className="diff-empty">{selected.running ? "This conversation is still active. Its diff becomes actionable after it stops." : actionable ? "No changes are present in this worktree." : `This worktree is ${runLabel(selected)}.`}</div>}
        </article>
        <aside className="review-notes">
          <Kicker>Worktree disposition</Kicker><h2>{conversationTitle(selected, 90)}</h2>
          <dl className="run-facts"><div><dt>Status</dt><dd>{runLabel(selected)}</dd></div><div><dt>Base</dt><dd>{selected.baseCommit.slice(0, 12) || "unknown"}</dd></div><div><dt>Files</dt><dd>{inspection?.status.files.length ?? "—"}</dd></div></dl>
          <p className="review-path">{selected.worktreePath}</p>
          {inspection?.status.files.length ? <div className="changed-files"><strong>Changed files</strong>{inspection.status.files.map((file) => <span key={file.path}><FileCode2 size={13} />{file.path}<b>{`${file.indexStatus}${file.worktreeStatus}`.trim() || "M"}</b></span>)}</div> : null}
          {actionable && !confirmingDiscard && <div className="review-actions"><button className="button primary" onClick={() => void integrate()} disabled={acting || loading || !inspection?.status.files.length}><Check size={16} />Integrate</button><button className="button danger" onClick={() => setConfirmingDiscard(true)} disabled={acting || loading}><Trash2 size={15} />Discard</button></div>}
          {actionable && confirmingDiscard && <div className="discard-confirm" role="alert"><strong>Discard this worktree?</strong><p>This removes the isolated worktree. The source project is not changed.</p><div><button className="button danger compact" onClick={() => void discard()} disabled={acting}>Remove worktree</button><button className="button quiet compact" onClick={() => setConfirmingDiscard(false)} disabled={acting}>Keep worktree</button></div></div>}
          {selected.integratedCommit && <p className="commit-result">Integrated as {selected.integratedCommit}</p>}
        </aside>
      </div>
    </section>
  );
}

function DiffView({ diff }: { diff: string }) {
  return <pre aria-label="Unified diff"><code>{diff.split("\n").map((line, index) => {
    const kind = line.startsWith("+++") || line.startsWith("---") || line.startsWith("diff --git") || line.startsWith("@@") ? "meta" : line.startsWith("+") ? "add" : line.startsWith("-") ? "remove" : "neutral";
    return <span className={`line ${kind}`} key={index}>{line || " "}</span>;
  })}</code></pre>;
}

type AccountChoice = "codex" | "anthropic" | "openai" | "minimax" | "custom";

const accountPresets: Record<Exclude<AccountChoice, "codex" | "custom">, { name: string; mark: string; help: string; baseUrl: string; apiFormat: "openai" | "anthropic"; models: string[]; keyHint: string }> = {
  anthropic: { name: "Anthropic", mark: "A", help: "Use Claude with an Anthropic API key.", baseUrl: "https://api.anthropic.com/v1", apiFormat: "anthropic", models: ["claude-fable-5", "claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"], keyHint: "Starts with sk-ant-" },
  openai: { name: "OpenAI", mark: "O", help: "Use GPT models with an OpenAI API key.", baseUrl: "https://api.openai.com/v1", apiFormat: "openai", models: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"], keyHint: "Starts with sk-" },
  minimax: { name: "MiniMax", mark: "M", help: "Use MiniMax models with one API key.", baseUrl: "https://api.minimax.io/v1", apiFormat: "openai", models: ["MiniMax-M3", "MiniMax-M2.7-highspeed", "MiniMax-M2.7"], keyHint: "Paste your MiniMax API key" },
};

function Accounts() {
  const { modelCatalog, setModelCatalog, setNotice, setView } = useHarnessStore();
  const [choice, setChoice] = useState<AccountChoice>();
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [signingIntoCodex, setSigningIntoCodex] = useState(false);
  const [customName, setCustomName] = useState("");
  const [customUrl, setCustomUrl] = useState("");
  const [customFormat, setCustomFormat] = useState<"openai" | "anthropic">("openai");
  const [customModels, setCustomModels] = useState("");
  const connected = new Set(modelCatalog?.connectedProviders || []);
  const providerCount = new Set(modelCatalog?.models.map((item) => item.provider) || []).size;

  const saveKey = async (event: FormEvent) => {
    event.preventDefault();
    if (!choice || choice === "codex" || !apiKey.trim()) return;
    const preset = choice === "custom" ? undefined : accountPresets[choice];
    const models = preset?.models || customModels.split(",").map((model) => model.trim()).filter(Boolean);
    const name = preset ? choice : customName.trim().toLowerCase();
    const baseUrl = preset?.baseUrl || customUrl.trim();
    const apiFormat = preset?.apiFormat || customFormat;
    if (!name || !baseUrl || !models.length) return;
    setSaving(true);
    try {
      const catalog = await harnessBridge.upsertProvider({ name, baseUrl, apiFormat, envKey: "", apiKey: apiKey.trim(), models });
      setModelCatalog(catalog);
      setApiKey("");
      setChoice(undefined);
      setNotice(`${preset?.name || customName.trim()} is connected. Pick one of its models in Talk to Rubyn.`);
    } catch (error) { setNotice(String(error)); } finally { setSaving(false); }
  };

  const signIntoCodex = async () => {
    setSigningIntoCodex(true);
    try {
      await harnessBridge.startCodexLogin();
      setNotice("Your browser is opening. Sign in there, then come back here.");
      for (let attempt = 0; attempt < 60; attempt += 1) {
        await new Promise((resolve) => window.setTimeout(resolve, 2000));
        const catalog = await harnessBridge.listModels();
        if (catalog.connectedProviders.includes("codex")) {
          setModelCatalog(catalog);
          setChoice(undefined);
          setNotice("Codex is connected. You're ready to talk to Rubyn.");
          return;
        }
      }
      setNotice("Sign-in is not finished yet. Choose Codex and try again when you're ready.");
    } catch (error) { setNotice(String(error)); } finally { setSigningIntoCodex(false); }
  };

  const choices: { id: AccountChoice; name: string; mark: string; help: string }[] = [
    { id: "codex", name: "Codex", mark: "C", help: "Sign in with your ChatGPT account. No key to copy." },
    ...Object.entries(accountPresets).map(([id, preset]) => ({ id: id as AccountChoice, name: preset.name, mark: preset.mark, help: preset.help })),
    { id: "custom", name: "Something else", mark: "+", help: "Connect another OpenAI- or Anthropic-compatible service." },
  ];

  return (
    <section className="accounts-page">
      <div className="section-title"><Kicker>One key. One click. Done.</Kicker><h1>Models & accounts</h1><p>Connect the brain Rubyn should use. Choose a company below and follow the one short step.</p></div>
      <div className="account-overview"><div className="connection-orb"><KeyRound size={22} /><span>{connected.size}</span></div><div><strong>{connected.size ? `${connected.size} connected` : "Connect your first model"}</strong><small>{providerCount} model services available · keys are locked on this computer</small></div><button className="button quiet compact" onClick={() => setView("agents")}>Go to Talk to Rubyn <ArrowRight size={14} /></button></div>
      {!choice ? <div className="account-grid" aria-label="Choose a model service">{choices.map((item) => {
        const isConnected = connected.has(item.id);
        return <button key={item.id} aria-label={item.name} className="account-choice" onClick={() => setChoice(item.id)}><span className={`account-mark ${item.id}`}>{item.mark}</span><span><strong>{item.name}</strong><small>{item.help}</small></span><b className={isConnected ? "connected" : ""}>{isConnected ? <><Check size={12} /> Connected</> : <>Set up <ChevronRight size={13} /></>}</b></button>;
      })}</div> : <article className="account-setup">
        <button className="account-back" onClick={() => { setChoice(undefined); setApiKey(""); }}><ChevronRight size={14} />All services</button>
        {choice === "codex" ? <div className="account-step"><span className="account-mark codex">C</span><Kicker>Codex</Kicker><h2>Sign in with ChatGPT</h2><p>Press the button. A browser window will open. Sign in, then come back—Rubyn will finish the rest.</p><button className="button primary" onClick={() => void signIntoCodex()} disabled={signingIntoCodex}>{signingIntoCodex ? <><RefreshCw size={15} />Waiting for you…</> : <>Open ChatGPT sign-in <ArrowUpRight size={15} /></>}</button><small>Rubyn Harness cannot see your password or ChatGPT token.</small></div> : <form className="account-step" onSubmit={saveKey}>
          <span className={`account-mark ${choice}`}>{choice === "custom" ? "+" : accountPresets[choice].mark}</span><Kicker>{choice === "custom" ? "Another service" : accountPresets[choice].name}</Kicker><h2>{choice === "custom" ? "Tell Rubyn where to connect" : `Paste your ${accountPresets[choice].name} key`}</h2><p>{choice === "custom" ? "These details are usually on the service's API page." : "Copy the key from your account page and paste it below. We'll lock it on this computer."}</p>
          {choice === "custom" && <div className="custom-account-fields"><label>Service name<input value={customName} onChange={(event) => setCustomName(event.target.value)} placeholder="My model service" /></label><label>Connection type<select value={customFormat} onChange={(event) => setCustomFormat(event.target.value as "openai" | "anthropic")}><option value="openai">Works like OpenAI</option><option value="anthropic">Works like Anthropic</option></select></label><label>Web address<input value={customUrl} onChange={(event) => setCustomUrl(event.target.value)} placeholder="https://api.example.com/v1" /></label><label>Model names<input value={customModels} onChange={(event) => setCustomModels(event.target.value)} placeholder="model-one, model-two" /><small>Separate names with commas.</small></label></div>}
          <label className="key-field">Secret key<input aria-label="Secret key" type="password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} placeholder={choice === "custom" ? "Paste the key here" : accountPresets[choice].keyHint} autoComplete="new-password" /><small>Stored encrypted. It never goes into project files.</small></label>
          <button className="button primary" disabled={saving || !apiKey.trim() || (choice === "custom" && (!customName.trim() || !customUrl.trim() || !customModels.trim()))}>{saving ? <><RefreshCw size={15} />Saving safely…</> : <><KeyRound size={15} />Save and connect</>}</button>
        </form>}
      </article>}
    </section>
  );
}

function Projects() {
  const { project, appState, modelCatalog, engineState, engineDetail, reducedMotion, setReducedMotion, setProject, setProjectData, setAppState, setNotice, openConversation } = useHarnessStore();
  const [path, setPath] = useState("");
  const [checking, setChecking] = useState(false);
  const [chiselMode, setChiselMode] = useState<"off" | "lite" | "full" | "ultra">();
  const [savingChisel, setSavingChisel] = useState(false);

  useEffect(() => {
    let current = true;
    harnessBridge.getChiselMode().then((mode) => { if (current) setChiselMode(mode); }).catch(() => undefined);
    return () => { current = false; };
  }, []);

  const choose = async (projectPath: string) => {
    setChecking(true);
    try {
      const inspected = await harnessBridge.inspectProject(projectPath.trim());
      const data = await harnessBridge.projectData(inspected.path);
      const state = appState || await harnessBridge.appState();
      const nextState: LocalAppState = {
        ...state,
        recentProjects: [
          { path: inspected.path, name: inspected.name },
          ...state.recentProjects.filter((recent) => recent.path !== inspected.path),
        ].slice(0, 24),
      };
      setProject(inspected);
      setProjectData(data);
      setAppState(await harnessBridge.saveAppState(nextState));
      setPath("");
      setNotice(`${inspected.isRails ? "Rails" : inspected.isRuby ? "Ruby" : "Git"} project ready.`);
      openConversation(data.runs[0]?.id || 0);
    } catch (error) { setNotice(String(error)); } finally { setChecking(false); }
  };
  const submit = (event: FormEvent) => { event.preventDefault(); if (path.trim()) void choose(path); };
  const browse = async () => {
    try {
      const selected = await harnessBridge.chooseProjectFolder();
      if (selected) await choose(selected);
    } catch (error) { setNotice(String(error)); }
  };
  const toggleChisel = async () => {
    if (!chiselMode) return;
    const enabled = chiselMode === "off";
    setSavingChisel(true);
    try {
      const mode = await harnessBridge.setChiselEnabled(enabled);
      setChiselMode(mode);
      setNotice(enabled ? "Chisel is on. Rubyn will prefer the smallest change that works." : "Chisel is off.");
    } catch (error) { setNotice(String(error)); } finally { setSavingChisel(false); }
  };

  return (
    <section>
      <div className="section-title"><Kicker>Explicit workspace boundary</Kicker><h1>Projects</h1><p>Choose a local Ruby or Rails folder. Rubyn Harness validates the repository before any agent can run.</p></div>
      <div className="project-layout">
        <article className="project-list">
          <div className="project-picker"><span className="project-picker-gem"><FolderKanban size={25} /></span><div><strong>Open a project</strong><small>Pick the folder that contains your Gemfile or Git repository.</small></div><button className="button primary" onClick={() => void browse()} disabled={checking}>{checking ? "Inspecting…" : "Choose project folder"}</button></div>
          {appState?.recentProjects.map((recent) => <button key={recent.path} className={project?.path === recent.path ? "selected" : ""} onClick={() => void choose(recent.path)} disabled={checking}><span><FolderKanban size={17} />{recent.name}</span><small>{project?.path === recent.path ? "active" : "open"}</small></button>)}
          {!appState?.recentProjects.length && <p className="project-empty">No recent projects.</p>}
          <details className="advanced-path"><summary>Enter a path manually</summary><form className="project-path" onSubmit={submit}><label>Local project path<input value={path} onChange={(event) => setPath(event.target.value)} placeholder="/path/to/rails-app" autoComplete="off" /></label><button className="button quiet" disabled={checking || !path.trim()}>{checking ? "Inspecting…" : "Open path"}</button></form></details>
        </article>
        <article className="settings-card">
          <Kicker>Native runtime</Kicker><h2>{project?.name || "No project selected"}</h2><p>{project?.path || "Choose an explicit local path"}</p>
          <div className="setting-row"><div><strong>Rubyn Code</strong><small>{engineDetail}</small></div><span className={engineState === "ready" ? "ready" : "not-ready"}>{engineState === "ready" && <Check size={14} />}{engineState}</span></div>
          <div className="setting-row"><div><strong>Chisel</strong><small>{chiselMode && chiselMode !== "off" ? `On · ${chiselMode} mode keeps Rubyn's changes focused` : "Ask Rubyn to write the minimum that works"}</small></div><button className={`switch ${chiselMode && chiselMode !== "off" ? "on" : ""}`} onClick={() => void toggleChisel()} role="switch" aria-label="Rubyn Chisel" aria-checked={Boolean(chiselMode && chiselMode !== "off")} disabled={!chiselMode || savingChisel || engineState !== "ready"} title="Applies to Rubyn Code runs; Codex uses its own behavior"><i /></button></div>
          {project && <div className="setting-row"><div><strong>Project kind</strong><small>{project.gitRoot || "Git root unavailable"}</small></div><span className="project-kind">{project.isRails ? "Rails" : project.isRuby ? "Ruby" : "Git"}</span></div>}
          <div className="setting-row"><div><strong>Reduced motion</strong><small>Disable decorative movement in this session</small></div><button className={`switch ${reducedMotion ? "on" : ""}`} onClick={() => setReducedMotion(!reducedMotion)} role="switch" aria-label="Reduced motion" aria-checked={reducedMotion}><i /></button></div>
          <div className="provider-summary"><strong>Models & accounts</strong><small>{modelCatalog?.connectedProviders.length || 0} connected · {modelCatalog?.models.length || 0} models available</small><button className="button quiet compact" onClick={() => useHarnessStore.getState().setView("accounts")}><KeyRound size={14} />Manage accounts</button></div>
        </article>
      </div>
    </section>
  );
}

function SidebarConversations() {
  const { project, projectData, activeConversationId, view, engineState, setProjectData, setNotice, openConversation } = useHarnessStore();
  const [renamingId, setRenamingId] = useState<number>();
  const [renameDraft, setRenameDraft] = useState("");
  const [savingId, setSavingId] = useState<number>();
  const conversations = [...(projectData?.runs || [])];
  const visible = conversations
    .filter((conversation) => !conversation.archivedAt)
    .sort((left, right) => Number(Boolean(right.pinned)) - Number(Boolean(left.pinned)) || right.updatedAt - left.updatedAt);
  const archived = conversations
    .filter((conversation) => Boolean(conversation.archivedAt))
    .sort((left, right) => (right.archivedAt || 0) - (left.archivedAt || 0));

  const save = async (request: { id: number; title?: string; pinned?: boolean; archived?: boolean }) => {
    if (!projectData) return;
    setSavingId(request.id);
    try {
      const updated = await harnessBridge.updateConversation(request);
      const current = useHarnessStore.getState().projectData;
      if (current) setProjectData({ ...current, runs: current.runs.map((conversation) => conversation.id === updated.id ? updated : conversation) });
      if (request.archived && activeConversationId === request.id) openConversation(0);
      setRenamingId(undefined);
      setNotice(request.archived === true ? "Conversation archived." : request.archived === false ? "Conversation restored." : request.title ? "Conversation renamed." : request.pinned ? "Conversation pinned." : "Conversation unpinned.");
    } catch (error) { setNotice(String(error)); } finally { setSavingId(undefined); }
  };

  return <div className="sidebar-threads">
    <div className="sidebar-label"><span>Conversations</span><button aria-label="New conversation" onClick={() => openConversation(0)} disabled={!project || engineState !== "ready"}><Plus size={13} /></button></div>
    {visible.slice(0, 20).map((conversation) => <div className={`sidebar-thread-row ${view === "agents" && activeConversationId === conversation.id ? "active" : ""}`} key={conversation.id}>
      <button className="thread-open" onClick={() => openConversation(conversation.id)}><span className={`pulse ${statusOf(conversation)}`} /><span><strong>{conversationTitle(conversation, 32)}</strong><small>{conversation.background ? "background run" : "conversation"} · {runLabel(conversation)}</small></span>{conversation.pinned && <Pin size={11} aria-label="Pinned" />}</button>
      <details className="thread-menu"><summary aria-label={`Actions for ${conversationTitle(conversation, 40)}`}><MoreHorizontal size={14} /></summary><div><button onClick={() => void save({ id: conversation.id, pinned: !conversation.pinned })}><Pin size={12} />{conversation.pinned ? "Unpin" : "Pin"}</button><button onClick={() => { setRenamingId(conversation.id); setRenameDraft(conversation.title || conversationTitle(conversation, 80)); }}><Pencil size={12} />Rename</button><button disabled={conversation.running} onClick={() => void save({ id: conversation.id, archived: true })}><Archive size={12} />Archive</button></div></details>
      {renamingId === conversation.id && <form className="thread-rename" onSubmit={(event) => { event.preventDefault(); if (renameDraft.trim()) void save({ id: conversation.id, title: renameDraft.trim() }); }}><input autoFocus aria-label="Conversation name" value={renameDraft} onChange={(event) => setRenameDraft(event.target.value)} maxLength={160} /><button disabled={!renameDraft.trim() || savingId === conversation.id}>Save</button><button type="button" onClick={() => setRenamingId(undefined)}>Cancel</button></form>}
    </div>)}
    {project && !visible.length && <small className="thread-empty">No conversations yet</small>}
    {archived.length > 0 && <details className="archived-threads"><summary>Archived ({archived.length})</summary>{archived.map((conversation) => <div key={conversation.id}><span>{conversationTitle(conversation, 34)}</span><button onClick={() => void save({ id: conversation.id, archived: false })} disabled={savingId === conversation.id}>Restore</button></div>)}</details>}
  </div>;
}

function CommandPalette({ onSwitchProject }: { onSwitchProject: (path: string) => Promise<void> }) {
  const { setView, setCommandOpen, projectData, appState, openConversation } = useHarnessStore();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const normalized = query.trim().toLowerCase();
  const options = [
    ...navigation.map((item) => ({ key: `view-${item.id}`, label: item.label, detail: "Open workspace", icon: item.icon, run: () => setView(item.id) })),
    ...(projectData?.runs || []).filter((conversation) => !conversation.archivedAt).map((conversation) => ({ key: `conversation-${conversation.id}`, label: conversationTitle(conversation, 60), detail: `Conversation · ${runLabel(conversation)}`, icon: MessageCircle, run: () => openConversation(conversation.id) })),
    ...(projectData?.tasks || []).map((task) => ({ key: `task-${task.id}`, label: task.title, detail: `Task · ${task.status}`, icon: Layers3, run: () => setView("workboard") })),
    ...(appState?.recentProjects || []).map((project) => ({ key: `project-${project.path}`, label: project.name, detail: "Open recent project", icon: FolderKanban, run: () => { void onSwitchProject(project.path); } })),
  ].filter((item) => !normalized || `${item.label} ${item.detail}`.toLowerCase().includes(normalized)).slice(0, 12);
  const open = (option: (typeof options)[number]) => { option.run(); setCommandOpen(false); };
  useEffect(() => { setActiveIndex(0); }, [query]);
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setCommandOpen(false); }}>
      <div className="command-palette" role="dialog" aria-modal="true" aria-label="Command palette">
        <div><Search size={17} /><input aria-label="Search commands" autoFocus value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === "ArrowDown") { event.preventDefault(); setActiveIndex((index) => Math.min(index + 1, options.length - 1)); } if (event.key === "ArrowUp") { event.preventDefault(); setActiveIndex((index) => Math.max(index - 1, 0)); } if (event.key === "Enter" && options[activeIndex]) open(options[activeIndex]); }} placeholder="Find a conversation, task, project, or view…" /><kbd>esc</kbd></div>
        {options.map((option, index) => <button className={activeIndex === index ? "selected" : ""} key={option.key} onMouseEnter={() => setActiveIndex(index)} onClick={() => open(option)}><option.icon size={16} /><span><strong>{option.label}</strong><small>{option.detail}</small></span>{activeIndex === index && <kbd>↵</kbd>}</button>)}
        {!options.length && <p className="command-empty">No matching conversations, tasks, projects, or views.</p>}
      </div>
    </div>
  );
}

function DesktopRequired() {
  return (
    <main className="desktop-required">
      <div className="desktop-card"><span className="brand-mark">R</span><Kicker>Rubyn Harness</Kicker><h1>Open the desktop app.</h1><p>This interface controls local Git worktrees and the bundled Rubyn Code process. The browser build deliberately contains no fixture data or simulated runtime.</p><div className="desktop-facts"><span><TerminalSquare size={15} />Local Rubyn process</span><span><GitBranch size={15} />Isolated worktrees</span><span><ShieldCheck size={15} />Native review actions</span></div></div>
    </main>
  );
}

export function App() {
  const store = useHarnessStore();
  const projectPath = store.project?.path;
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const [switchingProject, setSwitchingProject] = useState(false);

  useEffect(() => {
    if (!isDesktop()) return;
    let current = true;
    const boot = async () => {
      const state = useHarnessStore.getState();
      state.setLoading(true);
      try {
        const [engine, appState] = await Promise.all([harnessBridge.engineHealth(), harnessBridge.appState()]);
        if (!current) return;
        state.setEngine(engine.healthy ? "ready" : "unavailable", engine.version || engine.detail || "Rubyn Code unavailable");
        state.setAppState(appState);
        try { state.setModelCatalog(await harnessBridge.listModels()); } catch { /* Provider setup remains available after boot. */ }
        try { state.setSkills(await harnessBridge.listBundledSkills()); } catch { state.setSkills([]); }
        try { state.setGlobalRuns(await harnessBridge.listRuns()); } catch { state.setGlobalRuns([]); }
        const recent = appState.recentProjects[0];
        if (recent) {
          try {
            const project = await harnessBridge.inspectProject(recent.path);
            if (!current) return;
            state.setProject(project);
            state.setProjectData(await harnessBridge.projectData(project.path));
            state.setWayfinderMaps(await harnessBridge.listWayfinderMaps(project.path));
            state.setWayfinderBlockers(await harnessBridge.listWayfinderBlockers(project.path));
          } catch (error) { state.setNotice(`Recent project unavailable: ${String(error)}`); }
        }
      } catch (error) {
        if (current) {
          state.setEngine("unavailable", String(error));
          state.setNotice(String(error));
        }
      } finally { if (current) state.setLoading(false); }
    };
    void boot();
    return () => { current = false; };
  }, []);

  useEffect(() => {
    if (!isDesktop() || !projectPath) return;
    let busy = false;
    const poll = async () => {
      if (busy) return;
      busy = true;
      try {
        const state = useHarnessStore.getState();
        const [data, globalRuns] = await Promise.all([
          harnessBridge.projectData(projectPath),
          harnessBridge.listRuns(),
        ]);
        state.setProjectData(data);
        state.setGlobalRuns(globalRuns);
        try {
          const [maps, blockers] = await Promise.all([harnessBridge.listWayfinderMaps(projectPath), harnessBridge.listWayfinderBlockers(projectPath)]);
          state.setWayfinderMaps(maps); state.setWayfinderBlockers(blockers);
        } catch { /* Keep the active conversation responsive during a host migration. */ }
        for (const run of data.runs.filter((candidate) => candidate.running || candidate.id === state.selectedRunId)) {
          const batch = await harnessBridge.pollRunEvents(run.id, state.eventCursors[run.id]);
          state.appendRunEvents(run.id, batch.events, batch.nextEventId);
        }
      } catch {
        // A run can exit between project refresh and event polling; the next tick reconciles it.
      } finally { busy = false; }
    };
    void poll();
    const interval = window.setInterval(() => void poll(), 1300);
    return () => window.clearInterval(interval);
  }, [projectPath]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const state = useHarnessStore.getState();
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); state.setCommandOpen(true); }
      if (event.key === "Escape") { state.setCommandOpen(false); state.setMobileOpen(false); setProjectMenuOpen(false); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const ViewComponent = useMemo(() => ({
    control: ControlRoom,
    workboard: Workboard,
    agents: Runs,
    team: AgentTeam,
    wayfinder: Wayfinder,
    skills: Skills,
    review: Review,
    accounts: Accounts,
    projects: Projects,
  })[store.view], [store.view]);

  const switchProject = async (path: string) => {
    setSwitchingProject(true);
    try {
      const inspected = await harnessBridge.inspectProject(path);
      const data = await harnessBridge.projectData(inspected.path);
      const state = store.appState || await harnessBridge.appState();
      const nextState = { ...state, recentProjects: [{ path: inspected.path, name: inspected.name }, ...state.recentProjects.filter((recent) => recent.path !== inspected.path)].slice(0, 24) };
      store.setProject(inspected);
      store.setProjectData(data);
      store.setWayfinderMaps(await harnessBridge.listWayfinderMaps(inspected.path));
      store.setWayfinderBlockers(await harnessBridge.listWayfinderBlockers(inspected.path));
      store.setAppState(await harnessBridge.saveAppState(nextState));
      store.openConversation(data.runs[0]?.id || 0);
      setProjectMenuOpen(false);
      store.setNotice(`${inspected.name} is ready.`);
    } catch (error) { store.setNotice(String(error)); } finally { setSwitchingProject(false); }
  };
  const browseProject = async () => {
    try { const selected = await harnessBridge.chooseProjectFolder(); if (selected) await switchProject(selected); } catch (error) { store.setNotice(String(error)); }
  };

  if (!isDesktop()) return <DesktopRequired />;

  return (
    <div className={`app-shell ${store.reducedMotion ? "reduced-motion" : ""}`}>
      <aside className={`sidebar ${store.mobileOpen ? "open" : ""}`}>
        <div className="brand"><span className="brand-mark">R</span><span>rubyn</span><small>HARNESS</small><button className="sidebar-close" onClick={() => store.setMobileOpen(false)} aria-label="Close navigation"><PanelLeftClose size={18} /></button></div>
        <div className="project-switcher"><button className="project-switch" aria-expanded={projectMenuOpen} onClick={() => setProjectMenuOpen((open) => !open)}><span className="project-gem" /><div><small>PROJECT</small><strong>{store.project?.name || "Choose project"}</strong></div><ChevronRight size={15} /></button>{projectMenuOpen && <div className="project-popover"><button className="new-project" onClick={() => void browseProject()} disabled={switchingProject}><Plus size={14} />{switchingProject ? "Opening…" : "Open project folder"}</button>{store.appState?.recentProjects.map((recent) => <button className={recent.path === store.project?.path ? "active" : ""} key={recent.path} onClick={() => void switchProject(recent.path)} disabled={switchingProject}><span><FolderKanban size={14} />{recent.name}</span>{recent.path === store.project?.path && <Check size={13} />}</button>)}<button className="manage-projects" onClick={() => { store.setView("projects"); setProjectMenuOpen(false); }}>Project settings <ArrowRight size={13} /></button></div>}</div>
        <nav aria-label="Primary navigation">{primaryNavigation.map((item) => <button className={store.view === item.id ? "active" : ""} onClick={() => store.setView(item.id)} key={item.id}><item.icon size={17} /><span>{item.label}</span>{item.id === "wayfinder" && store.wayfinderBlockers.length > 0 && <b>{store.wayfinderBlockers.length}</b>}</button>)}</nav>
        {store.project && <div className="sidebar-wayfinder"><div className="sidebar-label"><span>Active maps</span><button aria-label="New Wayfinder map" onClick={() => { store.openWayfinderMap(undefined); store.setView("wayfinder"); }}><Plus size={13} /></button></div>{store.wayfinderMaps.filter((map) => map.status !== "archived").slice(0, 5).map((map) => <button key={map.id} className={store.view === "wayfinder" && store.activeWayfinderMapId === map.id ? "active" : ""} onClick={() => store.openWayfinderMap(map.id)}><span className={`pulse ${map.status === "draft" ? "waiting" : "running"}`} /><span><strong>{map.title}</strong><small>{map.status}</small></span></button>)}</div>}
        <SidebarConversations />
        <nav className="utility-nav" aria-label="Workspace utilities"><span className="sidebar-label">Workspace</span>{utilityNavigation.map((item) => <button className={store.view === item.id ? "active" : ""} onClick={() => store.setView(item.id)} key={item.id}><item.icon size={16} /><span>{item.label}</span></button>)}</nav>
        <div className="sidebar-foot"><button onClick={() => store.setView("projects")}><Settings2 size={16} />Project runtime</button><div className={`engine-status ${store.engineState}`} title={store.engineDetail}><span className="live-dot" />Rubyn · {store.engineState}</div></div>
      </aside>
      <main>
        <header className="topbar">
          <button className="menu-button" onClick={() => store.setMobileOpen(true)} aria-label="Open navigation"><Menu size={20} /></button>
          <div className="crumb"><span>{store.project?.name || "No project"}</span><ChevronRight size={14} /><strong>{labels[store.view]}</strong></div>
          <div className="topbar-actions"><button className="top-search" aria-label="Open command palette" onClick={() => store.setCommandOpen(true)}><Command size={15} /><span>Find anything</span><kbd>⌘ K</kbd></button></div>
        </header>
        <div className={`page ${store.view === "agents" ? "conversation-page" : ""}`}>{store.loading ? <div className="boot-state"><RefreshCw size={18} />Opening native workspace…</div> : <ViewComponent />}</div>
      </main>
      {store.notice && <div className="toast" role="status"><span>{store.notice}</span><button aria-label="Dismiss message" onClick={() => store.setNotice("")}><X size={15} /></button></div>}
      {store.commandOpen && <CommandPalette onSwitchProject={switchProject} />}
    </div>
  );
}
