use crate::domain::{
    AttachmentInput, EngineLaunchMode, EngineSession, EngineSessionOutput, LaunchEngineRequest,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::{
    collections::{HashMap, VecDeque},
    env,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub(crate) fn codex_executable() -> std::io::Result<PathBuf> {
    if let Some(configured) = env::var_os("RUBYN_CODEX_EXECUTABLE") {
        let configured = PathBuf::from(configured);
        if is_executable_file(&configured) {
            return Ok(configured);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "RUBYN_CODEX_EXECUTABLE does not point to an executable file: {}",
                configured.display()
            ),
        ));
    }

    let mut candidates: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| {
            env::split_paths(&path)
                .map(|directory| directory.join("codex"))
                .collect()
        })
        .unwrap_or_default();
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        candidates.extend([
            home.join(".local/bin/codex"),
            home.join(".npm-global/bin/codex"),
            home.join(".bun/bin/codex"),
            home.join(".volta/bin/codex"),
            home.join("Library/pnpm/codex"),
        ]);
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ]);

    candidates
        .into_iter()
        .find(|candidate| is_executable_file(candidate))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Codex CLI was not found. Install Codex, reopen Rubyn Harness, or set RUBYN_CODEX_EXECUTABLE to the Codex executable.",
            )
        })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[derive(Debug, Clone)]
pub enum EngineLocation {
    Bundled { root: PathBuf, ruby: PathBuf },
    Installed,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("A prompt cannot be empty")]
    EmptyPrompt,
    #[error("Unable to launch Rubyn Code: {0}")]
    Launch(#[from] std::io::Error),
    #[error("No managed Rubyn Code session exists with id {0}")]
    UnknownSession(u64),
    #[error("Parallel run limit reached; wait for one of the {0} active runs to finish")]
    AtCapacity(usize),
    #[error("Run {0} is no longer an active conversation")]
    ConversationClosed(u64),
    #[error("Run {0} must restart to switch model backends")]
    BackendChanged(u64),
    #[error("Attachment error: {0}")]
    Attachment(String),
}

#[derive(Default)]
pub struct EngineSupervisor {
    children: HashMap<u64, ManagedEngine>,
}

#[derive(Debug, Clone)]
pub struct EngineEvent {
    pub run_id: u64,
    pub sequence: u64,
    pub kind: String,
    pub payload: serde_json::Value,
    pub raw: String,
    pub created_at: u64,
}

struct ManagedEngine {
    child: Child,
    project_path: String,
    source_project_path: String,
    worktree_path: String,
    mode: String,
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
    stdin: Arc<Mutex<ChildStdin>>,
    running: bool,
    outcome: String,
    protocol: Arc<Mutex<ProtocolState>>,
    events: Arc<Mutex<Vec<EngineEvent>>>,
    acknowledged_event_sequence: u64,
    started_at: std::time::Instant,
    next_request_id: u64,
    queued_messages: VecDeque<PendingTurn>,
    provider: Option<String>,
    model: Option<String>,
    backend: EngineBackend,
    backend_thread_id: Arc<Mutex<Option<String>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineBackend {
    Rubyn,
    Codex,
}

struct PendingTurn {
    message: String,
    attachments: Vec<serde_json::Value>,
    provider: Option<String>,
    model: Option<String>,
}

#[derive(Default)]
struct ProtocolState {
    done: bool,
    final_text: bool,
    outcome: Option<String>,
    busy: bool,
}

impl ProtocolState {
    fn mark_status(&mut self, status: &str) -> bool {
        match status {
            "done" => {
                self.done = true;
                if self.final_text {
                    self.outcome = Some("waiting".into());
                    self.busy = false;
                    true
                } else {
                    false
                }
            }
            "error" => {
                self.outcome = Some("failed".into());
                self.busy = false;
                true
            }
            "cancelled" => {
                self.outcome = Some("cancelled".into());
                self.busy = false;
                true
            }
            _ => false,
        }
    }

    fn mark_final_text(&mut self) -> bool {
        self.final_text = true;
        if self.done {
            self.outcome = Some("waiting".into());
            self.busy = false;
            true
        } else {
            false
        }
    }

    fn begin_turn(&mut self) {
        self.done = false;
        self.final_text = false;
        self.outcome = None;
        self.busy = true;
    }
}

const MAX_CAPTURE_BYTES: usize = 1_048_576;
const CODEX_APPROVAL_POLICY: &str = "untrusted";

impl EngineSupervisor {
    pub fn at_capacity(&mut self, limit: usize) -> bool {
        self.refresh_statuses();
        self.children
            .values()
            .filter(|engine| engine.running)
            .count()
            >= limit.max(1)
    }

    pub fn launch(
        &mut self,
        id: u64,
        location: EngineLocation,
        request: LaunchEngineRequest,
        project: &Path,
        parallel_limit: usize,
    ) -> Result<EngineSession, EngineError> {
        if self.at_capacity(parallel_limit) {
            return Err(EngineError::AtCapacity(parallel_limit.max(1)));
        }
        if self.children.get(&id).is_some_and(|engine| engine.running) {
            return Err(EngineError::Launch(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("conversation {id} is already active"),
            )));
        }
        self.children.remove(&id);
        let mode = mode_label(&request.mode);
        let prepared_attachments = prepare_attachments(&request.attachments)?;
        let source_project_path = request.project_path.clone();
        let backend = backend_for_provider(request.provider.as_deref());
        let mut command = match backend {
            EngineBackend::Rubyn => engine_command(location, project, &request)?,
            EngineBackend::Codex => codex_command(project, &request)?,
        };
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            // The frontend never supplies a program name, arguments, or shell string.
            // Keeping stdin open is required for Rubyn's --ide JSON-RPC lifecycle. The
            // output pipes are drained so a verbose child can never block on a full pipe.
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = Arc::new(Mutex::new(child.stdin.take().ok_or_else(|| {
            EngineError::Launch(std::io::Error::other("engine stdin unavailable"))
        })?));
        let events = Arc::new(Mutex::new(Vec::new()));
        let event_sequence = Arc::new(AtomicU64::new(1));
        let backend_thread_id = Arc::new(Mutex::new(None));
        let (stdout, protocol) = match backend {
            EngineBackend::Rubyn => capture_protocol(
                id,
                child.stdout.take(),
                Arc::clone(&stdin),
                Arc::clone(&events),
                Arc::clone(&event_sequence),
            ),
            EngineBackend::Codex => capture_codex_protocol(
                child.stdout.take(),
                Arc::clone(&stdin),
                CodexCaptureConfig {
                    run_id: id,
                    events: Arc::clone(&events),
                    event_sequence: Arc::clone(&event_sequence),
                    thread_id: Arc::clone(&backend_thread_id),
                    workspace: project.to_path_buf(),
                    model: request.model.clone(),
                    resume_thread_id: request.backend_thread_id.clone(),
                    initial_prompt: match &request.mode {
                        EngineLaunchMode::Prompt { prompt } => prompt.clone(),
                        EngineLaunchMode::Ide => String::new(),
                    },
                },
            ),
        };
        if let Ok(mut state) = protocol.lock() {
            state.begin_turn();
        }
        let stderr = capture(id, child.stderr.take(), Arc::clone(&events), event_sequence);
        if backend == EngineBackend::Rubyn {
            if let EngineLaunchMode::Prompt { prompt } = &request.mode {
                let mut requests = send_request(
                    &stdin,
                    &serde_json::json!({
                        "jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"workspacePath": project, "extensionVersion": "rubyn-harness/0.1", "capabilities": {"streaming": true, "inlineDiff": true}}
                    }),
                );
                let prompt_request_id = if request.resume_session {
                    requests = requests.and_then(|_| {
                        send_request(
                            &stdin,
                            &serde_json::json!({
                                "jsonrpc": "2.0", "id": 2, "method": "session/resume",
                                "params": {"sessionId": format!("harness-{id}")}
                            }),
                        )
                    });
                    3
                } else {
                    2
                };
                requests = requests.and_then(|_| send_request(
                &stdin,
                &serde_json::json!({
                    "jsonrpc": "2.0", "id": prompt_request_id, "method": "prompt",
                    "params": {"sessionId": format!("harness-{id}"), "text": prompt, "attachments": prepared_attachments, "context": {"workspacePath": project, "provider": request.provider.clone(), "model": request.model.clone()}}
                }),
            ));
                if let Err(error) = requests {
                    let _ = terminate_process_group(&mut child, true);
                    let _ = child.wait();
                    return Err(error);
                }
            }
        }
        let session = EngineSession {
            id,
            project_path: project.to_string_lossy().into_owned(),
            source_project_path: source_project_path.clone(),
            worktree_path: project.to_string_lossy().into_owned(),
            mode,
            pid: Some(child.id()),
            running: true,
            outcome: "running".into(),
        };
        self.children.insert(
            id,
            ManagedEngine {
                child,
                project_path: session.project_path.clone(),
                source_project_path,
                worktree_path: session.worktree_path.clone(),
                mode: session.mode.clone(),
                stdout,
                stderr,
                stdin,
                running: true,
                outcome: "running".into(),
                protocol,
                events,
                acknowledged_event_sequence: 0,
                started_at: std::time::Instant::now(),
                next_request_id: if request.resume_session { 4 } else { 3 },
                queued_messages: VecDeque::new(),
                provider: request.provider,
                model: request.model,
                backend,
                backend_thread_id,
            },
        );
        Ok(session)
    }

    pub fn send_message(
        &mut self,
        id: u64,
        message: &str,
        attachments: &[AttachmentInput],
        provider: Option<&str>,
        model: Option<&str>,
    ) -> Result<(), EngineError> {
        let message = message.trim();
        if message.is_empty() {
            return Err(EngineError::EmptyPrompt);
        }
        let attachments = prepare_attachments(attachments)?;
        let managed = self
            .children
            .get_mut(&id)
            .ok_or(EngineError::UnknownSession(id))?;
        if managed.child.try_wait()?.is_some() || !managed.running {
            return Err(EngineError::ConversationClosed(id));
        }
        let next_provider = provider
            .map(str::to_owned)
            .or_else(|| managed.provider.clone());
        let next_model = model.map(str::to_owned).or_else(|| managed.model.clone());
        if backend_for_provider(next_provider.as_deref()) != managed.backend {
            return Err(EngineError::BackendChanged(id));
        }
        let protocol = managed
            .protocol
            .lock()
            .map_err(|_| EngineError::ConversationClosed(id))?;
        if protocol.busy {
            managed.queued_messages.push_back(PendingTurn {
                message: message.to_owned(),
                attachments,
                provider: next_provider,
                model: next_model,
            });
            return Ok(());
        }
        drop(protocol);
        managed.provider = next_provider;
        managed.model = next_model;
        send_turn(managed, id, message, attachments)
    }

    pub fn sessions(&mut self) -> Vec<EngineSession> {
        self.refresh_statuses();
        let mut sessions: Vec<_> = self
            .children
            .iter()
            .map(|(id, managed)| EngineSession {
                id: *id,
                project_path: managed.project_path.clone(),
                source_project_path: managed.source_project_path.clone(),
                worktree_path: managed.worktree_path.clone(),
                mode: managed.mode.clone(),
                pid: Some(managed.child.id()),
                running: managed.running,
                outcome: managed.outcome.clone(),
            })
            .collect();
        sessions.sort_by_key(|session| session.id);
        sessions
    }

    pub fn stop(&mut self, id: u64) -> Result<(), EngineError> {
        let managed = self
            .children
            .get_mut(&id)
            .ok_or(EngineError::UnknownSession(id))?;
        let completed_turn = managed.outcome == "waiting"
            || managed
                .protocol
                .lock()
                .ok()
                .and_then(|state| state.outcome.clone())
                .as_deref()
                == Some("waiting");
        if managed.child.try_wait()?.is_none() {
            let _ = send_request(
                &managed.stdin,
                &serde_json::json!({"jsonrpc":"2.0","id":9999,"method":"shutdown","params":{}}),
            );
            thread::sleep(std::time::Duration::from_millis(250));
        }
        if managed.child.try_wait()?.is_none() {
            terminate_process_group(&mut managed.child, false)?;
            thread::sleep(std::time::Duration::from_millis(250));
        }
        if managed.child.try_wait()?.is_none() {
            terminate_process_group(&mut managed.child, true)?;
            let _ = managed.child.wait();
        }
        managed.running = false;
        managed.outcome = if completed_turn {
            "completed".into()
        } else {
            "cancelled".into()
        };
        Ok(())
    }

    /// Answers a server-initiated `ide/askUser` JSON-RPC request. Keeping the
    /// request id opaque lets Rubyn own the conversation while the Harness
    /// renders and durably records the decision UI.
    pub fn answer_question(
        &mut self,
        id: u64,
        request_id: serde_json::Value,
        answer: serde_json::Value,
    ) -> Result<(), EngineError> {
        let managed = self
            .children
            .get_mut(&id)
            .ok_or(EngineError::UnknownSession(id))?;
        if managed.child.try_wait()?.is_some() || !managed.running {
            return Err(EngineError::ConversationClosed(id));
        }
        if !request_id.is_number() && !request_id.is_string() {
            return Err(EngineError::Launch(std::io::Error::other(
                "invalid Ask User request id",
            )));
        }
        send_request(
            &managed.stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": { "answer": answer }
            }),
        )
    }

    pub fn resolve_edit(
        &mut self,
        id: u64,
        edit_id: &str,
        accepted: bool,
    ) -> Result<(), EngineError> {
        let managed = self
            .children
            .get_mut(&id)
            .ok_or(EngineError::UnknownSession(id))?;
        if !managed.running {
            return Err(EngineError::ConversationClosed(id));
        }
        match managed.backend {
            EngineBackend::Rubyn => {
                let request_id = managed.next_request_id;
                managed.next_request_id = managed.next_request_id.saturating_add(1);
                send_request(
                    &managed.stdin,
                    &serde_json::json!({
                        "jsonrpc": "2.0", "id": request_id, "method": "acceptEdit",
                        "params": {"editId": edit_id, "accepted": accepted}
                    }),
                )
            }
            EngineBackend::Codex => {
                let encoded: serde_json::Value = serde_json::from_str(edit_id).map_err(|_| {
                    EngineError::Launch(std::io::Error::other("invalid Codex edit request id"))
                })?;
                let request_id = encoded.get("requestId").cloned().unwrap_or(encoded);
                send_request(
                    &managed.stdin,
                    &serde_json::json!({
                        "id": request_id,
                        "result": {"decision": if accepted { "accept" } else { "decline" }}
                    }),
                )
            }
        }
    }

    pub fn output(&mut self, id: u64) -> Result<EngineSessionOutput, EngineError> {
        self.refresh_statuses();
        let managed = self
            .children
            .get(&id)
            .ok_or(EngineError::UnknownSession(id))?;
        Ok(EngineSessionOutput {
            session: EngineSession {
                id,
                project_path: managed.project_path.clone(),
                source_project_path: managed.source_project_path.clone(),
                worktree_path: managed.worktree_path.clone(),
                mode: managed.mode.clone(),
                pid: Some(managed.child.id()),
                running: managed.running,
                outcome: managed.outcome.clone(),
            },
            stdout: managed
                .stdout
                .lock()
                .map(|value| value.clone())
                .unwrap_or_default(),
            stderr: managed
                .stderr
                .lock()
                .map(|value| value.clone())
                .unwrap_or_default(),
        })
    }

    pub fn snapshots(&mut self) -> Vec<EngineSessionOutput> {
        self.refresh_statuses();
        let ids: Vec<_> = self.children.keys().copied().collect();
        ids.into_iter()
            .filter_map(|id| self.output(id).ok())
            .collect()
    }

    pub fn pending_events(&self) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        for managed in self.children.values() {
            let Ok(captured) = managed.events.lock() else {
                continue;
            };
            events.extend(
                captured
                    .iter()
                    .filter(|event| event.sequence > managed.acknowledged_event_sequence)
                    .cloned(),
            );
        }
        events.sort_by_key(|event| (event.run_id, event.sequence));
        events
    }

    pub fn acknowledge_events(&mut self, events: &[EngineEvent]) {
        let mut acknowledgements = HashMap::new();
        for event in events {
            acknowledgements
                .entry(event.run_id)
                .and_modify(|sequence: &mut u64| *sequence = (*sequence).max(event.sequence))
                .or_insert(event.sequence);
        }
        for (run_id, sequence) in acknowledgements {
            if let Some(managed) = self.children.get_mut(&run_id) {
                managed.acknowledged_event_sequence =
                    managed.acknowledged_event_sequence.max(sequence);
                if let Ok(mut captured) = managed.events.lock() {
                    captured.retain(|event| event.sequence > sequence);
                }
            }
        }
        self.children.retain(|_, managed| managed.running);
    }

    fn refresh_statuses(&mut self) {
        for (id, managed) in &mut self.children {
            if managed.outcome == "running" {
                if let Some(protocol_outcome) = managed
                    .protocol
                    .lock()
                    .ok()
                    .and_then(|state| state.outcome.clone())
                {
                    managed.outcome = protocol_outcome;
                }
            }
            if managed.running && matches!(managed.outcome.as_str(), "failed" | "cancelled") {
                let _ = terminate_process_group(&mut managed.child, true);
                let _ = managed.child.wait();
                managed.running = false;
                continue;
            }
            let ready_for_next = managed
                .protocol
                .lock()
                .map(|state| !state.busy)
                .unwrap_or(false);
            if managed.running && ready_for_next {
                if let Some(turn) = managed.queued_messages.pop_front() {
                    managed.provider = turn.provider;
                    managed.model = turn.model;
                    if send_turn(managed, *id, &turn.message, turn.attachments).is_err() {
                        managed.outcome = "failed".into();
                    }
                }
            }
            if managed.running
                && managed.started_at.elapsed() > std::time::Duration::from_secs(1800)
            {
                let _ = terminate_process_group(&mut managed.child, true);
                let _ = managed.child.wait();
                managed.running = false;
                managed.outcome = "failed".into();
                continue;
            }
            match managed.child.try_wait() {
                Ok(Some(status)) => {
                    managed.running = false;
                    if managed.outcome == "running" {
                        let protocol_outcome = managed
                            .protocol
                            .lock()
                            .ok()
                            .and_then(|state| state.outcome.clone());
                        managed.outcome = protocol_outcome.unwrap_or_else(|| {
                            if status.success() && managed.mode == "ide" {
                                "succeeded".into()
                            } else {
                                "failed".into()
                            }
                        });
                    }
                }
                Ok(None) => managed.running = true,
                Err(_) => {
                    managed.running = false;
                    managed.outcome = "failed".into();
                }
            }
        }
    }
}

fn backend_for_provider(provider: Option<&str>) -> EngineBackend {
    if provider == Some("codex") {
        EngineBackend::Codex
    } else {
        EngineBackend::Rubyn
    }
}

fn codex_dynamic_tools() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "name": "wayfinder",
            "description": "Create, import, read, and update Rubyn Harness Wayfinder maps. Use import_map followed by create_node when bringing in an existing map; this removes the blank bootstrap node. A map_id may be a numeric ID or an exact map title.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list_maps", "get_map", "create_map", "import_map", "update_map", "create_node", "resolve_node", "retire_node"] },
                    "map_id": { "type": ["string", "integer"], "description": "Numeric map ID or exact map title" },
                    "node_id": { "type": ["string", "integer"] },
                    "title": { "type": "string" },
                    "idea": { "type": "string" },
                    "destination": { "type": "string" },
                    "notes": { "type": "string" },
                    "code_task_status": { "type": "string", "description": "Workflow column key chosen by the human for materialized code tasks" },
                    "node_type": { "type": "string", "enum": ["grill", "research", "prototype", "code", "user_action"] },
                    "question": { "type": "string" },
                    "description": { "type": "string" },
                    "outcome": { "type": "string" },
                    "resolution": { "type": "string" },
                    "model_role": { "type": "string" },
                    "effort": { "type": "string", "enum": ["low", "medium", "high"] },
                    "blocked_by": { "type": "array", "items": { "type": ["string", "integer"] } }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        },
        {
            "type": "function",
            "name": "harness_task",
            "description": "Read or manage shared Rubyn Harness tasks and todos.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["task", "todo"] },
                    "action": { "type": "string", "enum": ["list", "get", "create", "update", "complete"] },
                    "task_id": { "type": ["string", "integer"] },
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "outcome": { "type": "string" },
                    "status": { "type": "string" },
                    "blocked_by": { "type": "array", "items": { "type": "integer" } }
                },
                "required": ["kind", "action"],
                "additionalProperties": false
            }
        }
    ])
}

fn codex_tool_response(
    tool: &str,
    arguments: &serde_json::Value,
    workspace: &Path,
) -> serde_json::Value {
    let action = arguments
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("operation");
    let snapshot = workspace
        .parent()
        .map(|directory| directory.join("harness-control.json"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok());
    let read_result = match (tool, action, snapshot.as_ref()) {
        ("wayfinder", "list_maps", Some(snapshot)) => snapshot.get("wayfinder").cloned(),
        ("wayfinder", "get_map", Some(snapshot)) => {
            let reference = arguments.get("map_id");
            snapshot
                .get("wayfinder")
                .and_then(|value| value.as_array())
                .and_then(|maps| {
                    maps.iter()
                        .find(|entry| {
                            let map = entry.get("map").unwrap_or(entry);
                            reference.is_some_and(|reference| {
                                map.get("id") == Some(reference)
                                    || reference.as_str().is_some_and(|text| {
                                        map.get("id").is_some_and(|id| id == text)
                                            || map.get("title").and_then(serde_json::Value::as_str)
                                                == Some(text)
                                    })
                            })
                        })
                        .cloned()
                })
        }
        ("harness_task", "list", Some(snapshot)) => snapshot
            .get(
                if arguments.get("kind").and_then(serde_json::Value::as_str) == Some("todo") {
                    "todos"
                } else {
                    "tasks"
                },
            )
            .cloned(),
        ("harness_task", "get", Some(snapshot)) => {
            let collection =
                if arguments.get("kind").and_then(serde_json::Value::as_str) == Some("todo") {
                    "todos"
                } else {
                    "tasks"
                };
            let reference = arguments.get("task_id");
            snapshot
                .get(collection)
                .and_then(|value| value.as_array())
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| {
                            reference.is_some_and(|reference| {
                                item.get("id") == Some(reference)
                                    || reference.as_str().is_some_and(|text| {
                                        item.get("id").is_some_and(|id| id == text)
                                    })
                            })
                        })
                        .cloned()
                })
        }
        _ => None,
    };
    let text = read_result
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| format!("Rubyn Harness accepted {tool} {action}. The app will apply and audit this request."));
    serde_json::json!({
        "success": true,
        "contentItems": [{
            "type": "inputText",
            "text": text
        }]
    })
}

fn send_turn(
    managed: &mut ManagedEngine,
    id: u64,
    message: &str,
    attachments: Vec<serde_json::Value>,
) -> Result<(), EngineError> {
    let mut protocol = managed
        .protocol
        .lock()
        .map_err(|_| EngineError::ConversationClosed(id))?;
    protocol.begin_turn();
    managed.outcome = "running".into();
    let request_id = managed.next_request_id;
    managed.next_request_id = managed.next_request_id.saturating_add(1);
    let result = match managed.backend {
        EngineBackend::Rubyn => send_request(
            &managed.stdin,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": request_id, "method": "prompt",
                "params": {"sessionId": format!("harness-{id}"), "text": message, "attachments": attachments, "context": {"workspacePath": managed.worktree_path, "provider": managed.provider, "model": managed.model}}
            }),
        ),
        EngineBackend::Codex => {
            let thread_id = managed
                .backend_thread_id
                .lock()
                .ok()
                .and_then(|value| value.clone());
            match thread_id {
                Some(thread_id) => send_request(
                    &managed.stdin,
                    &serde_json::json!({
                        "id": request_id,
                        "method": "turn/start",
                        "params": {
                            "threadId": thread_id,
                            "input": [{"type":"text", "text":message}],
                            "cwd": managed.worktree_path,
                            "model": managed.model,
                            "approvalPolicy": CODEX_APPROVAL_POLICY,
                            "sandboxPolicy": {"type":"readOnly", "networkAccess":false}
                        }
                    }),
                ),
                None => Err(EngineError::ConversationClosed(id)),
            }
        }
    };
    if result.is_err() {
        protocol.busy = false;
        protocol.outcome = Some("failed".into());
        managed.outcome = "failed".into();
    }
    result
}

struct CodexCaptureConfig {
    run_id: u64,
    events: Arc<Mutex<Vec<EngineEvent>>>,
    event_sequence: Arc<AtomicU64>,
    thread_id: Arc<Mutex<Option<String>>>,
    workspace: PathBuf,
    model: Option<String>,
    resume_thread_id: Option<String>,
    initial_prompt: String,
}

fn capture_codex_protocol<R: Read + Send + 'static>(
    stream: Option<R>,
    stdin: Arc<Mutex<ChildStdin>>,
    config: CodexCaptureConfig,
) -> (Arc<Mutex<String>>, Arc<Mutex<ProtocolState>>) {
    let output = Arc::new(Mutex::new(String::new()));
    let protocol = Arc::new(Mutex::new(ProtocolState::default()));
    if let Some(stream) = stream {
        let state = Arc::clone(&protocol);
        let writer = Arc::clone(&stdin);
        thread::spawn(move || {
            let CodexCaptureConfig {
                run_id,
                events,
                event_sequence,
                thread_id,
                workspace,
                model,
                resume_thread_id,
                initial_prompt,
            } = config;
            let _ = send_request(
                &writer,
                &serde_json::json!({"id":0,"method":"initialize","params":{"clientInfo":{"name":"rubyn_harness","title":"Rubyn Harness","version":"0.1.0"},"capabilities":{"experimentalApi":true}}}),
            );
            let _ = send_request(
                &writer,
                &serde_json::json!({"method":"initialized","params":{}}),
            );
            let thread_method = if resume_thread_id.is_some() {
                "thread/resume"
            } else {
                "thread/start"
            };
            let thread_params = if let Some(thread_id) = resume_thread_id {
                serde_json::json!({"threadId":thread_id,"cwd":workspace,"model":model,"approvalPolicy":CODEX_APPROVAL_POLICY,"sandbox":"read-only"})
            } else {
                serde_json::json!({"cwd":workspace,"model":model,"approvalPolicy":CODEX_APPROVAL_POLICY,"sandbox":"read-only","dynamicTools":codex_dynamic_tools()})
            };
            let _ = send_request(
                &writer,
                &serde_json::json!({"id":1,"method":thread_method,"params":thread_params}),
            );
            let mut final_text = String::new();
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                if message.get("id").and_then(serde_json::Value::as_u64) == Some(1) {
                    if let Some(error) = message.get("error") {
                        let detail = error
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Codex could not start the conversation");
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "run/error",
                            serde_json::json!({"message":detail}),
                            &line,
                        );
                        let _ = state.lock().map(|mut value| {
                            value.mark_status("error");
                        });
                        break;
                    }
                    if let Some(value) = message
                        .pointer("/result/thread/id")
                        .and_then(serde_json::Value::as_str)
                    {
                        let value = value.to_owned();
                        let _ = thread_id
                            .lock()
                            .map(|mut current| *current = Some(value.clone()));
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "engine/thread",
                            serde_json::json!({"threadId":value}),
                            &line,
                        );
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "engine/harness_tools",
                            serde_json::json!({"version":1,"tools":["wayfinder","harness_task"]}),
                            &line,
                        );
                        let _ = send_request(
                            &writer,
                            &serde_json::json!({"id":2,"method":"turn/start","params":{"threadId":value,"input":[{"type":"text","text":initial_prompt}],"cwd":workspace,"model":model,"approvalPolicy":CODEX_APPROVAL_POLICY,"sandboxPolicy":{"type":"readOnly","networkAccess":false}}}),
                        );
                    }
                    continue;
                }
                let method = message.get("method").and_then(serde_json::Value::as_str);
                let mut params = message.get("params").cloned().unwrap_or_default();
                if method == Some("item/fileChange/requestApproval") {
                    if let Some(request_id) = message.get("id") {
                        let item_id = params
                            .get("itemId")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let changes = events.lock().ok().and_then(|current| {
                            current.iter().rev().find_map(|event| {
                                (event.kind == "tool/use"
                                    && event.payload.get("requestId")
                                        == Some(&serde_json::json!(item_id)))
                                .then(|| event.payload.pointer("/args/changes").cloned())
                                .flatten()
                            })
                        });
                        let payload =
                            codex_edit_approval_payload(request_id, item_id, changes.as_ref());
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            if payload.get("type").and_then(serde_json::Value::as_str)
                                == Some("create")
                            {
                                "file/create"
                            } else {
                                "file/edit"
                            },
                            payload,
                            &line,
                        );
                    }
                    continue;
                }
                if method == Some("item/commandExecution/requestApproval") {
                    if let Some(request_id) = message.get("id") {
                        let payload = codex_command_approval_payload(request_id, &params);
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "command/approval",
                            payload,
                            &line,
                        );
                    }
                    continue;
                }
                if method == Some("item/tool/call") {
                    let tool = params
                        .get("tool")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("dynamic_tool");
                    let arguments = params.get("arguments").cloned().unwrap_or_default();
                    let call_id = params.get("callId").cloned().unwrap_or_else(|| {
                        serde_json::json!(format!("tool-{}", event_sequence.load(Ordering::SeqCst)))
                    });
                    let already_started = events.lock().ok().is_some_and(|current| {
                        current.iter().any(|event| {
                            event.kind == "tool/use"
                                && event.payload.get("requestId") == Some(&call_id)
                        })
                    });
                    if !already_started {
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "tool/use",
                            serde_json::json!({"requestId":call_id,"tool":tool,"args":arguments}),
                            &line,
                        );
                    }
                    if let Some(id) = message.get("id") {
                        let _ = send_request(
                            &writer,
                            &serde_json::json!({"id":id,"result":codex_tool_response(tool, &arguments, &workspace)}),
                        );
                    }
                    continue;
                }
                if method == Some("ide/askUser") {
                    if let Some(request_id) = message.get("id") {
                        if let Some(object) = params.as_object_mut() {
                            object.insert("requestId".into(), request_id.clone());
                        }
                    }
                    push_event(&events, &event_sequence, run_id, "ide/askUser", params, "");
                    continue;
                }
                match method {
                    Some("item/agentMessage/delta") => {
                        if let Some(delta) = params.get("delta").and_then(serde_json::Value::as_str)
                        {
                            final_text.push_str(delta);
                            push_event(
                                &events,
                                &event_sequence,
                                run_id,
                                "stream/text",
                                serde_json::json!({"text":delta,"final":false}),
                                &line,
                            );
                        }
                    }
                    Some("item/reasoning/summaryTextDelta") => {
                        if let Some(delta) = params.get("delta").and_then(serde_json::Value::as_str)
                        {
                            push_event(
                                &events,
                                &event_sequence,
                                run_id,
                                "reasoning/delta",
                                serde_json::json!({
                                    "itemId": params.get("itemId"),
                                    "text": delta,
                                    "summary": true
                                }),
                                &line,
                            );
                        }
                    }
                    Some("item/started") => {
                        if let Some((kind, payload)) = normalize_codex_item(&params, false) {
                            push_event(&events, &event_sequence, run_id, kind, payload, &line);
                        }
                    }
                    Some("item/completed")
                        if params
                            .pointer("/item/type")
                            .and_then(serde_json::Value::as_str)
                            == Some("agentMessage") =>
                    {
                        if let Some(text) = params
                            .pointer("/item/text")
                            .and_then(serde_json::Value::as_str)
                        {
                            final_text = text.to_owned();
                        }
                    }
                    Some("item/completed") => {
                        if let Some((kind, payload)) = normalize_codex_item(&params, true) {
                            push_event(&events, &event_sequence, run_id, kind, payload, &line);
                        }
                    }
                    Some("item/commandExecution/outputDelta")
                    | Some("item/fileChange/outputDelta")
                    | Some("item/mcpToolCall/progress") => {
                        let text = params
                            .get("delta")
                            .or_else(|| params.get("message"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        if !text.is_empty() {
                            push_event(
                                &events,
                                &event_sequence,
                                run_id,
                                "tool/progress",
                                serde_json::json!({
                                    "requestId": params.get("itemId"),
                                    "text": text
                                }),
                                &line,
                            );
                        }
                    }
                    Some("thread/tokenUsage/updated") => {
                        if let Some(payload) = codex_token_usage_payload(&params) {
                            push_event(
                                &events,
                                &event_sequence,
                                run_id,
                                "token/usage",
                                payload,
                                "",
                            );
                        }
                    }
                    Some("turn/completed") => {
                        let turn_status = params
                            .pointer("/turn/status")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("completed");
                        let harness_status = match turn_status {
                            "failed" => "error",
                            "interrupted" | "cancelled" => "cancelled",
                            _ => "done",
                        };
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "agent/status",
                            serde_json::json!({"status":harness_status}),
                            &line,
                        );
                        push_event(
                            &events,
                            &event_sequence,
                            run_id,
                            "stream/text",
                            serde_json::json!({"text":final_text,"final":true}),
                            &line,
                        );
                        final_text.clear();
                        let _ = state.lock().map(|mut value| {
                            value.mark_status(harness_status);
                            if harness_status == "done" {
                                value.mark_final_text();
                            }
                        });
                    }
                    _ => {}
                }
            }
        });
    }
    (output, protocol)
}

fn normalize_codex_item(
    params: &serde_json::Value,
    completed: bool,
) -> Option<(&'static str, serde_json::Value)> {
    let item = params.get("item")?;
    let item_type = item.get("type")?.as_str()?;
    let request_id = item.get("id").cloned().unwrap_or_default();
    let (tool, args) = match item_type {
        "commandExecution" => (
            "shell",
            serde_json::json!({"command": item.get("command"), "cwd": item.get("cwd")}),
        ),
        "fileChange" => (
            "file_change",
            serde_json::json!({"changes": item.get("changes")}),
        ),
        "mcpToolCall" => (
            item.get("tool")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("mcp_tool"),
            serde_json::json!({
                "server": item.get("server"),
                "arguments": item.get("arguments")
            }),
        ),
        "dynamicToolCall" => (
            item.get("tool")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("dynamic_tool"),
            item.get("arguments").cloned().unwrap_or_default(),
        ),
        "collabAgentToolCall" => (
            item.get("tool")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("agent"),
            serde_json::json!({"prompt": item.get("prompt"), "model": item.get("model")}),
        ),
        "webSearch" => (
            "web_search",
            serde_json::json!({"query": item.get("query")}),
        ),
        _ => return None,
    };
    if !completed {
        return Some((
            "tool/use",
            serde_json::json!({
                "requestId": request_id,
                "tool": tool,
                "args": args,
                "requiresApproval": false
            }),
        ));
    }

    let status = item
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("completed");
    let success = !matches!(status, "failed" | "declined");
    let summary = match item_type {
        "commandExecution" => item
            .get("exitCode")
            .and_then(serde_json::Value::as_i64)
            .map(|code| format!("Command exited with code {code}"))
            .unwrap_or_else(|| "Command finished".to_owned()),
        "fileChange" => item
            .get("changes")
            .and_then(serde_json::Value::as_array)
            .map(|changes| {
                format!(
                    "Updated {} file{}",
                    changes.len(),
                    if changes.len() == 1 { "" } else { "s" }
                )
            })
            .unwrap_or_else(|| "File changes finished".to_owned()),
        "webSearch" => item
            .get("query")
            .and_then(serde_json::Value::as_str)
            .map(|query| format!("Searched for {query}"))
            .unwrap_or_else(|| "Web search finished".to_owned()),
        _ if success => "Finished".to_owned(),
        _ => item
            .pointer("/error/message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Tool failed")
            .to_owned(),
    };
    Some((
        "tool/result",
        serde_json::json!({
            "requestId": request_id,
            "tool": tool,
            "success": success,
            "summary": summary
        }),
    ))
}

fn codex_edit_approval_payload(
    request_id: &serde_json::Value,
    item_id: &str,
    changes: Option<&serde_json::Value>,
) -> serde_json::Value {
    let changes = changes
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let paths: Vec<_> = changes
        .iter()
        .filter_map(|change| change.get("path").and_then(serde_json::Value::as_str))
        .collect();
    let content = changes
        .iter()
        .map(|change| {
            let path = change
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown file");
            let diff = change
                .get("diff")
                .or_else(|| change.get("unified_diff"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Diff details were not supplied by Codex.");
            format!("--- {path}\n{diff}")
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let creates_only = !changes.is_empty()
        && changes.iter().all(|change| {
            change
                .pointer("/kind/type")
                .and_then(serde_json::Value::as_str)
                == Some("add")
        });
    serde_json::json!({
        "editId": serde_json::json!({"requestId": request_id, "itemId": item_id}).to_string(),
        "path": match paths.as_slice() {
            [] => "Codex file change".to_owned(),
            [path] => (*path).to_owned(),
            _ => format!("{} files: {}", paths.len(), paths.join(", ")),
        },
        "content": if content.is_empty() {
            "Codex requested permission to change files, but did not supply a preview.".to_owned()
        } else {
            content
        },
        "type": if creates_only { "create" } else { "modify" },
        "approvalKind": "fileChange"
    })
}

fn codex_command_approval_payload(
    request_id: &serde_json::Value,
    params: &serde_json::Value,
) -> serde_json::Value {
    let item_id = params
        .get("itemId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let approval_id = params.get("approvalId").cloned().unwrap_or_default();
    let command = params
        .get("command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Command details were not supplied by Codex.");
    let cwd = params
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Working directory was not supplied by Codex.");
    let reason = params
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .filter(|reason| !reason.trim().is_empty());
    let content = match reason {
        Some(reason) => format!("{command}\n\nReason: {reason}"),
        None => command.to_owned(),
    };
    serde_json::json!({
        "editId": serde_json::json!({
            "requestId": request_id,
            "itemId": item_id,
            "approvalId": approval_id
        }).to_string(),
        "path": cwd,
        "content": content,
        "type": "command",
        "approvalKind": "commandExecution"
    })
}

fn codex_token_usage_payload(params: &serde_json::Value) -> Option<serde_json::Value> {
    let total = params.pointer("/tokenUsage/total")?;
    let number = |key: &str| total.get(key).and_then(serde_json::Value::as_u64);
    Some(serde_json::json!({
        "inputTokens": number("inputTokens")?,
        "cachedInputTokens": number("cachedInputTokens").unwrap_or(0),
        "outputTokens": number("outputTokens")?,
        "reasoningOutputTokens": number("reasoningOutputTokens").unwrap_or(0),
        "totalTokens": number("totalTokens")?,
        "source": "provider"
    }))
}

/// Runs a single JSON-RPC request against Rubyn's IDE transport. This keeps
/// provider configuration owned by Rubyn Code, including its encrypted token
/// store, while giving the native host a typed management surface.
pub fn one_shot_rpc(
    location: EngineLocation,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, EngineError> {
    let mut command = match location {
        EngineLocation::Bundled { root, ruby } => {
            let mut command = Command::new(ruby);
            command
                .arg("-I")
                .arg(root.join("lib"))
                .arg(root.join("exe/rubyn-code"));
            command
        }
        EngineLocation::Installed => Command::new("rubyn-code"),
    };
    let mut child = command
        .arg("--ide")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let request = serde_json::json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params});
    if let Some(stdin) = child.stdin.as_mut() {
        writeln!(stdin, "{}", request).map_err(EngineError::Launch)?;
        stdin.flush().map_err(EngineError::Launch)?;
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| EngineError::Launch(std::io::Error::other("engine stdout unavailable")))?;
    let mut result = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(EngineError::Launch)?;
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(serde_json::Value::as_u64) != Some(1) {
            continue;
        }
        if let Some(error) = message.get("error") {
            result = Some(Err(EngineError::Launch(std::io::Error::other(
                error.to_string(),
            ))));
        } else {
            result = Some(Ok(message
                .get("result")
                .cloned()
                .unwrap_or(serde_json::Value::Null)));
        }
        break;
    }
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap_or_else(|| {
        Err(EngineError::Launch(std::io::Error::other(
            "Rubyn Code returned no RPC response",
        )))
    })
}

pub fn codex_one_shot_rpc(
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, EngineError> {
    let mut child = Command::new(codex_executable()?)
        .arg("app-server")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| EngineError::Launch(std::io::Error::other("Codex stdin unavailable")))?;
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"id":0,"method":"initialize","params":{"clientInfo":{"name":"rubyn_harness","title":"Rubyn Harness","version":"0.1.0"},"capabilities":{"experimentalApi":true}}})
    )?;
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"method":"initialized","params":{}})
    )?;
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"id":1,"method":method,"params":params})
    )?;
    stdin.flush()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| EngineError::Launch(std::io::Error::other("Codex stdout unavailable")))?;
    let mut result = None;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(serde_json::Value::as_u64) != Some(1) {
            continue;
        }
        result = Some(if let Some(error) = message.get("error") {
            Err(EngineError::Launch(std::io::Error::other(
                error.to_string(),
            )))
        } else {
            Ok(message
                .get("result")
                .cloned()
                .unwrap_or(serde_json::Value::Null))
        });
        break;
    }
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap_or_else(|| {
        Err(EngineError::Launch(std::io::Error::other(
            "Codex returned no RPC response",
        )))
    })
}

const MAX_ATTACHMENT_COUNT: usize = 10;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;

fn prepare_attachments(inputs: &[AttachmentInput]) -> Result<Vec<serde_json::Value>, EngineError> {
    if inputs.len() > MAX_ATTACHMENT_COUNT {
        return Err(EngineError::Attachment(format!(
            "select at most {MAX_ATTACHMENT_COUNT} files"
        )));
    }
    let mut total = 0_u64;
    inputs.iter().map(|input| {
        let path = PathBuf::from(&input.path).canonicalize().map_err(|error| EngineError::Attachment(format!("{}: {error}", input.path)))?;
        let metadata = path.metadata().map_err(|error| EngineError::Attachment(format!("{}: {error}", input.path)))?;
        if !metadata.is_file() { return Err(EngineError::Attachment(format!("{} is not a regular file", input.path))); }
        total = total.saturating_add(metadata.len());
        if total > MAX_TOTAL_ATTACHMENT_BYTES { return Err(EngineError::Attachment("attachments exceed the 20 MiB total limit".into())); }
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("attachment").to_owned();
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
        let media_type = match extension.as_str() {
            "png" => Some("image/png"), "jpg" | "jpeg" => Some("image/jpeg"),
            "gif" => Some("image/gif"), "webp" => Some("image/webp"), _ => None,
        };
        if let Some(media_type) = media_type {
            if metadata.len() > MAX_IMAGE_BYTES { return Err(EngineError::Attachment(format!("{name} exceeds the 8 MiB image limit"))); }
            let bytes = std::fs::read(&path).map_err(|error| EngineError::Attachment(format!("{name}: {error}")))?;
            Ok(serde_json::json!({"type":"image", "name":name, "mediaType":media_type, "data":BASE64.encode(bytes)}))
        } else {
            if metadata.len() > MAX_TEXT_BYTES { return Err(EngineError::Attachment(format!("{name} exceeds the 2 MiB text limit"))); }
            let bytes = std::fs::read(&path).map_err(|error| EngineError::Attachment(format!("{name}: {error}")))?;
            let text = String::from_utf8(bytes).map_err(|_| EngineError::Attachment(format!("{name} is not a supported image or UTF-8 text file")))?;
            Ok(serde_json::json!({"type":"text", "name":name, "text":text}))
        }
    }).collect()
}

impl Drop for EngineSupervisor {
    fn drop(&mut self) {
        for managed in self.children.values_mut() {
            if managed.child.try_wait().ok().flatten().is_none() {
                let _ = terminate_process_group(&mut managed.child, true);
                let _ = managed.child.wait();
            }
        }
    }
}

fn terminate_process_group(child: &mut Child, force: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        // SAFETY: the child was spawned into a new process group whose id is its pid.
        let result = unsafe { libc::kill(-(child.id() as i32), signal) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        child.kill()
    }
}

fn send_request(
    stdin: &Arc<Mutex<ChildStdin>>,
    payload: &serde_json::Value,
) -> Result<(), EngineError> {
    let mut stdin = stdin
        .lock()
        .map_err(|_| EngineError::Launch(std::io::Error::other("engine stdin unavailable")))?;
    writeln!(stdin, "{payload}")?;
    stdin.flush()?;
    Ok(())
}

fn capture_protocol<R: Read + Send + 'static>(
    run_id: u64,
    stream: Option<R>,
    stdin: Arc<Mutex<ChildStdin>>,
    events: Arc<Mutex<Vec<EngineEvent>>>,
    event_sequence: Arc<AtomicU64>,
) -> (Arc<Mutex<String>>, Arc<Mutex<ProtocolState>>) {
    let output = Arc::new(Mutex::new(String::new()));
    let protocol = Arc::new(Mutex::new(ProtocolState::default()));
    if let Some(stream) = stream {
        let captured = Arc::clone(&output);
        let state = Arc::clone(&protocol);
        thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                append_capture(&captured, &line);
                let message = serde_json::from_str::<serde_json::Value>(&line)
                    .unwrap_or_else(|_| serde_json::json!({"text": line}));
                let method = message.get("method").and_then(|value| value.as_str());
                let params = message.get("params").cloned().unwrap_or_default();
                push_event(
                    &events,
                    &event_sequence,
                    run_id,
                    method.unwrap_or("process/stdout"),
                    if method.is_some() {
                        params.clone()
                    } else {
                        message.clone()
                    },
                    &line,
                );
                match method {
                    Some("tool/use")
                        if params
                            .get("requiresApproval")
                            .and_then(|value| value.as_bool())
                            == Some(true) =>
                    {
                        if let Some(request_id) = params.get("requestId") {
                            let _ = send_request(
                                &stdin,
                                &serde_json::json!({"jsonrpc":"2.0","id":format!("deny-{request_id}"),"method":"approveToolUse","params":{"requestId":request_id,"approved":false}}),
                            );
                        }
                    }
                    Some("agent/status") => {
                        match params.get("status").and_then(|value| value.as_str()) {
                            Some("done") => {
                                let _ = state.lock().map(|mut state| state.mark_status("done"));
                            }
                            Some("error") => {
                                let _ = state.lock().map(|mut state| state.mark_status("error"));
                            }
                            Some("cancelled") => {
                                let _ =
                                    state.lock().map(|mut state| state.mark_status("cancelled"));
                            }
                            _ => {}
                        }
                    }
                    Some("stream/text")
                        if params.get("final").and_then(|value| value.as_bool()) == Some(true) =>
                    {
                        let _ = state.lock().map(|mut state| state.mark_final_text());
                    }
                    _ => {}
                }
            }
        });
    }
    (output, protocol)
}

fn append_capture(output: &Arc<Mutex<String>>, chunk: &str) {
    if let Ok(mut value) = output.lock() {
        let remaining = MAX_CAPTURE_BYTES.saturating_sub(value.len());
        if remaining > 0 {
            let mut boundary = chunk.len().min(remaining);
            while boundary > 0 && !chunk.is_char_boundary(boundary) {
                boundary -= 1;
            }
            value.push_str(&chunk[..boundary]);
            value.push('\n');
        }
    }
}

fn capture<R: Read + Send + 'static>(
    run_id: u64,
    stream: Option<R>,
    events: Arc<Mutex<Vec<EngineEvent>>>,
    event_sequence: Arc<AtomicU64>,
) -> Arc<Mutex<String>> {
    let output = Arc::new(Mutex::new(String::new()));
    if let Some(mut stream) = stream {
        let captured = Arc::clone(&output);
        thread::spawn(move || {
            let mut reported = false;
            let mut buffer = [0_u8; 4096];
            while let Ok(count) = stream.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                if !reported {
                    let summary = "Engine process emitted diagnostic output; details were withheld to protect local credentials.";
                    push_event(
                        &events,
                        &event_sequence,
                        run_id,
                        "process/stderr",
                        serde_json::json!({"text": summary}),
                        "",
                    );
                    if let Ok(mut value) = captured.lock() {
                        value.push_str(summary);
                        value.push('\n');
                    }
                    reported = true;
                }
            }
        });
    }
    output
}

fn push_event(
    events: &Arc<Mutex<Vec<EngineEvent>>>,
    sequence: &Arc<AtomicU64>,
    run_id: u64,
    kind: &str,
    payload: serde_json::Value,
    _raw: &str,
) {
    let event = EngineEvent {
        run_id,
        sequence: sequence.fetch_add(1, Ordering::Relaxed),
        kind: kind.to_owned(),
        payload,
        raw: String::new(),
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
    };
    if let Ok(mut captured) = events.lock() {
        captured.push(event);
    }
}

fn engine_command(
    location: EngineLocation,
    project: &Path,
    request: &LaunchEngineRequest,
) -> Result<Command, EngineError> {
    if matches!(&request.mode, EngineLaunchMode::Prompt { prompt } if prompt.trim().is_empty()) {
        return Err(EngineError::EmptyPrompt);
    }
    let mut arguments = vec![
        "--ide".to_owned(),
        "--dir".to_owned(),
        project.to_string_lossy().into_owned(),
        "--permission-mode".to_owned(),
        "default".to_owned(),
    ];
    if request.yolo {
        arguments.push("--yolo".to_owned());
    }
    let mut command = match location {
        EngineLocation::Bundled { root, ruby } => {
            let mut command = Command::new(ruby);
            command
                .arg("-I")
                .arg(root.join("lib"))
                .arg(root.join("exe/rubyn-code"));
            command
        }
        EngineLocation::Installed => Command::new("rubyn-code"),
    };
    if let Some(control_directory) = project.parent() {
        command.env(
            "RUBYN_HARNESS_CONTROL_FILE",
            control_directory.join("harness-control.json"),
        );
    }
    command.current_dir(project).args(arguments);
    Ok(command)
}

fn codex_command(project: &Path, request: &LaunchEngineRequest) -> Result<Command, EngineError> {
    codex_command_with_program(project, request, codex_executable()?)
}

fn codex_command_with_program(
    project: &Path,
    request: &LaunchEngineRequest,
    program: impl AsRef<Path>,
) -> Result<Command, EngineError> {
    if matches!(&request.mode, EngineLaunchMode::Prompt { prompt } if prompt.trim().is_empty()) {
        return Err(EngineError::EmptyPrompt);
    }
    if !request.attachments.is_empty() {
        return Err(EngineError::Attachment(
            "Codex conversations do not support harness attachments yet".into(),
        ));
    }
    let mut command = Command::new(program.as_ref());
    command
        .arg("app-server")
        .arg("--stdio")
        .current_dir(project);
    Ok(command)
}

fn mode_label(mode: &EngineLaunchMode) -> String {
    match mode {
        EngineLaunchMode::Ide => "ide".into(),
        EngineLaunchMode::Prompt { .. } => "prompt".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_prompt_before_spawning() {
        let request = LaunchEngineRequest {
            project_path: "/tmp".into(),
            mode: EngineLaunchMode::Prompt {
                prompt: "  ".into(),
            },
            yolo: false,
            attachments: vec![],
            provider: None,
            model: None,
            resume_session: false,
            backend_thread_id: None,
        };
        assert!(matches!(
            engine_command(EngineLocation::Installed, Path::new("/tmp"), &request),
            Err(EngineError::EmptyPrompt)
        ));
    }

    #[test]
    fn prompt_runs_use_the_typed_ide_protocol() {
        let request = LaunchEngineRequest {
            project_path: "/tmp".into(),
            mode: EngineLaunchMode::Prompt {
                prompt: "Inspect the app".into(),
            },
            yolo: false,
            attachments: vec![],
            provider: Some("openai".into()),
            model: Some("gpt-5.4".into()),
            resume_session: false,
            backend_thread_id: None,
        };
        let command = engine_command(EngineLocation::Installed, Path::new("/tmp"), &request)
            .expect("command should be valid");
        let args: Vec<_> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert!(args.starts_with(&[
            "--ide".into(),
            "--dir".into(),
            "/tmp".into(),
            "--permission-mode".into(),
            "default".into(),
        ]));
        assert!(!args.iter().any(|argument| argument == "-p"));
    }

    #[test]
    fn codex_runs_use_the_app_server_instead_of_copying_oauth_tokens() {
        assert_eq!(CODEX_APPROVAL_POLICY, "untrusted");
        let request = LaunchEngineRequest {
            project_path: "/tmp".into(),
            mode: EngineLaunchMode::Prompt {
                prompt: "Inspect the app".into(),
            },
            yolo: false,
            attachments: vec![],
            provider: Some("codex".into()),
            model: Some("gpt-5.6-terra".into()),
            resume_session: false,
            backend_thread_id: None,
        };

        let command = codex_command_with_program(Path::new("/tmp"), &request, "/test/codex")
            .expect("Codex command should be valid");
        assert_eq!(command.get_program(), "/test/codex");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["app-server", "--stdio"]
        );
        assert!(command
            .get_envs()
            .all(|(name, _)| !name.to_string_lossy().contains("TOKEN")));
    }

    #[test]
    fn normalizes_codex_command_activity_for_the_conversation_timeline() {
        let started = serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "command-7",
                "command": "bundle exec rspec",
                "cwd": "/work/app",
                "status": "inProgress"
            }
        });
        let (kind, payload) = normalize_codex_item(&started, false).expect("tool start");
        assert_eq!(kind, "tool/use");
        assert_eq!(payload["requestId"], "command-7");
        assert_eq!(payload["tool"], "shell");
        assert_eq!(payload["args"]["command"], "bundle exec rspec");

        let completed = serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "command-7",
                "command": "bundle exec rspec",
                "cwd": "/work/app",
                "status": "completed",
                "exitCode": 0
            }
        });
        let (kind, payload) = normalize_codex_item(&completed, true).expect("tool result");
        assert_eq!(kind, "tool/result");
        assert_eq!(payload["success"], true);
        assert_eq!(payload["summary"], "Command exited with code 0");
    }

    #[test]
    fn normalizes_codex_file_approval_into_one_auditable_decision() {
        let request_id = serde_json::json!(42);
        let changes = serde_json::json!([
            {
                "path": "app/controllers/posts_controller.rb",
                "diff": "@@ -1 +1 @@\n-old\n+new",
                "kind": {"type": "update"}
            },
            {
                "path": "test/controllers/posts_controller_test.rb",
                "diff": "@@ -0,0 +1 @@\n+test",
                "kind": {"type": "add"}
            }
        ]);

        let payload = codex_edit_approval_payload(&request_id, "file-change-42", Some(&changes));

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(payload["editId"].as_str().unwrap()).unwrap(),
            serde_json::json!({"requestId": 42, "itemId": "file-change-42"})
        );
        assert_eq!(
            payload["path"],
            "2 files: app/controllers/posts_controller.rb, test/controllers/posts_controller_test.rb"
        );
        assert_eq!(payload["type"], "modify");
        assert!(payload["content"]
            .as_str()
            .is_some_and(|content| content.contains("posts_controller_test.rb")));
    }

    #[test]
    fn normalizes_codex_command_approval_without_broadening_authority() {
        let request_id = serde_json::json!(73);
        let params = serde_json::json!({
            "threadId": "thread-3",
            "turnId": "turn-4",
            "itemId": "command-9",
            "approvalId": "approval-12",
            "command": "bundle exec rails test",
            "cwd": "/work/example-app",
            "reason": "Run the repository test suite",
            "availableDecisions": ["accept", "acceptForSession", "decline"]
        });

        let payload = codex_command_approval_payload(&request_id, &params);
        let identity = serde_json::from_str::<serde_json::Value>(
            payload["editId"].as_str().expect("encoded identity"),
        )
        .unwrap();

        assert_eq!(
            identity,
            serde_json::json!({
                "requestId": 73,
                "itemId": "command-9",
                "approvalId": "approval-12"
            })
        );
        assert_eq!(payload["approvalKind"], "commandExecution");
        assert_eq!(payload["type"], "command");
        assert_eq!(payload["path"], "/work/example-app");
        assert_eq!(
            payload["content"],
            "bundle exec rails test\n\nReason: Run the repository test suite"
        );
        assert!(payload.get("availableDecisions").is_none());
    }

    #[test]
    fn provider_frames_are_not_written_to_the_audit_log() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sequence = Arc::new(AtomicU64::new(1));

        push_event(
            &events,
            &sequence,
            3,
            "agent/status",
            serde_json::json!({"status":"done"}),
            "provider-frame-with-local-environment",
        );

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].raw.is_empty());
        assert!(!events[0].raw.contains("local-environment"));
    }

    #[test]
    fn normalizes_only_numeric_provider_token_usage() {
        let payload = codex_token_usage_payload(&serde_json::json!({
            "threadId": "thread-private",
            "tokenUsage": {
                "total": {
                    "inputTokens": 40_000,
                    "cachedInputTokens": 30_000,
                    "outputTokens": 1_200,
                    "reasoningOutputTokens": 200,
                    "totalTokens": 41_200,
                    "accountMetadata": "must-not-be-retained"
                }
            }
        }))
        .unwrap();

        assert_eq!(payload["inputTokens"], 40_000);
        assert_eq!(payload["cachedInputTokens"], 30_000);
        assert_eq!(payload["source"], "provider");
        assert!(payload.get("threadId").is_none());
        assert!(payload.get("accountMetadata").is_none());
    }

    #[test]
    fn codex_threads_receive_native_harness_tools() {
        let tools = codex_dynamic_tools();
        let tools = tools.as_array().expect("dynamic tools");
        assert!(tools.iter().any(|tool| tool["name"] == "wayfinder"));
        assert!(tools.iter().any(|tool| tool["name"] == "harness_task"));
        assert!(tools
            .iter()
            .find(|tool| tool["name"] == "wayfinder")
            .and_then(|tool| tool.pointer("/inputSchema/properties/action/enum"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| actions.iter().any(|action| action == "import_map")));
    }

    #[test]
    fn normalizes_codex_dynamic_tool_arguments_for_the_harness_control_plane() {
        let started = serde_json::json!({
            "item": {
                "type": "dynamicToolCall",
                "id": "wayfinder-7",
                "tool": "wayfinder",
                "arguments": {"action":"import_map","title":"PO chaser pivot"}
            }
        });
        let (kind, payload) = normalize_codex_item(&started, false).expect("dynamic tool start");
        assert_eq!(kind, "tool/use");
        assert_eq!(payload["tool"], "wayfinder");
        assert_eq!(payload["args"]["action"], "import_map");
        assert_eq!(payload["args"]["title"], "PO chaser pivot");
    }

    #[test]
    fn detects_an_executable_codex_file() {
        let root = std::env::temp_dir().join(format!("rubyn-codex-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary Codex directory");
        let executable = root.join("codex");
        std::fs::write(&executable, "#!/bin/sh\n").expect("Codex fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = executable
                .metadata()
                .expect("fixture metadata")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&executable, permissions).expect("executable fixture");
        }
        assert!(is_executable_file(&executable));
        let _ = std::fs::remove_file(executable);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn prepares_image_and_text_attachments_for_the_ide_protocol() {
        let root = std::env::temp_dir().join(format!("rubyn-attachment-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary attachment directory");
        let image = root.join("screen.png");
        let text_file = root.join("model.rb");
        std::fs::write(&image, [1_u8, 2, 3]).expect("image fixture");
        std::fs::write(&text_file, "class User; end\n").expect("text fixture");

        let prepared = prepare_attachments(&[
            AttachmentInput {
                path: image.to_string_lossy().into_owned(),
            },
            AttachmentInput {
                path: text_file.to_string_lossy().into_owned(),
            },
        ])
        .expect("valid attachments");

        assert_eq!(prepared[0]["type"], "image");
        assert_eq!(prepared[0]["mediaType"], "image/png");
        assert_eq!(prepared[0]["data"], "AQID");
        assert_eq!(prepared[1]["type"], "text");
        assert_eq!(prepared[1]["text"], "class User; end\n");
        let _ = std::fs::remove_file(image);
        let _ = std::fs::remove_file(text_file);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn success_waits_for_done_and_final_text() {
        let mut state = ProtocolState::default();
        assert!(!state.mark_status("done"));
        assert_eq!(state.outcome, None);
        assert!(state.mark_final_text());
        assert_eq!(state.outcome.as_deref(), Some("waiting"));
    }

    #[test]
    fn protocol_errors_override_a_clean_shutdown() {
        let mut state = ProtocolState::default();
        assert!(state.mark_status("error"));
        assert_eq!(state.outcome.as_deref(), Some("failed"));
    }

    #[test]
    fn protocol_cancellation_is_a_terminal_outcome() {
        let mut state = ProtocolState::default();
        assert!(state.mark_status("cancelled"));
        assert_eq!(state.outcome.as_deref(), Some("cancelled"));
        assert!(!state.mark_final_text());
        assert_eq!(state.outcome.as_deref(), Some("cancelled"));
    }
}
