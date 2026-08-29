import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  ArrowRight,
  Check,
  CircleHelp,
  FlaskConical,
  GitBranch,
  Map as MapIcon,
  Network,
  Play,
  Plus,
  Search,
  Sparkles,
  Trash2,
  UserRoundCheck,
  X,
} from "lucide-react";
import {
  harnessBridge,
  type CreateWayfinderTicketInput,
  type ModelOption,
  type WayfinderMapData,
  type WayfinderQuestion,
  type WayfinderTicket,
  type WayfinderTicketType,
  type WorkflowColumn,
} from "./bridge";
import { useHarnessStore } from "./store";
import {
  composeWayfinderChartPrompt,
  composeWayfinderLaunchPrompt,
  GRILLING_SKILL_PATH,
  WAYFINDER_SKILL_PATH,
} from "./wayfinderPrompts";

const ticketMeta: Record<WayfinderTicketType, { label: string; icon: typeof Sparkles; description: string }> = {
  grill: { label: "Grill", icon: CircleHelp, description: "Resolve a consequential human decision." },
  research: { label: "Research", icon: Search, description: "Reduce uncertainty with a bounded investigation." },
  prototype: { label: "Prototype", icon: FlaskConical, description: "Test an idea in a disposable worktree." },
  code: { label: "Code", icon: GitBranch, description: "Materialize an executable board task when unblocked." },
  user_action: { label: "User action", icon: UserRoundCheck, description: "Block on work only a person can complete." },
};

type AnswerDraft = Record<number, { answers: string[]; customAnswer: string }>;

function Dialog({ title, description, onClose, children }: { title: string; description: string; onClose: () => void; children: React.ReactNode }) {
  useEffect(() => {
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") onClose(); };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [onClose]);
  return <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section className="wayfinder-dialog" role="dialog" aria-modal="true" aria-labelledby="wayfinder-dialog-title">
      <header><div><span>WAYFINDER</span><h2 id="wayfinder-dialog-title">{title}</h2><p>{description}</p></div><button autoFocus aria-label="Close dialog" onClick={onClose}><X size={18} /></button></header>
      {children}
    </section>
  </div>;
}

function MapCreator({ onClose, onCreated }: { onClose: () => void; onCreated: (data: WayfinderMapData) => void }) {
  const project = useHarnessStore((state) => state.project)!;
  const models = useHarnessStore((state) => state.modelCatalog?.models) || [];
  const projectColumns = useHarnessStore((state) => state.projectData?.columns) || [];
  const setProjectData = useHarnessStore((state) => state.setProjectData);
  const setGlobalRuns = useHarnessStore((state) => state.setGlobalRuns);
  const setNotice = useHarnessStore((state) => state.setNotice);
  const solModels = models.filter((candidate) => `${candidate.tier} ${candidate.model}`.toLowerCase().includes("sol"));
  const [model, setModel] = useState<ModelOption | undefined>(solModels[0] || models[0]);
  const [idea, setIdea] = useState("");
  const [columns, setColumns] = useState<WorkflowColumn[]>(projectColumns);
  const [codeTaskStatus, setCodeTaskStatus] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let current = true;
    void harnessBridge.projectData(project.path).then((data) => {
      if (!current) return;
      setColumns(data.columns);
      setProjectData(data);
    }).catch((error) => setNotice(`Rubyn could not load the task columns: ${String(error)}`));
    return () => { current = false; };
  }, [project.path, setNotice, setProjectData]);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!idea.trim() || !model || !codeTaskStatus) return;
    setBusy(true);
    let data: WayfinderMapData | undefined;
    try {
      data = await harnessBridge.createWayfinderMap(project.path, idea.trim(), codeTaskStatus);
      const [wayfinder, grilling] = await Promise.all([
        harnessBridge.readSkill(WAYFINDER_SKILL_PATH),
        harnessBridge.readSkill(GRILLING_SKILL_PATH),
      ]);
      const prompt = composeWayfinderChartPrompt(data, wayfinder.content, grilling.content);
      const session = await harnessBridge.launchPrompt(project.path, prompt, [], model);
      const bootstrap = data.tickets.find((ticket) => ticket.title === "Name the destination") || data.tickets[0];
      if (bootstrap) await harnessBridge.linkWayfinderRun(bootstrap.id, session.id);
      try { setGlobalRuns(await harnessBridge.listRuns()); } catch { /* polling will reconcile */ }
      onCreated(await harnessBridge.getWayfinderMap(data.map.id));
    } catch (error) {
      if (data) onCreated(data);
      setNotice(`The map was saved, but Rubyn could not start the Grill: ${String(error)}`);
    } finally { setBusy(false); }
  };
  return <Dialog title="Start a Wayfinder map" description="Matt Pocock's Wayfinder and Grill Me instructions will turn the loose idea into app-native nodes and dependencies." onClose={onClose}>
    <form className="wayfinder-form" onSubmit={(event) => void submit(event)}>
      <label>What are you trying to figure out or deliver?<span>It can be incomplete. Rubyn will grill you before drawing the map.</span><textarea autoFocus required rows={6} value={idea} onChange={(event) => setIdea(event.target.value)} placeholder="We need to decide how…" /></label>
      <label>Where should code tasks go?<select aria-label="Code task column" required value={codeTaskStatus} onChange={(event) => setCodeTaskStatus(event.target.value)}><option value="">Choose a task column…</option>{columns.map((column) => <option key={column.id} value={column.key}>{column.name}</option>)}</select><span>When a code node becomes ready, Wayfinder will create its Task in this column.</span></label>
      <label>Planning model<select value={model ? `${model.provider}/${model.model}` : ""} onChange={(event) => setModel(models.find((option) => `${option.provider}/${option.model}` === event.target.value))}><option value="">No Rubyn-managed model available</option>{models.map((option) => <option key={`${option.provider}/${option.model}`} value={`${option.provider}/${option.model}`}>{option.provider} · {option.model}</option>)}</select><span>Every provider receives the same Harness planning controls.</span></label>
      <footer><button type="button" className="button quiet" onClick={onClose}>Cancel</button><button className="button primary" disabled={busy || !idea.trim() || !model || !codeTaskStatus}>{busy ? "Starting Rubyn…" : "Start Grill"}<ArrowRight size={15} /></button></footer>
    </form>
  </Dialog>;
}

function TicketComposer({ data, onClose, onSaved }: { data: WayfinderMapData; onClose: () => void; onSaved: () => void }) {
  const [type, setType] = useState<WayfinderTicketType>("research");
  const [title, setTitle] = useState("");
  const [question, setQuestion] = useState("");
  const [information, setInformation] = useState("");
  const [outcome, setOutcome] = useState("");
  const [dependsOn, setDependsOn] = useState<number[]>([]);
  const [modelRole, setModelRole] = useState("Terra");
  const [effort, setEffort] = useState("medium");
  const [budget, setBudget] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => setModelRole(type === "grill" ? "Sol" : type === "research" ? "Terra" : type === "prototype" ? "Terra" : "Sol"), [type]);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    const request: CreateWayfinderTicketInput = { mapId: data.map.id, title: title.trim(), question: question.trim(), information: information.trim(), outcome: outcome.trim(), ticketType: type, dependsOn, modelRole, effort, ...(budget ? { budgetCents: Number(budget) } : {}) };
    try { await harnessBridge.createWayfinderTicket(request); onSaved(); } finally { setBusy(false); }
  };
  return <Dialog title="Add a Wayfinder ticket" description="Define the uncertainty, context, and evidence that will settle it. Dependencies are validated before this becomes part of the map." onClose={onClose}>
    <form className="wayfinder-form" onSubmit={(event) => void submit(event)}>
      <fieldset className="ticket-type-grid"><legend>Ticket type</legend>{Object.entries(ticketMeta).map(([key, meta]) => <label key={key} className={type === key ? "selected" : ""}><input type="radio" name="ticket-type" value={key} checked={type === key} onChange={() => setType(key as WayfinderTicketType)} /><meta.icon size={17} /><strong>{meta.label}</strong><small>{meta.description}</small></label>)}</fieldset>
      <label>Title<input required autoFocus value={title} onChange={(event) => setTitle(event.target.value)} placeholder="A precise decision or outcome" /></label>
      <label>Question<span>What uncertainty does this ticket close?</span><textarea rows={2} value={question} onChange={(event) => setQuestion(event.target.value)} /></label>
      <label>Information<span>Context Rubyn must know before acting.</span><textarea rows={3} value={information} onChange={(event) => setInformation(event.target.value)} /></label>
      <label>Expected outcome<span>Observable evidence that makes the ticket resolved.</span><textarea rows={2} value={outcome} onChange={(event) => setOutcome(event.target.value)} /></label>
      <fieldset className="dependency-checks"><legend>Depends on</legend>{data.tickets.filter((ticket) => ticket.status !== "retired").map((ticket) => <label key={ticket.id}><input type="checkbox" checked={dependsOn.includes(ticket.id)} onChange={(event) => setDependsOn(event.target.checked ? [...dependsOn, ticket.id] : dependsOn.filter((id) => id !== ticket.id))} /><span>{ticket.title}</span></label>)}{!data.tickets.length && <small>No existing tickets.</small>}</fieldset>
      {type !== "user_action" && type !== "code" && <div className="launch-fields"><label>Model role<select value={modelRole} onChange={(event) => setModelRole(event.target.value)}><option>Sol</option><option>Terra</option></select></label><label>Effort<select value={effort} onChange={(event) => setEffort(event.target.value)}><option>low</option><option>medium</option><option>high</option></select></label><label>Budget (cents)<input inputMode="numeric" min="1" type="number" value={budget} onChange={(event) => setBudget(event.target.value)} placeholder="Optional" /></label></div>}
      <footer><button type="button" className="button quiet" onClick={onClose}>Cancel</button><button className="button primary" disabled={busy || !title.trim()}>{busy ? "Adding…" : "Add ticket"}</button></footer>
    </form>
  </Dialog>;
}

function QuestionCard({ question, value, onChange }: { question: WayfinderQuestion; value: { answers: string[]; customAnswer: string }; onChange: (value: { answers: string[]; customAnswer: string }) => void }) {
  const toggle = (id: string) => onChange({ ...value, answers: question.cardinality === "single" ? [id] : value.answers.includes(id) ? value.answers.filter((answer) => answer !== id) : [...value.answers, id] });
  return <fieldset className="grill-question"><legend><span>ROUND {question.round}</span>{question.prompt}</legend><div className="grill-options">{question.options.map((option) => <label key={option.id} className={value.answers.includes(option.id) ? "selected" : ""}><input type={question.cardinality === "single" ? "radio" : "checkbox"} name={`question-${question.id}`} checked={value.answers.includes(option.id)} onChange={() => toggle(option.id)} /><span><strong>{option.label}{option.recommended && <b>Recommended</b>}</strong><small>{option.description}</small><em><i>+</i>{option.pros}</em><em><i>−</i>{option.cons}</em></span></label>)}</div><label className="freeform-answer">Something else or added context<textarea rows={2} value={value.customAnswer} onChange={(event) => onChange({ ...value, customAnswer: event.target.value })} placeholder="Freeform always stays available…" /></label></fieldset>;
}

function LaunchPreview({ ticket, data, onClose, onLaunched }: { ticket: WayfinderTicket; data: WayfinderMapData; onClose: () => void; onLaunched: () => void }) {
  const { project, modelCatalog, globalRuns } = useHarnessStore();
  const models = modelCatalog?.models || [];
  const matching = models.filter((model) => `${model.tier} ${model.model}`.toLowerCase().includes(ticket.modelRole.toLowerCase()));
  const [model, setModel] = useState<ModelOption | undefined>(matching[0] || models[0]);
  const [busy, setBusy] = useState(false);
  const capacity = globalRuns.filter((run) => run.running).length;
  const launch = async () => {
    if (!project || !model) return;
    setBusy(true);
    try {
      const [wayfinder, grilling] = await Promise.all([
        harnessBridge.readSkill(WAYFINDER_SKILL_PATH),
        ticket.ticketType === "grill" || data.map.status === "draft" ? harnessBridge.readSkill(GRILLING_SKILL_PATH) : Promise.resolve(undefined),
      ]);
      const prompt = data.map.status === "draft" && ticket.title === "Name the destination" && grilling
        ? composeWayfinderChartPrompt(data, wayfinder.content, grilling.content)
        : composeWayfinderLaunchPrompt(ticket, data, wayfinder.content, grilling?.content);
      const session = await harnessBridge.launchPrompt(project.path, prompt, [], model);
      await harnessBridge.linkWayfinderRun(ticket.id, session.id);
      onLaunched();
    } finally { setBusy(false); }
  };
  return <Dialog title={`Launch ${ticketMeta[ticket.ticketType].label}`} description="Matt Pocock's Wayfinder instructions will guide this isolated run and can add app-native nodes through Rubyn." onClose={onClose}>
    <div className="launch-preview"><dl><div><dt>Ticket</dt><dd>{ticket.title}</dd></div><div><dt>Instructions</dt><dd>Wayfinder{ticket.ticketType === "grill" || data.map.status === "draft" ? " + Grilling" : ""}</dd></div><div><dt>App control</dt><dd>Create map nodes</dd></div><div><dt>Worktree</dt><dd>Disposable, isolated</dd></div><div><dt>Effort</dt><dd>{ticket.effort}</dd></div><div><dt>Budget</dt><dd>{ticket.budgetCents ? `$${(ticket.budgetCents / 100).toFixed(2)} ceiling` : "Unknown · no ticket ceiling"}</dd></div><div><dt>Concurrency</dt><dd>{capacity}/3 active</dd></div></dl><label>Resolved model<select value={model ? `${model.provider}/${model.model}` : ""} onChange={(event) => setModel(models.find((option) => `${option.provider}/${option.model}` === event.target.value))}><option value="">Unavailable</option>{models.map((option) => <option key={`${option.provider}/${option.model}`} value={`${option.provider}/${option.model}`}>{option.provider} · {option.model}</option>)}</select><span>Every provider receives the same Harness planning controls.</span></label>{!model && <p className="form-error">Configure a connected model for {ticket.modelRole} before launching.</p>}<footer><button className="button quiet" onClick={onClose}>Cancel</button><button className="button primary" disabled={!model || capacity >= 3 || busy} onClick={() => void launch()}><Play size={14} />{busy ? "Loading instructions…" : "Launch isolated run"}</button></footer></div>
  </Dialog>;
}

function DependencyGraph({ tickets, selected, onSelect }: { tickets: WayfinderTicket[]; selected?: number; onSelect: (id: number) => void }) {
  const visible = tickets.filter((ticket) => ticket.status !== "retired");
  const positions = new Map<number, { x: number; y: number }>();
  const depth = (ticket: WayfinderTicket, seen = new Set<number>()): number => ticket.dependsOn.length && !seen.has(ticket.id) ? 1 + Math.max(...ticket.dependsOn.map((id) => { const parent = visible.find((item) => item.id === id); return parent ? depth(parent, new Set([...seen, ticket.id])) : 0; })) : 0;
  const layers = new Map<number, WayfinderTicket[]>();
  visible.forEach((ticket) => { const level = depth(ticket); layers.set(level, [...(layers.get(level) || []), ticket]); });
  [...layers.entries()].forEach(([level, layer]) => layer.forEach((ticket, index) => positions.set(ticket.id, { x: 34 + level * 260, y: 50 + index * 104 })));
  const width = Math.max(620, (Math.max(0, ...layers.keys()) + 1) * 260 + 40);
  const height = Math.max(210, ...[...positions.values()].map((position) => position.y + 90));
  return <div className="wayfinder-graph" tabIndex={0} aria-label="Ticket dependency graph"><div style={{ width, height }}><svg width={width} height={height} aria-hidden="true">{visible.flatMap((ticket) => ticket.dependsOn.map((dependency) => { const from = positions.get(dependency); const to = positions.get(ticket.id); return from && to ? <path key={`${dependency}-${ticket.id}`} d={`M ${from.x + 190} ${from.y + 31} C ${from.x + 224} ${from.y + 31}, ${to.x - 34} ${to.y + 31}, ${to.x} ${to.y + 31}`} /> : null; }))}</svg>{visible.map((ticket) => { const position = positions.get(ticket.id)!; return <button key={ticket.id} style={{ left: position.x, top: position.y }} className={`wayfinder-node ${ticket.status} ${selected === ticket.id ? "selected" : ""}`} onClick={() => onSelect(ticket.id)}><span>{ticketMeta[ticket.ticketType].label}</span><strong>{ticket.title}</strong><small>{ticket.status}</small></button>; })}</div></div>;
}

function WayfinderMapWorkspace({ data, reload }: { data: WayfinderMapData; reload: () => Promise<void> }) {
  const store = useHarnessStore();
  const [selectedId, setSelectedId] = useState<number | undefined>();
  const [composer, setComposer] = useState(false);
  const [launching, setLaunching] = useState<WayfinderTicket>();
  const [answers, setAnswers] = useState<AnswerDraft>(() => Object.fromEntries(data.questions.map((question) => [question.id, { answers: question.answers, customAnswer: question.customAnswer }])));
  const [destination, setDestination] = useState(data.map.destination);
  const [resultNote, setResultNote] = useState("");
  const [resolution, setResolution] = useState("");
  const selected = data.tickets.find((ticket) => ticket.id === selectedId);
  const bootstrap = data.tickets.find((ticket) => ticket.title === "Name the destination");
  const bootstrapRun = store.globalRuns.find((run) => run.id === bootstrap?.linkedRunId);
  const frontier = data.tickets.filter((ticket) => ticket.status === "frontier");
  const blockers = data.tickets.filter((ticket) => ticket.ticketType === "user_action" && !["resolved", "retired"].includes(ticket.status));
  const resolved = data.tickets.filter((ticket) => ["resolved", "retired"].includes(ticket.status)).length;
  const openQuestions = data.questions.filter((question) => !question.answeredAt).slice(0, 3);
  const submitAnswers = async () => {
    if (!openQuestions.length) return;
    const ticketId = openQuestions[0].ticketId;
    const next = await harnessBridge.submitWayfinderAnswers(ticketId, openQuestions.map((question) => ({ questionId: question.id, ...(answers[question.id] || { answers: [], customAnswer: "" }) })));
    store.setWayfinderData(next);
  };
  const saveDestination = async () => { store.setWayfinderData(await harnessBridge.updateWayfinderMap(data.map.id, { destination })); };
  const activate = async () => { await saveDestination(); store.setWayfinderData(await harnessBridge.activateWayfinderMap(data.map.id)); await reload(); };
  const completeAction = async (ticket: WayfinderTicket) => { store.setWayfinderData(await harnessBridge.completeWayfinderUserAction(ticket.id, resultNote)); setResultNote(""); await reload(); };
  const resolve = async (ticket: WayfinderTicket) => { store.setWayfinderData(await harnessBridge.resolveWayfinderTicket(ticket.id, resolution)); setResolution(""); await reload(); };
  const canArchive = data.tickets.every((ticket) => ["resolved", "retired"].includes(ticket.status));
  const canActivateGeneratedMap = Boolean(data.map.destination.trim() && data.tickets.some((ticket) => ticket.id !== bootstrap?.id));
  useEffect(() => {
    if (!bootstrap?.linkedRunId) return;
    void reload();
    if (!bootstrapRun?.running) return;
    const timer = window.setInterval(() => void reload(), 2500);
    return () => window.clearInterval(timer);
  }, [bootstrap?.linkedRunId, bootstrapRun?.running, bootstrapRun?.updatedAt, reload]);
  return <section className="wayfinder-workspace">
    <header className="wayfinder-destination"><div><span>DESTINATION</span><input aria-label="Map title" value={data.map.title} onChange={() => {}} readOnly /><textarea aria-label="Destination" value={destination} onChange={(event) => setDestination(event.target.value)} placeholder="The observable future state this map must reach…" /></div><div className="map-progress"><strong>{data.tickets.length ? Math.round(resolved / data.tickets.length * 100) : 0}%</strong><span>{resolved} of {data.tickets.length} settled</span><i><b style={{ width: `${data.tickets.length ? resolved / data.tickets.length * 100 : 0}%` }} /></i><small>{data.map.status}</small></div><div className="destination-actions"><button className="button quiet" onClick={() => void saveDestination()}>Save destination</button><button className="button quiet" disabled={!canArchive || data.map.status === "archived"} onClick={async () => { if (window.confirm("Archive this completed map? Its history remains immutable.")) { store.setWayfinderData(await harnessBridge.archiveWayfinderMap(data.map.id)); await reload(); } }}><Archive size={14} />Archive</button></div></header>
    {data.map.status === "draft" && bootstrap?.linkedRunId ? <section className="bootstrap-grill"><header><div><span>WAYFINDER + GRILL ME</span><h2>{bootstrapRun?.running ? "Rubyn is mapping this with you." : "Review the map Rubyn produced."}</h2><p>The conversation settles human decisions. Rubyn then writes the destination, nodes, and dependencies directly into this graph.</p></div><Sparkles size={22} /></header><div className="activation-review"><Check size={18} /><div><strong>{canActivateGeneratedMap ? "First frontier is ready for review" : "Continue the Grill to reveal the first frontier"}</strong><p>Code nodes become Tasks only after you activate the map and their dependencies are settled.</p></div><button className="button quiet" onClick={() => store.openConversation(bootstrap.linkedRunId!)}>Open Grill conversation</button><button className="button quiet" onClick={() => void reload()}>Refresh map</button><button className="button primary" disabled={!canActivateGeneratedMap || Boolean(bootstrapRun?.running)} onClick={() => void activate()}>Activate map</button></div></section> : data.map.status === "draft" && <section className="bootstrap-grill"><header><div><span>GRILL ME · BOOTSTRAP</span><h2>Turn the idea into a navigable map.</h2><p>{openQuestions.length ? "Answer the saved bootstrap questions, or launch the frontier Grill below." : "Launch the frontier Grill below so Rubyn can create the destination and map nodes."}</p></div><Sparkles size={22} /></header>{openQuestions.map((question) => <QuestionCard key={question.id} question={question} value={answers[question.id] || { answers: [], customAnswer: "" }} onChange={(value) => setAnswers((current) => ({ ...current, [question.id]: value }))} />)}{openQuestions.length > 0 && <button className="button primary" disabled={openQuestions.some((question) => { const value = answers[question.id]; return !value?.answers.length && !value?.customAnswer.trim(); })} onClick={() => void submitAnswers()}>Submit round <ArrowRight size={15} /></button>}</section>}
    <section><div className="wayfinder-section-title"><div><span>DEPENDENCY GRAPH</span><h2>The work and the fog</h2></div><button className="button primary" onClick={() => setComposer(true)}><Plus size={14} />Add ticket</button></div><DependencyGraph tickets={data.tickets} selected={selectedId} onSelect={setSelectedId} /></section>
    <div className="wayfinder-columns"><section><div className="wayfinder-section-title"><div><span>NEXT FRONTIER</span><h2>{frontier[0]?.title || "No unlocked ticket"}</h2><p>{frontier[0] ? `${ticketMeta[frontier[0].ticketType].label} is ready because every dependency is settled.` : "Resolve or retire blockers to reveal the next useful move."}</p></div>{frontier[0] && !["code", "user_action"].includes(frontier[0].ticketType) && <button className="button primary" onClick={() => setLaunching(frontier[0])}><Play size={14} />Preview launch</button>}</div><div className="ticket-list">{data.tickets.map((ticket) => <button key={ticket.id} className={selectedId === ticket.id ? "selected" : ""} onClick={() => setSelectedId(ticket.id)}><span className={`ticket-state ${ticket.status}`} /><div><strong>{ticket.title}</strong><small>{ticketMeta[ticket.ticketType].label} · {ticket.status} · brief v{ticket.briefVersion}</small></div>{ticket.linkedTaskId && <b>Task #{ticket.linkedTaskId}</b>}{ticket.linkedRunId && <b>Run #{ticket.linkedRunId}</b>}<ArrowRight size={14} /></button>)}</div></section>
      <aside className="ticket-inspector">{selected ? <><header><span>{ticketMeta[selected.ticketType].label.toUpperCase()} · {selected.status.toUpperCase()}</span><h2>{selected.title}</h2><p>{selected.question || selected.outcome || "No brief detail yet."}</p></header><dl><div><dt>Information</dt><dd>{selected.information || "—"}</dd></div><div><dt>Outcome</dt><dd>{selected.outcome || "—"}</dd></div><div><dt>Dependencies</dt><dd>{selected.dependsOn.length ? selected.dependsOn.map((id) => data.tickets.find((ticket) => ticket.id === id)?.title || `#${id}`).join(", ") : "None"}</dd></div><div><dt>Route</dt><dd>{selected.modelRole} · {selected.effort}{selected.budgetCents ? ` · $${(selected.budgetCents / 100).toFixed(2)}` : ""}</dd></div></dl>{selected.ticketType === "user_action" && selected.status !== "resolved" && <label className="inspector-action">Result note<textarea value={resultNote} onChange={(event) => setResultNote(event.target.value)} rows={3} /><button className="button primary" disabled={!resultNote.trim()} onClick={() => void completeAction(selected)}>Complete action</button></label>}{selected.status === "frontier" && !["code", "user_action"].includes(selected.ticketType) && <button className="button primary wide" onClick={() => setLaunching(selected)}><Play size={14} />Preview launch</button>}{selected.status === "frontier" && selected.ticketType === "grill" && <label className="inspector-action">Approved resolution<textarea value={resolution} onChange={(event) => setResolution(event.target.value)} rows={4} /><button className="button primary" disabled={!resolution.trim()} onClick={() => void resolve(selected)}>Approve resolution</button></label>}{!["resolved", "retired"].includes(selected.status) && <button className="button danger wide" onClick={async () => { if (window.confirm("Retire this ticket? Linked task and run history will be preserved.")) { store.setWayfinderData(await harnessBridge.retireWayfinderTicket(selected.id)); await reload(); } }}><Trash2 size={14} />Retire ticket</button>}</> : <div className="inspector-empty"><Network size={24} /><strong>Select a ticket</strong><p>Its brief, dependencies, route, blocker controls, and evidence appear here.</p></div>}</aside>
    </div>
    <section className="blocker-area"><div className="wayfinder-section-title"><div><span>USER BLOCKERS</span><h2>{blockers.length ? `${blockers.length} need your action` : "No human blockers"}</h2></div></div>{blockers.map((ticket) => <button key={ticket.id} onClick={() => setSelectedId(ticket.id)}><UserRoundCheck size={16} /><span><strong>{ticket.title}</strong><small>{ticket.outcome || "A result note is required to unlock dependents."}</small></span><ArrowRight size={14} /></button>)}</section>
    <section className="wayfinder-history"><div className="wayfinder-section-title"><div><span>IMMUTABLE HISTORY</span><h2>What changed and why</h2></div></div>{[...data.events].reverse().map((event) => <article key={event.id}><span>{new Date(event.createdAt * 1000).toLocaleString()}</span><strong>{event.kind.replaceAll("_", " ")}</strong><small>{event.actor}{event.ticketId ? ` · ticket #${event.ticketId}` : ""}</small></article>)}</section>
    {composer && <TicketComposer data={data} onClose={() => setComposer(false)} onSaved={async () => { setComposer(false); await reload(); }} />}
    {launching && <LaunchPreview ticket={launching} data={data} onClose={() => setLaunching(undefined)} onLaunched={async () => { setLaunching(undefined); await reload(); store.setNotice(`Rubyn is working on ${launching.title}.`); }} />}
  </section>;
}

export function Wayfinder() {
  const store = useHarnessStore();
  const [creator, setCreator] = useState(false);
  const [loading, setLoading] = useState(false);
  const { setWayfinderMaps, setWayfinderBlockers, setWayfinderData, openWayfinderMap } = store;
  const projectPath = store.project?.path;
  const loadMaps = useCallback(async () => {
    if (!projectPath) return;
    const [maps, blockers] = await Promise.all([harnessBridge.listWayfinderMaps(projectPath), harnessBridge.listWayfinderBlockers(projectPath)]);
    setWayfinderMaps(maps); setWayfinderBlockers(blockers);
  }, [projectPath, setWayfinderBlockers, setWayfinderMaps]);
  const loadMap = useCallback(async (id: number) => { setLoading(true); try { setWayfinderData(await harnessBridge.getWayfinderMap(id)); openWayfinderMap(id); } finally { setLoading(false); } }, [openWayfinderMap, setWayfinderData]);
  useEffect(() => { void loadMaps(); }, [loadMaps]);
  useEffect(() => { if (store.activeWayfinderMapId) void loadMap(store.activeWayfinderMapId); }, [loadMap, store.activeWayfinderMapId]);
  const reloadActiveMap = useCallback(async () => {
    if (!store.activeWayfinderMapId) return;
    setWayfinderData(await harnessBridge.getWayfinderMap(store.activeWayfinderMapId));
    await loadMaps();
  }, [loadMaps, setWayfinderData, store.activeWayfinderMapId]);
  const active = useMemo(() => store.wayfinderMaps.filter((map) => map.status !== "archived"), [store.wayfinderMaps]);
  if (!store.project) return <div className="empty-state-card"><span className="empty-gem" /><h2>Choose a project first</h2><p>Every Wayfinder map belongs to exactly one Ruby or Rails project.</p><button className="button primary" onClick={() => store.setView("projects")}>Choose project</button></div>;
  if (loading) return <div className="boot-state">Opening Wayfinder map…</div>;
  if (store.activeWayfinderMapId && store.wayfinderData?.map.id === store.activeWayfinderMapId) return <WayfinderMapWorkspace data={store.wayfinderData} reload={reloadActiveMap} />;
  return <section className="wayfinder-index"><header><div><span>WAYFINDER</span><h1>Map the destination,<br /><em>then move the frontier.</em></h1><p>Turn uncertainty into a dependency graph of decisions, research, prototypes, code, and human actions. Grill Me is the decision engine inside every map.</p></div><button className="button primary" onClick={() => setCreator(true)}><Plus size={15} />New map</button></header>{active.length ? <div className="map-grid">{active.map((map) => { const attention = store.wayfinderBlockers.filter((ticket) => ticket.mapId === map.id).length; return <button key={map.id} onClick={() => void loadMap(map.id)}><MapIcon size={20} /><span><small>{map.status}{attention ? ` · ${attention} need you` : ""}</small><strong>{map.title}</strong><p>{map.destination || map.idea}</p></span><ArrowRight size={16} /></button>; })}</div> : <div className="empty-state-card"><span className="empty-gem" /><h2>No maps yet</h2><p>Start with a loose idea. Grill Me will help establish the destination, boundaries, fog, and first frontier.</p><button className="button primary" onClick={() => setCreator(true)}>Start your first map</button></div>}{store.wayfinderMaps.some((map) => map.status === "archived") && <details className="archived-maps"><summary>Archived maps ({store.wayfinderMaps.filter((map) => map.status === "archived").length})</summary>{store.wayfinderMaps.filter((map) => map.status === "archived").map((map) => <button key={map.id} onClick={() => void loadMap(map.id)}>{map.title}<ArrowRight size={13} /></button>)}</details>}{creator && <MapCreator onClose={() => setCreator(false)} onCreated={(data) => { setCreator(false); store.setWayfinderData(data); store.openWayfinderMap(data.map.id); void loadMaps(); }} />}</section>;
}
