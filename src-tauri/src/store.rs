use crate::{
    domain::{
        AgentProfile, CreateWayfinderTicketRequest, DiagnosticStateSummary, EditApprovalRecord,
        LocalAppState, ProjectData, ProjectRecord, RunEventRecord, RunRecord, TaskRecord,
        TodoRecord, UpdateWayfinderTicketRequest, WayfinderAnswer, WayfinderEvent, WayfinderMap,
        WayfinderMapData, WayfinderQuestion, WayfinderTicket, WorkflowColumn,
    },
    engine::EngineEvent,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const DATABASE_VERSION: u32 = 9;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Unable to access Rubyn Harness local state: {0}")]
    Io(#[from] io::Error),
    #[error("Rubyn Harness local state is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0} was not found")]
    NotFound(String),
    #[error("Invalid local data: {0}")]
    Validation(String),
    #[error("The requested operation conflicts with current run state: {0}")]
    Conflict(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct PersistentDatabase {
    version: u32,
    app_state: LocalAppState,
    projects: Vec<ProjectRecord>,
    agents: Vec<AgentProfile>,
    columns: Vec<WorkflowColumn>,
    tasks: Vec<TaskRecord>,
    todos: Vec<TodoRecord>,
    runs: Vec<RunRecord>,
    events: Vec<RunEventRecord>,
    approvals: Vec<EditApprovalRecord>,
    wayfinder_maps: Vec<WayfinderMap>,
    wayfinder_tickets: Vec<WayfinderTicket>,
    wayfinder_questions: Vec<WayfinderQuestion>,
    wayfinder_events: Vec<WayfinderEvent>,
    next_project_id: u64,
    next_agent_id: u64,
    next_column_id: u64,
    next_task_id: u64,
    next_todo_id: u64,
    next_run_id: u64,
    next_event_id: u64,
    next_approval_id: u64,
    next_wayfinder_map_id: u64,
    next_wayfinder_ticket_id: u64,
    next_wayfinder_question_id: u64,
    next_wayfinder_event_id: u64,
}

impl Default for PersistentDatabase {
    fn default() -> Self {
        Self {
            version: DATABASE_VERSION,
            app_state: LocalAppState::default(),
            projects: Vec::new(),
            agents: Vec::new(),
            columns: Vec::new(),
            tasks: Vec::new(),
            todos: Vec::new(),
            runs: Vec::new(),
            events: Vec::new(),
            approvals: Vec::new(),
            wayfinder_maps: Vec::new(),
            wayfinder_tickets: Vec::new(),
            wayfinder_questions: Vec::new(),
            wayfinder_events: Vec::new(),
            next_project_id: 1,
            next_agent_id: 1,
            next_column_id: 1,
            next_task_id: 1,
            next_todo_id: 1,
            next_run_id: 1,
            next_event_id: 1,
            next_approval_id: 1,
            next_wayfinder_map_id: 1,
            next_wayfinder_ticket_id: 1,
            next_wayfinder_question_id: 1,
            next_wayfinder_event_id: 1,
        }
    }
}

pub struct StateRepository {
    directory: PathBuf,
    file: PathBuf,
    backup: PathBuf,
    database: PersistentDatabase,
}

impl StateRepository {
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, StoreError> {
        let directory = directory.as_ref().to_path_buf();
        fs::create_dir_all(&directory)?;
        let file = directory.join("harness-database.json");
        let backup = directory.join("harness-database.backup.json");
        let legacy = directory.join("state.json");
        let mut restore_primary = false;
        let database = if file.is_file() {
            match read_database(&file) {
                Ok(database) => database,
                Err(primary_error) => match read_database(&backup) {
                    Ok(database) => {
                        restore_primary = true;
                        database
                    }
                    Err(_) => return Err(primary_error),
                },
            }
        } else {
            let mut database = PersistentDatabase::default();
            if legacy.is_file() {
                database.app_state = serde_json::from_slice(&fs::read(legacy)?)?;
            }
            database
        };

        let mut repository = Self {
            directory,
            file,
            backup,
            database,
        };
        let previous_version = repository.database.version;
        repository.repair_counters();
        repository.ensure_workflow_columns();
        repository.migrate_wayfinder_task_columns();
        if previous_version < 6 {
            repository.ensure_agent_profiles();
            repository.migrate_column_policies();
        }
        if previous_version < 7 {
            repository.migrate_default_agent_instructions();
        }
        repository.migrate_legacy_task_statuses();
        repository.migrate_conversation_metadata();
        repository.refresh_task_readiness();
        let _ = repository.recover_interrupted_runs();
        let scrubbed_provider_diagnostics = repository.scrub_provider_diagnostics();
        let _ = restore_primary;
        // Opening is also the migration boundary; persist normalized columns and statuses.
        repository.save()?;
        if scrubbed_provider_diagnostics {
            fs::copy(&repository.file, &repository.backup)?;
            File::open(&repository.backup)?.sync_all()?;
        }
        Ok(repository)
    }

    pub fn snapshot(&self) -> LocalAppState {
        self.database.app_state.clone()
    }

    pub fn diagnostic_summary(&self) -> DiagnosticStateSummary {
        let mut lifecycle_counts = BTreeMap::new();
        for run in &self.database.runs {
            *lifecycle_counts.entry(run.lifecycle.clone()).or_insert(0) += 1;
        }
        DiagnosticStateSummary {
            schema_version: self.database.version,
            project_count: self.database.projects.len(),
            trusted_project_count: self.database.app_state.trusted_project_paths.len(),
            run_count: self.database.runs.len(),
            running_run_count: self.database.runs.iter().filter(|run| run.running).count(),
            lifecycle_counts,
            pending_approval_count: self
                .database
                .approvals
                .iter()
                .filter(|approval| approval.status == "pending")
                .count(),
            wayfinder_map_count: self.database.wayfinder_maps.len(),
        }
    }

    pub fn managed_worktree_inventory(&self) -> Vec<(PathBuf, PathBuf)> {
        self.database
            .runs
            .iter()
            .filter_map(|run| {
                let project = self
                    .database
                    .projects
                    .iter()
                    .find(|project| project.id == run.project_id)?;
                Some((
                    PathBuf::from(&project.path),
                    PathBuf::from(&run.worktree_path),
                ))
            })
            .collect()
    }

    pub fn replace(&mut self, state: LocalAppState) -> Result<LocalAppState, StoreError> {
        self.database.app_state = state.normalized();
        self.save()?;
        Ok(self.snapshot())
    }

    pub fn record_project(&mut self, path: &Path) -> Result<ProjectRecord, StoreError> {
        self.database.app_state.record_project(path);
        let project = self.ensure_project_inner(path);
        self.save()?;
        Ok(project)
    }

    pub fn projects(&self) -> Vec<ProjectRecord> {
        let mut projects = self.database.projects.clone();
        projects.sort_by_key(|project| std::cmp::Reverse(project.updated_at));
        projects
    }

    pub fn create_agent_profile(
        &mut self,
        project_path: &Path,
        name: &str,
        role: &str,
        instructions: &str,
    ) -> Result<AgentProfile, StoreError> {
        let project = self.ensure_project_inner(project_path);
        let name = clean_required(name, "agent name", 80)?;
        let role = clean_required(role, "agent role", 80)?;
        let instructions = clean_optional(instructions, 20_000)?;
        if self
            .database
            .agents
            .iter()
            .any(|agent| agent.project_id == project.id && agent.name.eq_ignore_ascii_case(&name))
        {
            return Err(StoreError::Conflict(
                "agent names must be unique within a project".into(),
            ));
        }
        let now = timestamp();
        let agent = AgentProfile {
            id: self.take_agent_id(),
            project_id: project.id,
            name,
            role,
            instructions,
            created_at: now,
            updated_at: now,
        };
        self.database.agents.push(agent.clone());
        self.save()?;
        Ok(agent)
    }

    pub fn update_agent_profile(
        &mut self,
        id: u64,
        name: Option<&str>,
        role: Option<&str>,
        instructions: Option<&str>,
    ) -> Result<AgentProfile, StoreError> {
        let existing = self
            .database
            .agents
            .iter()
            .find(|agent| agent.id == id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("agent profile {id}")))?;
        let name = name
            .map(|value| clean_required(value, "agent name", 80))
            .transpose()?;
        let role = role
            .map(|value| clean_required(value, "agent role", 80))
            .transpose()?;
        let instructions = instructions
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        if let Some(name) = &name {
            if self.database.agents.iter().any(|agent| {
                agent.id != id
                    && agent.project_id == existing.project_id
                    && agent.name.eq_ignore_ascii_case(name)
            }) {
                return Err(StoreError::Conflict(
                    "agent names must be unique within a project".into(),
                ));
            }
        }
        let agent = self
            .database
            .agents
            .iter_mut()
            .find(|agent| agent.id == id)
            .expect("agent existence was checked");
        if let Some(name) = name {
            agent.name = name;
        }
        if let Some(role) = role {
            agent.role = role;
        }
        if let Some(instructions) = instructions {
            agent.instructions = instructions;
        }
        agent.updated_at = timestamp();
        let agent = agent.clone();
        self.save()?;
        Ok(agent)
    }

    pub fn delete_agent_profile(&mut self, id: u64) -> Result<(), StoreError> {
        if !self.database.agents.iter().any(|agent| agent.id == id) {
            return Err(StoreError::NotFound(format!("agent profile {id}")));
        }
        for column in &mut self.database.columns {
            if column.agent_id == Some(id) {
                column.agent_id = None;
            }
        }
        for task in &mut self.database.tasks {
            if task.assigned_agent_id == Some(id) {
                task.assigned_agent_id = None;
            }
        }
        self.database.agents.retain(|agent| agent.id != id);
        self.save()
    }

    pub fn create_workflow_column(
        &mut self,
        project_path: &Path,
        name: &str,
    ) -> Result<WorkflowColumn, StoreError> {
        let name = clean_required(name, "column name", 80)?;
        let project = self.ensure_project_inner(project_path);
        let key = unique_column_key(&name, &self.database.columns, project.id);
        let position = self
            .database
            .columns
            .iter()
            .filter(|column| column.project_id == project.id)
            .map(|column| column.position)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let column = WorkflowColumn {
            id: self.take_column_id(),
            project_id: project.id,
            key,
            name,
            position,
            terminal: false,
            agent_id: None,
        };
        self.database.columns.push(column.clone());
        self.save()?;
        Ok(column)
    }

    pub fn update_workflow_column(
        &mut self,
        id: u64,
        name: Option<&str>,
        position: Option<u32>,
        agent_id: Option<Option<u64>>,
    ) -> Result<WorkflowColumn, StoreError> {
        let name = name
            .map(|value| clean_required(value, "column name", 80))
            .transpose()?;
        let project_id = self
            .database
            .columns
            .iter()
            .find(|column| column.id == id)
            .map(|column| column.project_id)
            .ok_or_else(|| StoreError::NotFound(format!("workflow column {id}")))?;
        if let Some(Some(agent_id)) = agent_id {
            if !self
                .database
                .agents
                .iter()
                .any(|agent| agent.id == agent_id && agent.project_id == project_id)
            {
                return Err(StoreError::Validation(
                    "column agent must belong to the same project".into(),
                ));
            }
        }
        let column = self
            .database
            .columns
            .iter_mut()
            .find(|column| column.id == id)
            .expect("column existence was checked");
        if let Some(name) = name {
            column.name = name;
        }
        if let Some(agent_id) = agent_id {
            column.agent_id = agent_id;
        }
        let column_id = column.id;
        if let Some(position) = position {
            let mut ids: Vec<_> = self
                .columns_for_project(project_id)
                .into_iter()
                .map(|item| item.id)
                .collect();
            ids.retain(|candidate| *candidate != column_id);
            ids.insert((position as usize).min(ids.len()), column_id);
            for (position, candidate) in ids.into_iter().enumerate() {
                if let Some(item) = self
                    .database
                    .columns
                    .iter_mut()
                    .find(|item| item.id == candidate)
                {
                    item.position = position as u32;
                }
            }
        } else {
            self.normalize_column_positions(project_id);
        }
        self.save()?;
        self.database
            .columns
            .iter()
            .find(|item| item.id == column_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("workflow column {column_id}")))
    }

    pub fn delete_workflow_column(
        &mut self,
        id: u64,
        move_tasks_to: u64,
    ) -> Result<(), StoreError> {
        let source = self
            .database
            .columns
            .iter()
            .find(|column| column.id == id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("workflow column {id}")))?;
        let target = self
            .database
            .columns
            .iter()
            .find(|column| column.id == move_tasks_to && column.project_id == source.project_id)
            .cloned()
            .ok_or_else(|| {
                StoreError::Validation("choose another column in this project".into())
            })?;
        if source.id == target.id {
            return Err(StoreError::Validation(
                "tasks must move to a different column".into(),
            ));
        }
        if self.columns_for_project(source.project_id).len() <= 1 {
            return Err(StoreError::Conflict(
                "a board must retain at least one column".into(),
            ));
        }
        for task in self
            .database
            .tasks
            .iter_mut()
            .filter(|task| task.project_id == source.project_id && task.status == source.key)
        {
            task.status = target.key.clone();
            task.assigned_agent_id = target.agent_id;
            task.updated_at = timestamp();
        }
        for map in
            self.database.wayfinder_maps.iter_mut().filter(|map| {
                map.project_id == source.project_id && map.code_task_status == source.key
            })
        {
            map.code_task_status = target.key.clone();
            map.updated_at = timestamp();
        }
        if source.terminal {
            if let Some(target_column) = self
                .database
                .columns
                .iter_mut()
                .find(|column| column.id == target.id)
            {
                target_column.terminal = true;
            }
        }
        self.database.columns.retain(|column| column.id != id);
        self.normalize_column_positions(source.project_id);
        self.refresh_task_readiness();
        self.save()
    }

    pub fn assign_task(
        &mut self,
        task_id: u64,
        run_id: Option<u64>,
    ) -> Result<TaskRecord, StoreError> {
        let project_id = self.task(task_id)?.project_id;
        if let Some(run_id) = run_id {
            let run = self.run(run_id)?;
            if run.project_id != project_id {
                return Err(StoreError::Validation(
                    "task and agent conversation must belong to the same project".into(),
                ));
            }
        }
        let task = self
            .database
            .tasks
            .iter_mut()
            .find(|task| task.id == task_id)
            .expect("task existence was checked");
        task.assigned_run_id = run_id;
        task.updated_at = timestamp();
        let task = task.clone();
        if let Some(run_id) = run_id {
            let run = self.run_mut(run_id)?;
            run.background = true;
            run.updated_at = timestamp();
        }
        self.save()?;
        Ok(task)
    }

    pub fn project_data(&self, path: &Path) -> Result<ProjectData, StoreError> {
        let project = self.project_by_path(path)?;
        Ok(ProjectData {
            agents: self
                .database
                .agents
                .iter()
                .filter(|agent| agent.project_id == project.id)
                .cloned()
                .collect(),
            columns: self.columns_for_project(project.id),
            tasks: self
                .database
                .tasks
                .iter()
                .filter(|task| task.project_id == project.id)
                .cloned()
                .collect(),
            todos: self
                .database
                .todos
                .iter()
                .filter(|todo| todo.project_id == project.id)
                .cloned()
                .collect(),
            runs: self.runs_for_project_id(project.id),
            approvals: self
                .database
                .approvals
                .iter()
                .filter(|approval| {
                    self.database
                        .runs
                        .iter()
                        .any(|run| run.id == approval.run_id && run.project_id == project.id)
                })
                .cloned()
                .collect(),
            project,
        })
    }

    pub fn create_task(
        &mut self,
        project_path: &Path,
        title: &str,
        detail: &str,
        outcome: &str,
        status: &str,
        depends_on: Vec<u64>,
    ) -> Result<TaskRecord, StoreError> {
        let title = clean_required(title, "task title", 500)?;
        let detail = clean_optional(detail, 20_000)?;
        let outcome = clean_optional(outcome, 20_000)?;
        let project = self.ensure_project_inner(project_path);
        let status = self.task_status_for_project(project.id, status)?;
        self.validate_dependencies(project.id, None, &depends_on)?;
        let assigned_agent_id = self
            .database
            .columns
            .iter()
            .find(|column| column.project_id == project.id && column.key == status)
            .and_then(|column| column.agent_id);
        let now = timestamp();
        let task = TaskRecord {
            id: self.take_task_id(),
            project_id: project.id,
            title,
            detail,
            outcome,
            status,
            depends_on,
            ready: false,
            assigned_run_id: None,
            assigned_agent_id,
            created_at: now,
            updated_at: now,
        };
        self.database.tasks.push(task.clone());
        self.validate_task_graph(project.id)?;
        self.refresh_task_readiness();
        let task = self.task(task.id)?;
        self.save()?;
        Ok(task)
    }

    pub fn update_task(
        &mut self,
        id: u64,
        title: Option<&str>,
        detail: Option<&str>,
        outcome: Option<&str>,
        status: Option<&str>,
        depends_on: Option<Vec<u64>>,
    ) -> Result<TaskRecord, StoreError> {
        let title = title
            .map(|value| clean_required(value, "task title", 500))
            .transpose()?;
        let detail = detail
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let outcome = outcome
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let project_id = self.task(id)?.project_id;
        let status = status
            .map(|value| self.task_status_for_project(project_id, value))
            .transpose()?;
        if let Some(dependencies) = &depends_on {
            self.validate_dependencies(project_id, Some(id), dependencies)?;
        }
        let requires_dependencies = status.as_deref().is_some_and(|value| {
            self.database.columns.iter().any(|column| {
                column.project_id == project_id
                    && column.key == value
                    && (column.position >= 2 || column.terminal)
            })
        });
        if requires_dependencies {
            let dependencies = depends_on.as_deref().unwrap_or_else(|| {
                self.database
                    .tasks
                    .iter()
                    .find(|task| task.id == id)
                    .map(|task| task.depends_on.as_slice())
                    .unwrap_or(&[])
            });
            let terminal_keys: HashSet<_> = self
                .database
                .columns
                .iter()
                .filter(|column| column.project_id == project_id && column.terminal)
                .map(|column| column.key.clone())
                .collect();
            let completed: HashSet<_> = self
                .database
                .tasks
                .iter()
                .filter(|task| terminal_keys.contains(&task.status))
                .map(|task| task.id)
                .collect();
            if !dependencies
                .iter()
                .all(|dependency| completed.contains(dependency))
            {
                return Err(StoreError::Conflict(
                    "complete task dependencies before advancing this task".into(),
                ));
            }
        }
        let original_dependencies = self.task(id)?.depends_on;
        let policy_agent_id = status.as_deref().and_then(|next_status| {
            self.database
                .columns
                .iter()
                .find(|column| column.project_id == project_id && column.key == next_status)
                .and_then(|column| column.agent_id)
        });
        let status_is_changing = status
            .as_deref()
            .is_some_and(|next_status| self.task(id).is_ok_and(|task| task.status != next_status));
        {
            let task = self
                .database
                .tasks
                .iter_mut()
                .find(|task| task.id == id)
                .expect("task existence was checked");
            if let Some(title) = title {
                task.title = title;
            }
            if let Some(detail) = detail {
                task.detail = detail;
            }
            if let Some(outcome) = outcome {
                task.outcome = outcome;
            }
            if let Some(status) = status {
                task.status = status;
                if status_is_changing {
                    task.assigned_agent_id = policy_agent_id;
                }
            }
            if let Some(dependencies) = depends_on {
                task.depends_on = dependencies;
            }
            task.updated_at = timestamp();
        }
        if let Err(error) = self.validate_task_graph(project_id) {
            self.database
                .tasks
                .iter_mut()
                .find(|task| task.id == id)
                .expect("task existence was checked")
                .depends_on = original_dependencies;
            return Err(error);
        }
        self.refresh_task_readiness();
        let task = self.task(id)?;
        self.save()?;
        Ok(task)
    }

    pub fn create_todo(
        &mut self,
        project_path: &Path,
        title: &str,
        owner: &str,
        status: &str,
    ) -> Result<TodoRecord, StoreError> {
        let title = clean_required(title, "todo title", 500)?;
        let owner = clean_required(owner, "todo owner", 200)?;
        let status = todo_status(status)?;
        let project = self.ensure_project_inner(project_path);
        let now = timestamp();
        let todo = TodoRecord {
            id: self.take_todo_id(),
            project_id: project.id,
            title,
            owner,
            status,
            assigned_run_id: None,
            created_at: now,
            updated_at: now,
        };
        self.database.todos.push(todo.clone());
        self.save()?;
        Ok(todo)
    }

    pub fn update_todo(
        &mut self,
        id: u64,
        title: Option<&str>,
        owner: Option<&str>,
        status: Option<&str>,
    ) -> Result<TodoRecord, StoreError> {
        let title = title
            .map(|value| clean_required(value, "todo title", 500))
            .transpose()?;
        let owner = owner
            .map(|value| clean_required(value, "todo owner", 200))
            .transpose()?;
        let status = status.map(todo_status).transpose()?;
        let todo = self
            .database
            .todos
            .iter_mut()
            .find(|todo| todo.id == id)
            .ok_or_else(|| StoreError::NotFound(format!("todo {id}")))?;
        if let Some(title) = title {
            todo.title = title;
        }
        if let Some(owner) = owner {
            todo.owner = owner;
        }
        if let Some(status) = status {
            todo.status = status;
        }
        todo.updated_at = timestamp();
        let todo = todo.clone();
        self.save()?;
        Ok(todo)
    }

    pub fn assign_todo(
        &mut self,
        todo_id: u64,
        run_id: Option<u64>,
    ) -> Result<TodoRecord, StoreError> {
        let project_id = self
            .database
            .todos
            .iter()
            .find(|todo| todo.id == todo_id)
            .map(|todo| todo.project_id)
            .ok_or_else(|| StoreError::NotFound(format!("todo {todo_id}")))?;
        if let Some(run_id) = run_id {
            if self.run(run_id)?.project_id != project_id {
                return Err(StoreError::Validation(
                    "todo and agent conversation must belong to the same project".into(),
                ));
            }
        }
        let todo = self
            .database
            .todos
            .iter_mut()
            .find(|todo| todo.id == todo_id)
            .expect("todo existence was checked");
        todo.assigned_run_id = run_id;
        todo.updated_at = timestamp();
        let todo = todo.clone();
        self.save()?;
        Ok(todo)
    }

    pub fn allocate_run(
        &mut self,
        project_path: &Path,
        worktree_path: &Path,
        base_commit: String,
        prompt: String,
        mode: String,
    ) -> Result<RunRecord, StoreError> {
        let project = self.ensure_project_inner(project_path);
        let now = timestamp();
        let title = conversation_title(&prompt);
        let run = RunRecord {
            id: self.take_run_id(),
            project_id: project.id,
            source_project_path: project.path,
            worktree_path: worktree_path.to_string_lossy().into_owned(),
            base_commit,
            prompt,
            title,
            pinned: false,
            archived_at: None,
            background: false,
            mode,
            pid: None,
            running: false,
            outcome: "queued".into(),
            lifecycle: "retained".into(),
            stdout: String::new(),
            stderr: String::new(),
            integrated_commit: None,
            created_at: now,
            updated_at: now,
            finished_at: None,
        };
        self.database.runs.push(run.clone());
        self.push_system_event(
            run.id,
            "run/created",
            serde_json::json!({"worktreePath": run.worktree_path, "baseCommit": run.base_commit}),
        );
        self.save()?;
        Ok(run)
    }

    pub fn update_conversation(
        &mut self,
        id: u64,
        title: Option<&str>,
        pinned: Option<bool>,
        archived: Option<bool>,
    ) -> Result<RunRecord, StoreError> {
        let title = title
            .map(|value| clean_required(value, "conversation title", 160))
            .transpose()?;
        let now = timestamp();
        let run = self.run_mut(id)?;
        if archived == Some(true) && run.running {
            return Err(StoreError::Conflict(
                "end the conversation before archiving it".into(),
            ));
        }
        if let Some(title) = title {
            run.title = title;
        }
        if let Some(pinned) = pinned {
            run.pinned = pinned;
        }
        if let Some(archived) = archived {
            run.archived_at = archived.then_some(now);
            if archived {
                run.pinned = false;
            }
        }
        run.updated_at = now;
        let run = run.clone();
        self.save()?;
        Ok(run)
    }

    pub fn mark_run_started(&mut self, id: u64, pid: Option<u32>) -> Result<RunRecord, StoreError> {
        let run = self.run_mut(id)?;
        run.pid = pid;
        run.running = true;
        run.outcome = "running".into();
        run.finished_at = None;
        run.updated_at = timestamp();
        let run = run.clone();
        self.push_system_event(id, "run/started", serde_json::json!({"pid": pid}));
        self.save()?;
        Ok(run)
    }

    pub fn mark_run_launch_failed(
        &mut self,
        id: u64,
        detail: &str,
    ) -> Result<RunRecord, StoreError> {
        let now = timestamp();
        let clean_detail = clean_optional(detail, 20_000)?;
        let run = self.run_mut(id)?;
        run.running = false;
        run.outcome = "failed".into();
        run.stderr = clean_detail;
        run.updated_at = now;
        run.finished_at = Some(now);
        let run = run.clone();
        self.push_system_event(
            id,
            "run/launch_failed",
            serde_json::json!({"error": detail}),
        );
        self.save()?;
        Ok(run)
    }

    pub fn sync_run(
        &mut self,
        id: u64,
        running: bool,
        outcome: &str,
        pid: Option<u32>,
        stdout: &str,
        stderr: &str,
    ) -> Result<RunRecord, StoreError> {
        let now = timestamp();
        let run = self.run_mut(id)?;
        let changed = run.running != running
            || run.outcome != outcome
            || run.pid != pid
            || run.stdout != stdout
            || run.stderr != stderr;
        if !changed {
            return Ok(run.clone());
        }
        let outcome_changed = run.outcome != outcome;
        run.running = running;
        run.outcome = outcome.to_owned();
        run.pid = pid;
        run.stdout = stdout.to_owned();
        run.stderr = stderr.to_owned();
        run.updated_at = now;
        if !running && outcome != "running" {
            run.finished_at.get_or_insert(now);
        }
        let run = run.clone();
        if !running && outcome != "running" {
            for approval in &mut self.database.approvals {
                if approval.run_id == id && approval.status == "pending" {
                    approval.status = "expired".into();
                    approval.decided_at = Some(now);
                }
            }
        }
        if outcome_changed {
            self.push_system_event(
                id,
                "run/outcome",
                serde_json::json!({"outcome": outcome, "running": running}),
            );
        }
        self.save()?;
        Ok(run)
    }

    pub fn append_engine_events(&mut self, events: &[EngineEvent]) -> Result<(), StoreError> {
        let mut changed = false;
        for event in events {
            if self.database.events.iter().any(|stored| {
                stored.run_id == event.run_id
                    && stored.protocol_sequence == event.sequence
                    && event.sequence != 0
                    && stored.created_at == event.created_at
                    && stored.kind == event.kind
                    && stored.raw == event.raw
            }) {
                continue;
            }
            let id = self.take_event_id();
            self.database.events.push(RunEventRecord {
                id,
                run_id: event.run_id,
                protocol_sequence: event.sequence,
                kind: event.kind.clone(),
                payload: event.payload.clone(),
                raw: event.raw.clone(),
                created_at: event.created_at,
            });
            if matches!(
                event.kind.as_str(),
                "file/edit" | "file/create" | "command/approval"
            ) {
                if let (Some(edit_id), Some(path), Some(content)) = (
                    event
                        .payload
                        .get("editId")
                        .and_then(serde_json::Value::as_str),
                    event
                        .payload
                        .get("path")
                        .and_then(serde_json::Value::as_str),
                    event
                        .payload
                        .get("content")
                        .and_then(serde_json::Value::as_str),
                ) {
                    if !self.database.approvals.iter().any(|approval| {
                        approval.run_id == event.run_id && approval.edit_id == edit_id
                    }) {
                        let approval_id = self.take_approval_id();
                        self.database.approvals.push(EditApprovalRecord {
                            id: approval_id,
                            run_id: event.run_id,
                            edit_id: edit_id.to_owned(),
                            path: path.to_owned(),
                            content: content.to_owned(),
                            edit_type: event
                                .payload
                                .get("type")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or(if event.kind == "file/create" {
                                    "create"
                                } else {
                                    "modify"
                                })
                                .to_owned(),
                            approval_kind: event
                                .payload
                                .get("approvalKind")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("fileChange")
                                .to_owned(),
                            status: "pending".into(),
                            requested_at: event.created_at,
                            decided_at: None,
                        });
                    }
                }
            }
            let run_event_count = self
                .database
                .events
                .iter()
                .filter(|stored| stored.run_id == event.run_id)
                .count();
            if run_event_count > 5_000 {
                let mut to_remove = run_event_count - 5_000;
                self.database.events.retain(|stored| {
                    if stored.run_id == event.run_id && to_remove > 0 {
                        to_remove -= 1;
                        false
                    } else {
                        true
                    }
                });
            }
            changed = true;
        }
        if changed {
            self.save()?;
        }
        Ok(())
    }

    pub fn pending_edit_approval(
        &self,
        run_id: u64,
        edit_id: &str,
    ) -> Result<EditApprovalRecord, StoreError> {
        self.run(run_id)?;
        self.database
            .approvals
            .iter()
            .find(|approval| {
                approval.run_id == run_id
                    && approval.edit_id == edit_id
                    && approval.status == "pending"
            })
            .cloned()
            .ok_or_else(|| StoreError::Conflict(format!("edit approval {edit_id} is not pending")))
    }

    pub fn resolve_edit_approval(
        &mut self,
        run_id: u64,
        edit_id: &str,
        accepted: bool,
    ) -> Result<EditApprovalRecord, StoreError> {
        self.pending_edit_approval(run_id, edit_id)?;
        let now = timestamp();
        let approval = self
            .database
            .approvals
            .iter_mut()
            .find(|approval| approval.run_id == run_id && approval.edit_id == edit_id)
            .expect("approval existence was checked");
        approval.status = if accepted { "approved" } else { "denied" }.into();
        approval.decided_at = Some(now);
        let approval = approval.clone();
        self.push_system_event(
            run_id,
            "approval/decision",
            serde_json::json!({"editId": edit_id, "accepted": accepted, "path": approval.path, "approvalKind": approval.approval_kind}),
        );
        self.save()?;
        Ok(approval)
    }

    pub fn append_user_message(
        &mut self,
        run_id: u64,
        message: &str,
        attachments: serde_json::Value,
    ) -> Result<(), StoreError> {
        self.run(run_id)?;
        let message = clean_required(message, "message", 100_000)?;
        self.push_system_event(
            run_id,
            "chat/user",
            serde_json::json!({"text": message, "attachments": attachments}),
        );
        self.save()
    }

    pub fn backend_thread_id(&self, run_id: u64) -> Result<Option<String>, StoreError> {
        let run = self.run(run_id)?;
        let recorded = self
            .database
            .events
            .iter()
            .rev()
            .find(|event| event.run_id == run_id && event.kind == "engine/thread")
            .and_then(|event| event.payload.get("threadId"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if recorded.is_some() {
            return Ok(recorded);
        }
        Ok(run.stdout.lines().rev().find_map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()?
                .pointer("/result/thread/id")?
                .as_str()
                .map(str::to_owned)
        }))
    }

    pub fn codex_harness_tools_enabled(&self, run_id: u64) -> Result<bool, StoreError> {
        self.run(run_id)?;
        Ok(self
            .database
            .events
            .iter()
            .any(|event| event.run_id == run_id && event.kind == "engine/harness_tools"))
    }

    pub fn conversation_context(&self, run_id: u64) -> Result<String, StoreError> {
        let run = self.run(run_id)?;
        let mut lines = vec![format!("User: {}", run.prompt)];
        for event in self
            .database
            .events
            .iter()
            .filter(|event| event.run_id == run_id)
        {
            match event.kind.as_str() {
                "chat/user" => {
                    if let Some(text) = event
                        .payload
                        .get("text")
                        .and_then(serde_json::Value::as_str)
                    {
                        lines.push(format!("User: {text}"));
                    }
                }
                "stream/text"
                    if event
                        .payload
                        .get("final")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true) =>
                {
                    if let Some(text) = event
                        .payload
                        .get("text")
                        .and_then(serde_json::Value::as_str)
                    {
                        lines.push(format!("Rubyn: {text}"));
                    }
                }
                _ => {}
            }
        }
        Ok(lines
            .into_iter()
            .rev()
            .take(30)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n\n"))
    }

    pub fn apply_harness_tool_events(&mut self, events: &[EngineEvent]) -> Result<(), StoreError> {
        for event in events.iter().filter(|event| event.kind == "tool/use") {
            let tool = event.payload.get("tool").and_then(|value| value.as_str());
            if !matches!(tool, Some("harness_task" | "wayfinder")) {
                continue;
            }
            let Some(args) = event
                .payload
                .get("args")
                .and_then(|value| value.as_object())
            else {
                continue;
            };
            let request_id = event.payload.get("requestId").cloned().unwrap_or_default();
            if self.database.events.iter().any(|stored| {
                stored.run_id == event.run_id
                    && matches!(
                        stored.kind.as_str(),
                        "harness/control_applied" | "harness/control_rejected"
                    )
                    && stored.payload.get("requestId") == Some(&request_id)
            }) {
                continue;
            }
            let run = self.run(event.run_id)?;
            let project_path = PathBuf::from(&run.source_project_path);
            let project_id = self
                .database
                .projects
                .iter()
                .find(|project| project.path == run.source_project_path)
                .map(|project| project.id);
            let kind = if tool == Some("wayfinder") {
                "wayfinder"
            } else {
                args.get("kind")
                    .and_then(|value| value.as_str())
                    .unwrap_or("task")
            };
            let raw_action = args
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("list");
            let action = match (tool, raw_action) {
                (Some("wayfinder"), "list_maps") => "list",
                (Some("wayfinder"), "get_map") => "get",
                _ => raw_action,
            };
            let result = match (kind, action) {
                ("wayfinder", "create_map" | "import_map") => {
                    let title = args
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let idea = args
                        .get("idea")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or(title);
                    let imported = action == "import_map";
                    self.create_wayfinder_map(
                        &project_path,
                        idea,
                        args.get("code_task_status")
                            .and_then(|value| value.as_str()),
                    )
                    .and_then(|created| {
                        let updated = self.update_wayfinder_map(
                            created.map.id,
                            (!title.trim().is_empty()).then_some(title),
                            args.get("destination").and_then(|value| value.as_str()),
                            args.get("notes").and_then(|value| value.as_str()),
                        )?;
                        if imported {
                            if let Some(bootstrap) = updated
                                .tickets
                                .iter()
                                .find(|ticket| ticket.title == "Name the destination")
                            {
                                self.retire_wayfinder_ticket(bootstrap.id)?;
                            }
                        }
                        self.wayfinder_map_data(created.map.id)
                    })
                    .map(|_| ())
                }
                ("wayfinder", "update_map") => {
                    let map_id = json_u64(args.get("map_id")).ok().or_else(|| {
                        let title = args.get("map_id")?.as_str()?;
                        self.database
                            .wayfinder_maps
                            .iter()
                            .find(|map| Some(map.project_id) == project_id && map.title == title)
                            .map(|map| map.id)
                    });
                    map_id
                        .ok_or_else(|| StoreError::Validation("Wayfinder map was not found".into()))
                        .and_then(|map_id| {
                            self.update_wayfinder_map(
                                map_id,
                                args.get("title").and_then(|value| value.as_str()),
                                args.get("destination").and_then(|value| value.as_str()),
                                args.get("notes").and_then(|value| value.as_str()),
                            )
                        })
                        .map(|_| ())
                }
                ("wayfinder", "create_node") => {
                    let map_id = json_u64(args.get("map_id")).ok().or_else(|| {
                        let title = args.get("map_id")?.as_str()?;
                        self.database
                            .wayfinder_maps
                            .iter()
                            .find(|map| Some(map.project_id) == project_id && map.title == title)
                            .map(|map| map.id)
                    });
                    map_id
                        .ok_or_else(|| StoreError::Validation("Wayfinder map was not found".into()))
                        .and_then(|map_id| {
                            let request = CreateWayfinderTicketRequest {
                                map_id,
                                title: args
                                    .get("title")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or_default()
                                    .to_owned(),
                                question: args
                                    .get("question")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or_default()
                                    .to_owned(),
                                information: args
                                    .get("description")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or_default()
                                    .to_owned(),
                                outcome: args
                                    .get("outcome")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or_default()
                                    .to_owned(),
                                ticket_type: args
                                    .get("node_type")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or("grill")
                                    .to_owned(),
                                depends_on: args
                                    .get("blocked_by")
                                    .and_then(|value| value.as_array())
                                    .map(|items| {
                                        items
                                            .iter()
                                            .filter_map(|value| {
                                                value.as_u64().or_else(|| {
                                                    let reference = value.as_str()?;
                                                    reference.parse().ok().or_else(|| {
                                                        self.database
                                                            .wayfinder_tickets
                                                            .iter()
                                                            .find(|ticket| {
                                                                ticket.map_id == map_id
                                                                    && ticket.title == reference
                                                            })
                                                            .map(|ticket| ticket.id)
                                                    })
                                                })
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                model_role: args
                                    .get("model_role")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_owned),
                                effort: args
                                    .get("effort")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_owned),
                                budget_cents: None,
                            };
                            self.create_wayfinder_ticket(&request, "rubyn")
                        })
                        .map(|_| ())
                }
                ("wayfinder", "resolve_node") => {
                    let id = json_u64(args.get("node_id").or_else(|| args.get("task_id")));
                    id.and_then(|id| {
                        self.resolve_wayfinder_ticket(
                            id,
                            args.get("resolution")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default(),
                            &[],
                            &[],
                        )
                    })
                    .map(|_| ())
                }
                ("wayfinder", "retire_node") => {
                    let id = json_u64(args.get("node_id").or_else(|| args.get("task_id")));
                    id.and_then(|id| self.retire_wayfinder_ticket(id))
                        .map(|_| ())
                }
                ("wayfinder", "list" | "get") => Ok(()),
                ("task", "create") => {
                    let title = args
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let detail = args
                        .get("description")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let outcome = args
                        .get("outcome")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let dependencies = args
                        .get("blocked_by")
                        .and_then(|value| value.as_array())
                        .map(|items| items.iter().filter_map(serde_json::Value::as_u64).collect())
                        .unwrap_or_default();
                    self.create_task(
                        &project_path,
                        title,
                        detail,
                        outcome,
                        "queued",
                        dependencies,
                    )
                    .map(|_| ())
                }
                ("task", "update" | "complete") => {
                    let id = json_u64(args.get("task_id"));
                    let status = if action == "complete" {
                        Some("done")
                    } else {
                        args.get("status")
                            .and_then(|value| value.as_str())
                            .map(agent_task_status)
                    };
                    id.and_then(|id| {
                        self.update_task(
                            id,
                            args.get("title").and_then(|value| value.as_str()),
                            args.get("description").and_then(|value| value.as_str()),
                            args.get("outcome").and_then(|value| value.as_str()),
                            status,
                            None,
                        )
                    })
                    .map(|_| ())
                }
                ("todo", "create") => {
                    let title = args
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    self.create_todo(&project_path, title, "Rubyn", "queued")
                        .map(|_| ())
                }
                ("todo", "update" | "complete") => {
                    let id = json_u64(args.get("task_id"));
                    let status = if action == "complete" {
                        Some("done")
                    } else {
                        args.get("status")
                            .and_then(|value| value.as_str())
                            .map(agent_todo_status)
                    };
                    id.and_then(|id| {
                        self.update_todo(
                            id,
                            args.get("title").and_then(|value| value.as_str()),
                            Some("Rubyn"),
                            status,
                        )
                    })
                    .map(|_| ())
                }
                (_, "list" | "get") => Ok(()),
                _ => Err(StoreError::Validation(format!(
                    "unsupported Harness operation {kind}/{action}"
                ))),
            };
            match result {
                Ok(()) => self.push_system_event(
                    event.run_id,
                    "harness/control_applied",
                    serde_json::json!({"kind": kind, "action": action, "requestId": request_id}),
                ),
                Err(error) => self.push_system_event(
                    event.run_id,
                    "harness/control_rejected",
                    serde_json::json!({"kind": kind, "action": action, "requestId": request_id, "error": error.to_string()}),
                ),
            }
        }
        self.refresh_control_snapshots()?;
        self.save()
    }

    pub fn refresh_control_snapshots(&self) -> Result<(), StoreError> {
        for run in self.database.runs.iter().filter(|run| run.running) {
            self.write_control_snapshot(run)?;
        }
        Ok(())
    }

    pub fn refresh_run_control_snapshot(&self, run_id: u64) -> Result<(), StoreError> {
        let run = self.run(run_id)?;
        self.write_control_snapshot(&run)
    }

    fn write_control_snapshot(&self, run: &RunRecord) -> Result<(), StoreError> {
        let data = self.project_data(Path::new(&run.source_project_path))?;
        let wayfinder_maps = self.wayfinder_maps(Path::new(&run.source_project_path))?;
        let wayfinder: Vec<_> = wayfinder_maps
            .iter()
            .filter_map(|map| self.wayfinder_map_data(map.id).ok())
            .collect();
        let path = Path::new(&run.worktree_path)
            .parent()
            .ok_or_else(|| StoreError::Validation("run worktree has no control directory".into()))?
            .join("harness-control.json");
        let temporary = path.with_extension("json.tmp");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(&serde_json::json!({
                "version": 1,
                "project": data.project,
                "tasks": data.tasks,
                "todos": data.todos,
                "wayfinder": wayfinder,
            }))?,
        )?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn wayfinder_maps(&self, project_path: &Path) -> Result<Vec<WayfinderMap>, StoreError> {
        let project = self.project_by_path(project_path)?;
        let mut maps: Vec<_> = self
            .database
            .wayfinder_maps
            .iter()
            .filter(|map| map.project_id == project.id)
            .cloned()
            .collect();
        maps.sort_by_key(|map| std::cmp::Reverse(map.updated_at));
        Ok(maps)
    }

    pub fn wayfinder_map_data(&self, map_id: u64) -> Result<WayfinderMapData, StoreError> {
        let map = self.wayfinder_map(map_id)?;
        let mut tickets: Vec<_> = self
            .database
            .wayfinder_tickets
            .iter()
            .filter(|ticket| ticket.map_id == map_id)
            .cloned()
            .collect();
        tickets.sort_by_key(|ticket| ticket.id);
        let ticket_ids: HashSet<_> = tickets.iter().map(|ticket| ticket.id).collect();
        let mut questions: Vec<_> = self
            .database
            .wayfinder_questions
            .iter()
            .filter(|question| ticket_ids.contains(&question.ticket_id))
            .cloned()
            .collect();
        questions.sort_by_key(|question| question.id);
        let mut events: Vec<_> = self
            .database
            .wayfinder_events
            .iter()
            .filter(|event| event.map_id == map_id)
            .cloned()
            .collect();
        events.sort_by_key(|event| event.id);
        Ok(WayfinderMapData {
            map,
            tickets,
            questions,
            events,
        })
    }

    pub fn create_wayfinder_map(
        &mut self,
        project_path: &Path,
        idea: &str,
        code_task_status: Option<&str>,
    ) -> Result<WayfinderMapData, StoreError> {
        let idea = clean_required(idea, "Wayfinder idea", 20_000)?;
        let project = self.ensure_project_inner(project_path);
        let code_task_status = match code_task_status {
            Some(status) => self.task_status_for_project(project.id, status)?,
            None => {
                self.columns_for_project(project.id)
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        StoreError::Validation("the project has no workflow columns".into())
                    })?
                    .key
            }
        };
        let now = timestamp();
        let title = idea.lines().next().unwrap_or("New Wayfinder map");
        let title = if title.chars().count() > 90 {
            format!("{}…", title.chars().take(89).collect::<String>())
        } else {
            title.to_owned()
        };
        let map = WayfinderMap {
            id: self.take_wayfinder_map_id(),
            project_id: project.id,
            title,
            idea,
            destination: String::new(),
            notes: String::new(),
            code_task_status,
            status: "draft".into(),
            created_at: now,
            updated_at: now,
        };
        self.database.wayfinder_maps.push(map.clone());
        let ticket = WayfinderTicket {
            id: self.take_wayfinder_ticket_id(),
            map_id: map.id,
            title: "Name the destination".into(),
            question: "What does reaching the end of this map look like?".into(),
            information: map.idea.clone(),
            outcome: "A destination and first decision frontier approved by the user.".into(),
            ticket_type: "grill".into(),
            status: "frontier".into(),
            depends_on: Vec::new(),
            linked_task_id: None,
            linked_run_id: None,
            brief_version: 1,
            resolution: String::new(),
            result_note: String::new(),
            model_role: "sol".into(),
            effort: "high".into(),
            budget_cents: None,
            created_at: now,
            updated_at: now,
        };
        self.database.wayfinder_tickets.push(ticket.clone());
        self.push_wayfinder_event(
            map.id,
            Some(ticket.id),
            "map/created",
            "user",
            serde_json::json!({"idea": map.idea}),
        );
        self.save()?;
        self.wayfinder_map_data(map.id)
    }

    pub fn update_wayfinder_map(
        &mut self,
        map_id: u64,
        title: Option<&str>,
        destination: Option<&str>,
        notes: Option<&str>,
    ) -> Result<WayfinderMapData, StoreError> {
        let title = title
            .map(|value| clean_required(value, "map title", 500))
            .transpose()?;
        let destination = destination
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let notes = notes
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let map = self.wayfinder_map_mut(map_id)?;
        if let Some(title) = title {
            map.title = title;
        }
        if let Some(destination) = destination {
            map.destination = destination;
        }
        if let Some(notes) = notes {
            map.notes = notes;
        }
        map.updated_at = timestamp();
        self.push_wayfinder_event(map_id, None, "map/updated", "user", serde_json::json!({}));
        self.save()?;
        self.wayfinder_map_data(map_id)
    }

    pub fn create_wayfinder_ticket(
        &mut self,
        request: &CreateWayfinderTicketRequest,
        actor: &str,
    ) -> Result<WayfinderTicket, StoreError> {
        let map = self.wayfinder_map(request.map_id)?;
        if map.status == "archived" {
            return Err(StoreError::Conflict(
                "archived maps cannot accept tickets".into(),
            ));
        }
        let ticket_type = wayfinder_ticket_type(&request.ticket_type)?;
        self.validate_wayfinder_dependencies(request.map_id, None, &request.depends_on)?;
        let now = timestamp();
        let ticket = WayfinderTicket {
            id: self.take_wayfinder_ticket_id(),
            map_id: request.map_id,
            title: clean_required(&request.title, "ticket title", 500)?,
            question: clean_optional(&request.question, 20_000)?,
            information: clean_optional(&request.information, 20_000)?,
            outcome: clean_optional(&request.outcome, 20_000)?,
            ticket_type: ticket_type.into(),
            status: "blocked".into(),
            depends_on: request.depends_on.clone(),
            linked_task_id: None,
            linked_run_id: None,
            brief_version: 1,
            resolution: String::new(),
            result_note: String::new(),
            model_role: clean_optional(
                request
                    .model_role
                    .as_deref()
                    .unwrap_or(default_wayfinder_model(ticket_type)),
                100,
            )?,
            effort: clean_optional(request.effort.as_deref().unwrap_or("medium"), 100)?,
            budget_cents: request.budget_cents,
            created_at: now,
            updated_at: now,
        };
        self.database.wayfinder_tickets.push(ticket.clone());
        if let Err(error) = self.validate_wayfinder_graph(request.map_id) {
            self.database
                .wayfinder_tickets
                .retain(|candidate| candidate.id != ticket.id);
            return Err(error);
        }
        self.refresh_wayfinder_frontier(request.map_id)?;
        self.push_wayfinder_event(
            request.map_id,
            Some(ticket.id),
            "ticket/created",
            actor,
            serde_json::json!({"type": ticket_type}),
        );
        self.materialize_unblocked_code_tickets(request.map_id)?;
        self.save()?;
        self.wayfinder_ticket(ticket.id)
    }

    pub fn update_wayfinder_ticket(
        &mut self,
        request: &UpdateWayfinderTicketRequest,
    ) -> Result<WayfinderTicket, StoreError> {
        let id = request.id;
        let original = self.wayfinder_ticket(id)?;
        if let Some(dependencies) = &request.depends_on {
            self.validate_wayfinder_dependencies(original.map_id, Some(id), dependencies)?;
        }
        let title = request
            .title
            .as_deref()
            .map(|value| clean_required(value, "ticket title", 500))
            .transpose()?;
        let question = request
            .question
            .as_deref()
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let information = request
            .information
            .as_deref()
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let outcome = request
            .outcome
            .as_deref()
            .map(|value| clean_optional(value, 20_000))
            .transpose()?;
        let model_role = request
            .model_role
            .as_deref()
            .map(|value| clean_required(value, "model role", 100))
            .transpose()?;
        let effort = request
            .effort
            .as_deref()
            .map(|value| clean_required(value, "effort", 100))
            .transpose()?;
        let ticket = self.wayfinder_ticket_mut(id)?;
        if let Some(value) = title {
            ticket.title = value;
        }
        if let Some(value) = question {
            ticket.question = value;
        }
        if let Some(value) = information {
            ticket.information = value;
        }
        if let Some(value) = outcome {
            ticket.outcome = value;
        }
        if let Some(value) = request.depends_on.clone() {
            ticket.depends_on = value;
        }
        if let Some(value) = model_role {
            ticket.model_role = value;
        }
        if let Some(value) = effort {
            ticket.effort = value;
        }
        if let Some(value) = request.budget_cents {
            ticket.budget_cents = value;
        }
        ticket.brief_version = ticket.brief_version.saturating_add(1);
        ticket.updated_at = timestamp();
        if let Err(error) = self.validate_wayfinder_graph(original.map_id) {
            *self.wayfinder_ticket_mut(id)? = original;
            return Err(error);
        }
        self.refresh_wayfinder_frontier(original.map_id)?;
        self.push_wayfinder_event(
            original.map_id,
            Some(id),
            "ticket/updated",
            "user",
            serde_json::json!({"briefVersion": self.wayfinder_ticket(id)?.brief_version}),
        );
        self.materialize_unblocked_code_tickets(original.map_id)?;
        self.save()?;
        self.wayfinder_ticket(id)
    }

    pub fn submit_wayfinder_answers(
        &mut self,
        ticket_id: u64,
        answers: &[WayfinderAnswer],
    ) -> Result<WayfinderMapData, StoreError> {
        let ticket = self.wayfinder_ticket(ticket_id)?;
        for answer in answers {
            let question = self
                .database
                .wayfinder_questions
                .iter_mut()
                .find(|question| {
                    question.id == answer.question_id && question.ticket_id == ticket_id
                })
                .ok_or_else(|| StoreError::NotFound(format!("question {}", answer.question_id)))?;
            if question.answered_at.is_some() {
                if question.answers != answer.answers
                    || question.custom_answer != answer.custom_answer
                {
                    return Err(StoreError::Conflict(format!(
                        "question {} was already answered",
                        answer.question_id
                    )));
                }
                continue;
            }
            if question.cardinality == "single" && answer.answers.len() > 1 {
                return Err(StoreError::Validation(format!(
                    "question {} accepts one answer",
                    answer.question_id
                )));
            }
            if answer.answers.is_empty() && answer.custom_answer.trim().is_empty() {
                return Err(StoreError::Validation(format!(
                    "question {} requires an answer",
                    answer.question_id
                )));
            }
            question.answers = answer.answers.clone();
            question.custom_answer = clean_optional(&answer.custom_answer, 20_000)?;
            question.answered_at = Some(timestamp());
        }
        let destination_answer = self
            .database
            .wayfinder_questions
            .iter()
            .find(|question| question.ticket_id == ticket_id && question.round == 1)
            .and_then(question_answer_text);
        if let Some(destination) = destination_answer {
            let map = self.wayfinder_map_mut(ticket.map_id)?;
            if map.destination.is_empty() {
                map.destination = destination;
                map.updated_at = timestamp();
            }
        }
        self.push_wayfinder_event(
            ticket.map_id,
            Some(ticket_id),
            "grill/answers_submitted",
            "user",
            serde_json::json!({"questionIds": answers.iter().map(|answer| answer.question_id).collect::<Vec<_>>() }),
        );
        self.save()?;
        self.wayfinder_map_data(ticket.map_id)
    }

    pub fn activate_wayfinder_map(&mut self, map_id: u64) -> Result<WayfinderMapData, StoreError> {
        let data = self.wayfinder_map_data(map_id)?;
        if data.map.status != "draft" {
            return Err(StoreError::Conflict(
                "only a draft map can be activated".into(),
            ));
        }
        if data.map.destination.trim().is_empty() {
            return Err(StoreError::Validation(
                "the map needs a destination before activation".into(),
            ));
        }
        if data.tickets.is_empty() {
            return Err(StoreError::Validation(
                "the map needs at least one ticket".into(),
            ));
        }
        self.wayfinder_map_mut(map_id)?.status = "active".into();
        self.resolve_bootstrap_ticket(map_id);
        self.refresh_wayfinder_frontier(map_id)?;
        self.materialize_unblocked_code_tickets(map_id)?;
        self.push_wayfinder_event(map_id, None, "map/activated", "user", serde_json::json!({}));
        self.save()?;
        self.wayfinder_map_data(map_id)
    }

    pub fn resolve_wayfinder_ticket(
        &mut self,
        ticket_id: u64,
        resolution: &str,
        additions: &[CreateWayfinderTicketRequest],
        retire_ticket_ids: &[u64],
    ) -> Result<WayfinderMapData, StoreError> {
        let snapshot = self.database.clone();
        let ticket = self.wayfinder_ticket(ticket_id)?;
        let resolution = clean_required(resolution, "resolution", 50_000)?;
        {
            let stored = self.wayfinder_ticket_mut(ticket_id)?;
            stored.status = "resolved".into();
            stored.resolution = resolution.clone();
            stored.updated_at = timestamp();
        }
        for id in retire_ticket_ids {
            if let Err(error) = self.retire_wayfinder_ticket_inner(*id, "rubyn") {
                self.database = snapshot;
                self.save()?;
                return Err(error);
            }
        }
        for addition in additions {
            if addition.map_id != ticket.map_id {
                self.database = snapshot;
                self.save()?;
                return Err(StoreError::Validation(
                    "graph deltas cannot cross maps".into(),
                ));
            }
            if let Err(error) = self.create_wayfinder_ticket(addition, "rubyn") {
                self.database = snapshot;
                self.save()?;
                return Err(error);
            }
        }
        self.refresh_wayfinder_frontier(ticket.map_id)?;
        self.materialize_unblocked_code_tickets(ticket.map_id)?;
        self.push_wayfinder_event(
            ticket.map_id,
            Some(ticket_id),
            "ticket/resolved",
            "user",
            serde_json::json!({"resolution": resolution, "added": additions.len(), "retired": retire_ticket_ids}),
        );
        self.save()?;
        self.wayfinder_map_data(ticket.map_id)
    }

    pub fn complete_wayfinder_user_action(
        &mut self,
        ticket_id: u64,
        result_note: &str,
    ) -> Result<WayfinderMapData, StoreError> {
        let ticket = self.wayfinder_ticket(ticket_id)?;
        if ticket.ticket_type != "user_action" {
            return Err(StoreError::Validation(
                "only User Action tickets use blocker completion".into(),
            ));
        }
        let result_note = clean_required(result_note, "result note", 20_000)?;
        let stored = self.wayfinder_ticket_mut(ticket_id)?;
        stored.status = "resolved".into();
        stored.result_note = result_note.clone();
        stored.resolution = result_note.clone();
        stored.updated_at = timestamp();
        self.refresh_wayfinder_frontier(ticket.map_id)?;
        self.materialize_unblocked_code_tickets(ticket.map_id)?;
        self.push_wayfinder_event(
            ticket.map_id,
            Some(ticket_id),
            "user_action/completed",
            "user",
            serde_json::json!({"result": result_note}),
        );
        self.save()?;
        self.wayfinder_map_data(ticket.map_id)
    }

    pub fn link_wayfinder_run(
        &mut self,
        ticket_id: u64,
        run_id: u64,
    ) -> Result<WayfinderTicket, StoreError> {
        self.run(run_id)?;
        let ticket = self.wayfinder_ticket(ticket_id)?;
        let run = self.run_mut(run_id)?;
        run.background = true;
        run.updated_at = timestamp();
        let stored = self.wayfinder_ticket_mut(ticket_id)?;
        stored.linked_run_id = Some(run_id);
        stored.status = "active".into();
        stored.updated_at = timestamp();
        self.push_wayfinder_event(
            ticket.map_id,
            Some(ticket_id),
            "ticket/run_linked",
            "user",
            serde_json::json!({"runId": run_id, "briefVersion": ticket.brief_version}),
        );
        self.save()?;
        self.wayfinder_ticket(ticket_id)
    }

    pub fn retire_wayfinder_ticket(
        &mut self,
        ticket_id: u64,
    ) -> Result<WayfinderMapData, StoreError> {
        let ticket = self.wayfinder_ticket(ticket_id)?;
        self.retire_wayfinder_ticket_inner(ticket_id, "user")?;
        self.refresh_wayfinder_frontier(ticket.map_id)?;
        self.save()?;
        self.wayfinder_map_data(ticket.map_id)
    }

    pub fn archive_wayfinder_map(&mut self, map_id: u64) -> Result<WayfinderMapData, StoreError> {
        let data = self.wayfinder_map_data(map_id)?;
        if data
            .tickets
            .iter()
            .any(|ticket| !matches!(ticket.status.as_str(), "resolved" | "retired"))
        {
            return Err(StoreError::Conflict(
                "resolve or retire every ticket before completing the map".into(),
            ));
        }
        let map = self.wayfinder_map_mut(map_id)?;
        map.status = "archived".into();
        map.updated_at = timestamp();
        self.push_wayfinder_event(
            map_id,
            None,
            "map/archived",
            "user",
            serde_json::json!({"destination": data.map.destination}),
        );
        self.save()?;
        self.wayfinder_map_data(map_id)
    }

    pub fn wayfinder_blockers(
        &self,
        project_path: &Path,
    ) -> Result<Vec<WayfinderTicket>, StoreError> {
        let project = self.project_by_path(project_path)?;
        let map_ids: HashSet<_> = self
            .database
            .wayfinder_maps
            .iter()
            .filter(|map| map.project_id == project.id && map.status != "archived")
            .map(|map| map.id)
            .collect();
        Ok(self
            .database
            .wayfinder_tickets
            .iter()
            .filter(|ticket| {
                map_ids.contains(&ticket.map_id)
                    && ticket.ticket_type == "user_action"
                    && ticket.status != "resolved"
                    && ticket.status != "retired"
            })
            .cloned()
            .collect())
    }

    fn resolve_bootstrap_ticket(&mut self, map_id: u64) {
        if let Some(ticket) = self
            .database
            .wayfinder_tickets
            .iter_mut()
            .find(|ticket| ticket.map_id == map_id && ticket.title == "Name the destination")
        {
            ticket.status = "resolved".into();
            ticket.resolution =
                "Destination and first frontier approved during map activation.".into();
            ticket.updated_at = timestamp();
        }
    }

    fn refresh_wayfinder_frontier(&mut self, map_id: u64) -> Result<(), StoreError> {
        self.validate_wayfinder_graph(map_id)?;
        let resolved: HashSet<_> = self
            .database
            .wayfinder_tickets
            .iter()
            .filter(|ticket| {
                ticket.map_id == map_id && matches!(ticket.status.as_str(), "resolved" | "retired")
            })
            .map(|ticket| ticket.id)
            .collect();
        for ticket in self
            .database
            .wayfinder_tickets
            .iter_mut()
            .filter(|ticket| ticket.map_id == map_id)
        {
            if matches!(ticket.status.as_str(), "resolved" | "retired" | "active") {
                continue;
            }
            ticket.status = if ticket.depends_on.iter().all(|id| resolved.contains(id)) {
                "frontier"
            } else {
                "blocked"
            }
            .into();
        }
        Ok(())
    }

    fn materialize_unblocked_code_tickets(&mut self, map_id: u64) -> Result<(), StoreError> {
        if self.wayfinder_map(map_id)?.status != "active" {
            return Ok(());
        }
        let map = self.wayfinder_map(map_id)?;
        let project_id = map.project_id;
        let project_path = PathBuf::from(
            self.database
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .ok_or_else(|| StoreError::NotFound(format!("project {project_id}")))?
                .path
                .clone(),
        );
        let ready: Vec<_> = self
            .database
            .wayfinder_tickets
            .iter()
            .filter(|ticket| {
                ticket.map_id == map_id
                    && ticket.ticket_type == "code"
                    && ticket.status == "frontier"
                    && ticket.linked_task_id.is_none()
            })
            .cloned()
            .collect();
        for ticket in ready {
            let task_dependencies = ticket
                .depends_on
                .iter()
                .filter_map(|dependency_id| {
                    self.database
                        .wayfinder_tickets
                        .iter()
                        .find(|candidate| {
                            candidate.id == *dependency_id
                                && candidate.ticket_type == "code"
                                && candidate.status == "resolved"
                        })
                        .and_then(|dependency| dependency.linked_task_id)
                })
                .collect();
            let task_status = self
                .task_status_for_project(project_id, &map.code_task_status)
                .or_else(|_| {
                    self.columns_for_project(project_id)
                        .into_iter()
                        .next()
                        .map(|column| column.key)
                        .ok_or_else(|| {
                            StoreError::Validation("the project has no workflow columns".into())
                        })
                })?;
            let task = self.create_task(
                &project_path,
                &ticket.title,
                &ticket.information,
                &ticket.outcome,
                &task_status,
                task_dependencies,
            )?;
            let stored = self.wayfinder_ticket_mut(ticket.id)?;
            stored.linked_task_id = Some(task.id);
            stored.status = "active".into();
            stored.updated_at = timestamp();
            self.push_wayfinder_event(
                map_id,
                Some(ticket.id),
                "code/task_materialized",
                "harness",
                serde_json::json!({"taskId": task.id}),
            );
        }
        Ok(())
    }

    fn retire_wayfinder_ticket_inner(
        &mut self,
        ticket_id: u64,
        actor: &str,
    ) -> Result<(), StoreError> {
        let ticket = self.wayfinder_ticket(ticket_id)?;
        let stored = self.wayfinder_ticket_mut(ticket_id)?;
        stored.status = "retired".into();
        stored.updated_at = timestamp();
        self.push_wayfinder_event(
            ticket.map_id,
            Some(ticket_id),
            "ticket/retired",
            actor,
            serde_json::json!({"linkedTaskId": ticket.linked_task_id, "linkedRunId": ticket.linked_run_id}),
        );
        Ok(())
    }

    fn validate_wayfinder_dependencies(
        &self,
        map_id: u64,
        ticket_id: Option<u64>,
        dependencies: &[u64],
    ) -> Result<(), StoreError> {
        if dependencies.iter().copied().collect::<HashSet<_>>().len() != dependencies.len() {
            return Err(StoreError::Validation(
                "ticket dependencies must be unique".into(),
            ));
        }
        if ticket_id.is_some_and(|id| dependencies.contains(&id)) {
            return Err(StoreError::Validation(
                "a ticket cannot block itself".into(),
            ));
        }
        for dependency in dependencies {
            let ticket = self.wayfinder_ticket(*dependency)?;
            if ticket.map_id != map_id {
                return Err(StoreError::Validation(
                    "ticket dependencies cannot cross maps".into(),
                ));
            }
        }
        Ok(())
    }

    fn validate_wayfinder_graph(&self, map_id: u64) -> Result<(), StoreError> {
        let tickets: Vec<_> = self
            .database
            .wayfinder_tickets
            .iter()
            .filter(|ticket| ticket.map_id == map_id && ticket.status != "retired")
            .collect();
        let ids: HashSet<_> = tickets.iter().map(|ticket| ticket.id).collect();
        let mut indegree: HashMap<u64, usize> =
            tickets.iter().map(|ticket| (ticket.id, 0)).collect();
        let mut outgoing: HashMap<u64, Vec<u64>> = HashMap::new();
        for ticket in &tickets {
            for dependency in &ticket.depends_on {
                if !ids.contains(dependency) {
                    return Err(StoreError::Validation(format!(
                        "ticket {} depends on a missing or retired ticket",
                        ticket.id
                    )));
                }
                *indegree.entry(ticket.id).or_default() += 1;
                outgoing.entry(*dependency).or_default().push(ticket.id);
            }
        }
        let mut queue: Vec<u64> = indegree
            .iter()
            .filter_map(|(id, count)| (*count == 0).then_some(*id))
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop() {
            visited += 1;
            for child in outgoing.get(&id).into_iter().flatten() {
                let count = indegree.get_mut(child).expect("child is in graph");
                *count -= 1;
                if *count == 0 {
                    queue.push(*child);
                }
            }
        }
        if visited != tickets.len() {
            return Err(StoreError::Validation(
                "Wayfinder ticket dependencies contain a cycle".into(),
            ));
        }
        Ok(())
    }

    fn wayfinder_map(&self, id: u64) -> Result<WayfinderMap, StoreError> {
        self.database
            .wayfinder_maps
            .iter()
            .find(|map| map.id == id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("Wayfinder map {id}")))
    }

    fn wayfinder_map_mut(&mut self, id: u64) -> Result<&mut WayfinderMap, StoreError> {
        self.database
            .wayfinder_maps
            .iter_mut()
            .find(|map| map.id == id)
            .ok_or_else(|| StoreError::NotFound(format!("Wayfinder map {id}")))
    }

    fn wayfinder_ticket(&self, id: u64) -> Result<WayfinderTicket, StoreError> {
        self.database
            .wayfinder_tickets
            .iter()
            .find(|ticket| ticket.id == id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("Wayfinder ticket {id}")))
    }

    fn wayfinder_ticket_mut(&mut self, id: u64) -> Result<&mut WayfinderTicket, StoreError> {
        self.database
            .wayfinder_tickets
            .iter_mut()
            .find(|ticket| ticket.id == id)
            .ok_or_else(|| StoreError::NotFound(format!("Wayfinder ticket {id}")))
    }

    fn push_wayfinder_event(
        &mut self,
        map_id: u64,
        ticket_id: Option<u64>,
        kind: &str,
        actor: &str,
        payload: serde_json::Value,
    ) {
        let id = self.take_wayfinder_event_id();
        self.database.wayfinder_events.push(WayfinderEvent {
            id,
            map_id,
            ticket_id,
            kind: kind.into(),
            actor: actor.into(),
            payload,
            created_at: timestamp(),
        });
    }

    pub fn run(&self, id: u64) -> Result<RunRecord, StoreError> {
        self.database
            .runs
            .iter()
            .find(|run| run.id == id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("run {id}")))
    }

    pub fn runs(&self, project_path: Option<&Path>) -> Result<Vec<RunRecord>, StoreError> {
        let mut runs = match project_path {
            Some(path) => {
                let project = self.project_by_path(path)?;
                self.runs_for_project_id(project.id)
            }
            None => self.database.runs.clone(),
        };
        runs.sort_by_key(|run| std::cmp::Reverse(run.id));
        Ok(runs)
    }

    pub fn events(&self, run_id: u64, after_id: u64) -> Result<Vec<RunEventRecord>, StoreError> {
        self.run(run_id)?;
        Ok(self
            .database
            .events
            .iter()
            .filter(|event| event.run_id == run_id && event.id > after_id)
            .cloned()
            .collect())
    }

    pub fn mark_integrated(
        &mut self,
        id: u64,
        commit: &str,
        cleanup_pending: bool,
    ) -> Result<RunRecord, StoreError> {
        let run = self.run_mut(id)?;
        if run.running {
            return Err(StoreError::Conflict(
                "a running run cannot be integrated".into(),
            ));
        }
        run.lifecycle = if cleanup_pending {
            "integrated_cleanup_pending"
        } else {
            "integrated"
        }
        .into();
        run.integrated_commit = Some(commit.to_owned());
        run.updated_at = timestamp();
        let run = run.clone();
        self.push_system_event(
            id,
            "worktree/integrated",
            serde_json::json!({"commit": commit, "cleanupPending": cleanup_pending}),
        );
        let linked_task_ids: HashSet<_> = self
            .database
            .tasks
            .iter()
            .filter(|task| task.assigned_run_id == Some(id))
            .map(|task| task.id)
            .collect();
        let resolved_tickets: Vec<_> = self
            .database
            .wayfinder_tickets
            .iter_mut()
            .filter(|ticket| {
                ticket.ticket_type == "code"
                    && ticket
                        .linked_task_id
                        .is_some_and(|task_id| linked_task_ids.contains(&task_id))
                    && ticket.status != "retired"
            })
            .map(|ticket| {
                ticket.status = "resolved".into();
                ticket.resolution = format!("Integrated as {commit}");
                ticket.updated_at = timestamp();
                (ticket.map_id, ticket.id)
            })
            .collect();
        for (map_id, ticket_id) in resolved_tickets {
            self.push_wayfinder_event(
                map_id,
                Some(ticket_id),
                "code/integrated",
                "harness",
                serde_json::json!({"runId": id, "commit": commit}),
            );
            self.refresh_wayfinder_frontier(map_id)?;
            self.materialize_unblocked_code_tickets(map_id)?;
        }
        self.save()?;
        Ok(run)
    }

    pub fn mark_integration_started(&mut self, id: u64) -> Result<RunRecord, StoreError> {
        let run = self.run_mut(id)?;
        if run.running || run.lifecycle != "retained" {
            return Err(StoreError::Conflict(
                "only a stopped retained run can begin integration".into(),
            ));
        }
        run.lifecycle = "integrating".into();
        run.updated_at = timestamp();
        let run = run.clone();
        self.push_system_event(id, "worktree/integration_started", serde_json::json!({}));
        self.save()?;
        Ok(run)
    }

    pub fn mark_integration_failed(
        &mut self,
        id: u64,
        detail: &str,
    ) -> Result<RunRecord, StoreError> {
        let run = self.run_mut(id)?;
        if run.lifecycle == "integrating" {
            run.lifecycle = "retained".into();
            run.updated_at = timestamp();
        }
        let run = run.clone();
        self.push_system_event(
            id,
            "worktree/integration_failed",
            serde_json::json!({"error": clean_optional(detail, 20_000)?}),
        );
        self.save()?;
        Ok(run)
    }

    pub fn mark_discarded(
        &mut self,
        id: u64,
        cleanup_pending: bool,
    ) -> Result<RunRecord, StoreError> {
        let run = self.run_mut(id)?;
        if run.running {
            return Err(StoreError::Conflict(
                "a running run cannot be discarded".into(),
            ));
        }
        if run.lifecycle != "retained" {
            return Err(StoreError::Conflict(
                "only a retained run can be discarded".into(),
            ));
        }
        run.lifecycle = if cleanup_pending {
            "discard_cleanup_pending"
        } else {
            "discarded"
        }
        .into();
        run.updated_at = timestamp();
        let run = run.clone();
        self.push_system_event(
            id,
            "worktree/discarded",
            serde_json::json!({"cleanupPending": cleanup_pending}),
        );
        self.save()?;
        Ok(run)
    }

    pub fn mark_cleanup_complete(&mut self, id: u64) -> Result<RunRecord, StoreError> {
        let run = self.run_mut(id)?;
        let disposition = match run.lifecycle.as_str() {
            "integrated_cleanup_pending" => {
                run.lifecycle = "integrated".into();
                "integrated"
            }
            "discard_cleanup_pending" => {
                run.lifecycle = "discarded".into();
                "discarded"
            }
            _ => {
                return Err(StoreError::Conflict(
                    "only a cleanup-pending run can complete cleanup".into(),
                ))
            }
        };
        run.updated_at = timestamp();
        let run = run.clone();
        self.push_system_event(
            id,
            "worktree/cleanup_completed",
            serde_json::json!({"disposition": disposition}),
        );
        self.save()?;
        Ok(run)
    }

    fn task(&self, id: u64) -> Result<TaskRecord, StoreError> {
        self.database
            .tasks
            .iter()
            .find(|task| task.id == id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("task {id}")))
    }

    fn project_by_path(&self, path: &Path) -> Result<ProjectRecord, StoreError> {
        let path = path.to_string_lossy();
        self.database
            .projects
            .iter()
            .find(|project| project.path == path)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("project {path}")))
    }

    fn ensure_project_inner(&mut self, path: &Path) -> ProjectRecord {
        let canonical = path.to_string_lossy().into_owned();
        let now = timestamp();
        if let Some(project) = self
            .database
            .projects
            .iter_mut()
            .find(|project| project.path == canonical)
        {
            project.updated_at = now;
            return project.clone();
        }
        let project = ProjectRecord {
            id: self.take_project_id(),
            name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Project")
                .to_owned(),
            path: canonical,
            created_at: now,
            updated_at: now,
        };
        self.database.projects.push(project.clone());
        self.add_default_agents(project.id);
        self.add_default_columns(project.id);
        project
    }

    fn columns_for_project(&self, project_id: u64) -> Vec<WorkflowColumn> {
        let mut columns: Vec<_> = self
            .database
            .columns
            .iter()
            .filter(|column| column.project_id == project_id)
            .cloned()
            .collect();
        columns.sort_by_key(|column| (column.position, column.id));
        columns
    }

    fn add_default_columns(&mut self, project_id: u64) {
        for (position, (key, name, terminal, agent_role)) in [
            ("backlog", "Backlog", false, None),
            ("planning", "Planning", false, Some("planning")),
            (
                "implementing",
                "Implementing",
                false,
                Some("implementation"),
            ),
            ("review", "Review", false, Some("review")),
            ("done", "Done", true, None),
        ]
        .into_iter()
        .enumerate()
        {
            let id = self.take_column_id();
            self.database.columns.push(WorkflowColumn {
                id,
                project_id,
                key: key.into(),
                name: name.into(),
                position: position as u32,
                terminal,
                agent_id: agent_role.and_then(|role| {
                    self.database
                        .agents
                        .iter()
                        .find(|agent| agent.project_id == project_id && agent.role == role)
                        .map(|agent| agent.id)
                }),
            });
        }
    }

    fn add_default_agents(&mut self, project_id: u64) {
        let now = timestamp();
        for (name, role, _, instructions) in default_agent_profiles() {
            let id = self.take_agent_id();
            self.database.agents.push(AgentProfile {
                id,
                project_id,
                name: name.into(),
                role: role.into(),
                instructions: instructions.into(),
                created_at: now,
                updated_at: now,
            });
        }
    }

    fn migrate_default_agent_instructions(&mut self) {
        let now = timestamp();
        for agent in &mut self.database.agents {
            if let Some((_, _, _, instructions)) =
                default_agent_profiles()
                    .iter()
                    .find(|(name, role, legacy, _)| {
                        agent.name == *name && agent.role == *role && agent.instructions == *legacy
                    })
            {
                agent.instructions = (*instructions).into();
                agent.updated_at = now;
            }
        }
    }

    fn ensure_workflow_columns(&mut self) {
        let project_ids: Vec<_> = self
            .database
            .projects
            .iter()
            .map(|project| project.id)
            .collect();
        for project_id in project_ids {
            if !self
                .database
                .columns
                .iter()
                .any(|column| column.project_id == project_id)
            {
                self.add_default_columns(project_id);
            }
        }
    }

    fn migrate_wayfinder_task_columns(&mut self) {
        let choices: HashMap<u64, (HashSet<String>, String)> = self
            .database
            .projects
            .iter()
            .filter_map(|project| {
                let columns = self.columns_for_project(project.id);
                let first = columns.first()?.key.clone();
                let keys = columns.into_iter().map(|column| column.key).collect();
                Some((project.id, (keys, first)))
            })
            .collect();
        for map in &mut self.database.wayfinder_maps {
            if let Some((valid, first)) = choices.get(&map.project_id) {
                if !valid.contains(&map.code_task_status) {
                    map.code_task_status = first.clone();
                }
            }
        }
    }

    fn ensure_agent_profiles(&mut self) {
        let project_ids: Vec<_> = self
            .database
            .projects
            .iter()
            .map(|project| project.id)
            .collect();
        for project_id in project_ids {
            if !self
                .database
                .agents
                .iter()
                .any(|agent| agent.project_id == project_id)
            {
                self.add_default_agents(project_id);
            }
        }
    }

    fn migrate_column_policies(&mut self) {
        for column in &mut self.database.columns {
            let role = match column.key.as_str() {
                "planning" => Some("planning"),
                "implementing" => Some("implementation"),
                "review" => Some("review"),
                _ => None,
            };
            if column.agent_id.is_none() {
                column.agent_id = role.and_then(|role| {
                    self.database
                        .agents
                        .iter()
                        .find(|agent| agent.project_id == column.project_id && agent.role == role)
                        .map(|agent| agent.id)
                });
            }
        }
    }

    fn migrate_legacy_task_statuses(&mut self) {
        for task in &mut self.database.tasks {
            task.status = match task.status.as_str() {
                "queued" => "backlog".into(),
                "blocked" => "planning".into(),
                "doing" => "implementing".into(),
                other => other.to_owned(),
            };
        }
    }

    fn migrate_conversation_metadata(&mut self) {
        let mut background_ids: HashSet<_> = self
            .database
            .tasks
            .iter()
            .filter_map(|task| task.assigned_run_id)
            .collect();
        background_ids.extend(
            self.database
                .wayfinder_tickets
                .iter()
                .filter_map(|ticket| ticket.linked_run_id),
        );
        for run in &mut self.database.runs {
            if run.title.trim().is_empty() {
                run.title = conversation_title(&run.prompt);
            }
            if background_ids.contains(&run.id) {
                run.background = true;
            }
        }
    }

    fn task_status_for_project(&self, project_id: u64, status: &str) -> Result<String, StoreError> {
        let requested = status.trim().to_ascii_lowercase();
        let normalized = match requested.as_str() {
            "queued" | "pending" => "backlog".into(),
            "blocked" => "planning".into(),
            "doing" | "in_progress" => "implementing".into(),
            "completed" => "done".into(),
            other => other.to_owned(),
        };
        if self
            .database
            .columns
            .iter()
            .any(|column| column.project_id == project_id && column.key == normalized)
        {
            Ok(normalized)
        } else if matches!(requested.as_str(), "queued" | "pending") {
            self.columns_for_project(project_id)
                .into_iter()
                .find(|column| !column.terminal)
                .or_else(|| self.columns_for_project(project_id).into_iter().next())
                .map(|column| column.key)
                .ok_or_else(|| StoreError::Validation("the project has no workflow columns".into()))
        } else {
            Err(StoreError::Validation(format!(
                "unknown workflow column {status:?}"
            )))
        }
    }

    fn normalize_column_positions(&mut self, project_id: u64) {
        let mut ids: Vec<_> = self
            .database
            .columns
            .iter()
            .filter(|column| column.project_id == project_id)
            .map(|column| (column.position, column.id))
            .collect();
        ids.sort_unstable();
        for (position, (_, id)) in ids.into_iter().enumerate() {
            if let Some(column) = self
                .database
                .columns
                .iter_mut()
                .find(|column| column.id == id)
            {
                column.position = position as u32;
            }
        }
    }

    fn runs_for_project_id(&self, project_id: u64) -> Vec<RunRecord> {
        let mut runs: Vec<_> = self
            .database
            .runs
            .iter()
            .filter(|run| run.project_id == project_id)
            .cloned()
            .collect();
        runs.sort_by_key(|run| std::cmp::Reverse(run.id));
        runs
    }

    fn run_mut(&mut self, id: u64) -> Result<&mut RunRecord, StoreError> {
        self.database
            .runs
            .iter_mut()
            .find(|run| run.id == id)
            .ok_or_else(|| StoreError::NotFound(format!("run {id}")))
    }

    fn validate_dependencies(
        &self,
        project_id: u64,
        task_id: Option<u64>,
        dependencies: &[u64],
    ) -> Result<(), StoreError> {
        let unique: HashSet<_> = dependencies.iter().copied().collect();
        if unique.len() != dependencies.len() {
            return Err(StoreError::Validation(
                "task dependencies cannot contain duplicates".into(),
            ));
        }
        if task_id.is_some_and(|id| unique.contains(&id)) {
            return Err(StoreError::Validation(
                "a task cannot depend on itself".into(),
            ));
        }
        for dependency in dependencies {
            let task = self.task(*dependency)?;
            if task.project_id != project_id {
                return Err(StoreError::Validation(
                    "task dependencies must belong to the same project".into(),
                ));
            }
        }
        Ok(())
    }

    fn validate_task_graph(&self, project_id: u64) -> Result<(), StoreError> {
        let edges: HashMap<_, _> = self
            .database
            .tasks
            .iter()
            .filter(|task| task.project_id == project_id)
            .map(|task| (task.id, task.depends_on.as_slice()))
            .collect();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        for task_id in edges.keys().copied() {
            if graph_has_cycle(task_id, &edges, &mut visiting, &mut visited) {
                return Err(StoreError::Validation(
                    "task dependencies would create a cycle".into(),
                ));
            }
        }
        Ok(())
    }

    fn refresh_task_readiness(&mut self) {
        let terminal_keys: HashSet<_> = self
            .database
            .columns
            .iter()
            .filter(|column| column.terminal)
            .map(|column| (column.project_id, column.key.clone()))
            .collect();
        let done: HashSet<_> = self
            .database
            .tasks
            .iter()
            .filter(|task| terminal_keys.contains(&(task.project_id, task.status.clone())))
            .map(|task| task.id)
            .collect();
        for task in &mut self.database.tasks {
            task.ready = !terminal_keys.contains(&(task.project_id, task.status.clone()))
                && task.depends_on.iter().all(|id| done.contains(id));
        }
    }

    fn push_system_event(&mut self, run_id: u64, kind: &str, payload: serde_json::Value) {
        let id = self.take_event_id();
        self.database.events.push(RunEventRecord {
            id,
            run_id,
            protocol_sequence: 0,
            kind: kind.into(),
            raw: String::new(),
            payload,
            created_at: timestamp(),
        });
    }

    fn recover_interrupted_runs(&mut self) -> bool {
        let interrupted: Vec<_> = self
            .database
            .runs
            .iter_mut()
            .filter(|run| run.running)
            .map(|run| {
                let now = timestamp();
                run.running = false;
                run.outcome = "failed".into();
                run.updated_at = now;
                run.finished_at = Some(now);
                run.id
            })
            .collect();
        let interrupted_integrations: Vec<_> = self
            .database
            .runs
            .iter_mut()
            .filter(|run| run.lifecycle == "integrating")
            .map(|run| {
                run.lifecycle = "retained".into();
                run.updated_at = timestamp();
                run.id
            })
            .collect();
        for run_id in &interrupted {
            self.push_system_event(
                *run_id,
                "run/interrupted",
                serde_json::json!({"reason": "application restarted"}),
            );
        }
        let interrupted_ids: HashSet<_> = interrupted.iter().copied().collect();
        for approval in &mut self.database.approvals {
            if approval.status == "pending" && interrupted_ids.contains(&approval.run_id) {
                approval.status = "expired".into();
                approval.decided_at = Some(timestamp());
            }
        }
        for run_id in &interrupted_integrations {
            self.push_system_event(
                *run_id,
                "worktree/integration_interrupted",
                serde_json::json!({"reason": "application restarted; retry integration to reconcile"}),
            );
        }
        !interrupted.is_empty() || !interrupted_integrations.is_empty()
    }

    fn scrub_provider_diagnostics(&mut self) -> bool {
        let mut changed = false;
        for run in &mut self.database.runs {
            if !run.stdout.is_empty() || !run.stderr.is_empty() {
                run.stdout.clear();
                run.stderr.clear();
                changed = true;
            }
        }
        for event in &mut self.database.events {
            if !event.raw.is_empty() {
                event.raw.clear();
                changed = true;
            }
            let provider_diagnostic = event.kind == "codex/event"
                || event.kind == "process/stderr"
                || event.kind.starts_with("mcpServer/")
                || event.kind.starts_with("account/")
                || event.kind.starts_with("remoteControl/")
                || event.kind.starts_with("thread/")
                || event.kind.starts_with("turn/");
            if provider_diagnostic && event.payload != serde_json::json!({"withheld": true}) {
                event.payload = serde_json::json!({"withheld": true});
                changed = true;
            }
        }
        changed
    }

    fn repair_counters(&mut self) {
        self.database.version = DATABASE_VERSION;
        self.database.next_project_id =
            next_after(self.database.projects.iter().map(|item| item.id));
        self.database.next_agent_id = next_after(self.database.agents.iter().map(|item| item.id));
        self.database.next_column_id = next_after(self.database.columns.iter().map(|item| item.id));
        self.database.next_task_id = next_after(self.database.tasks.iter().map(|item| item.id));
        self.database.next_todo_id = next_after(self.database.todos.iter().map(|item| item.id));
        self.database.next_run_id = next_after(self.database.runs.iter().map(|item| item.id));
        self.database.next_event_id = next_after(self.database.events.iter().map(|item| item.id));
        self.database.next_approval_id =
            next_after(self.database.approvals.iter().map(|item| item.id));
        self.database.next_wayfinder_map_id =
            next_after(self.database.wayfinder_maps.iter().map(|item| item.id));
        self.database.next_wayfinder_ticket_id =
            next_after(self.database.wayfinder_tickets.iter().map(|item| item.id));
        self.database.next_wayfinder_question_id =
            next_after(self.database.wayfinder_questions.iter().map(|item| item.id));
        self.database.next_wayfinder_event_id =
            next_after(self.database.wayfinder_events.iter().map(|item| item.id));
    }

    fn take_project_id(&mut self) -> u64 {
        take_id(&mut self.database.next_project_id)
    }

    fn take_agent_id(&mut self) -> u64 {
        take_id(&mut self.database.next_agent_id)
    }

    fn take_column_id(&mut self) -> u64 {
        take_id(&mut self.database.next_column_id)
    }

    fn take_task_id(&mut self) -> u64 {
        take_id(&mut self.database.next_task_id)
    }

    fn take_todo_id(&mut self) -> u64 {
        take_id(&mut self.database.next_todo_id)
    }

    fn take_run_id(&mut self) -> u64 {
        take_id(&mut self.database.next_run_id)
    }

    fn take_event_id(&mut self) -> u64 {
        take_id(&mut self.database.next_event_id)
    }

    fn take_approval_id(&mut self) -> u64 {
        take_id(&mut self.database.next_approval_id)
    }

    fn take_wayfinder_map_id(&mut self) -> u64 {
        take_id(&mut self.database.next_wayfinder_map_id)
    }

    fn take_wayfinder_ticket_id(&mut self) -> u64 {
        take_id(&mut self.database.next_wayfinder_ticket_id)
    }

    fn take_wayfinder_event_id(&mut self) -> u64 {
        take_id(&mut self.database.next_wayfinder_event_id)
    }

    fn save(&self) -> Result<(), StoreError> {
        let serialized = serde_json::to_vec_pretty(&self.database)?;
        let temporary = self.directory.join("harness-database.tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);
        if self.file.is_file() {
            fs::copy(&self.file, &self.backup)?;
            File::open(&self.backup)?.sync_all()?;
        }
        if let Err(error) = fs::rename(&temporary, &self.file) {
            if error.kind() != io::ErrorKind::AlreadyExists {
                return Err(error.into());
            }
            fs::remove_file(&self.file)?;
            fs::rename(&temporary, &self.file)?;
        }
        #[cfg(unix)]
        File::open(&self.directory)?.sync_all()?;
        Ok(())
    }
}

fn graph_has_cycle(
    task_id: u64,
    edges: &HashMap<u64, &[u64]>,
    visiting: &mut HashSet<u64>,
    visited: &mut HashSet<u64>,
) -> bool {
    if visited.contains(&task_id) {
        return false;
    }
    if !visiting.insert(task_id) {
        return true;
    }
    if edges.get(&task_id).is_some_and(|dependencies| {
        dependencies
            .iter()
            .any(|dependency| graph_has_cycle(*dependency, edges, visiting, visited))
    }) {
        return true;
    }
    visiting.remove(&task_id);
    visited.insert(task_id);
    false
}

fn read_database(path: &Path) -> Result<PersistentDatabase, StoreError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn wayfinder_ticket_type(value: &str) -> Result<&'static str, StoreError> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "grill" | "grilling" => Ok("grill"),
        "research" => Ok("research"),
        "prototype" => Ok("prototype"),
        "code" => Ok("code"),
        "user_action" | "user action" | "task" => Ok("user_action"),
        _ => Err(StoreError::Validation(
            "ticket type must be Grill, Research, Prototype, Code, or User Action".into(),
        )),
    }
}

fn default_wayfinder_model(ticket_type: &str) -> &'static str {
    match ticket_type {
        "research" => "terra",
        "prototype" => "terra",
        _ => "sol",
    }
}

fn question_answer_text(question: &WayfinderQuestion) -> Option<String> {
    if !question.custom_answer.trim().is_empty() {
        return Some(question.custom_answer.clone());
    }
    let labels: Vec<_> = question
        .answers
        .iter()
        .filter_map(|id| question.options.iter().find(|option| &option.id == id))
        .map(|option| option.description.clone())
        .collect();
    (!labels.is_empty()).then(|| labels.join("; "))
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn conversation_title(prompt: &str) -> String {
    let first_line = prompt
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("New conversation");
    let mut title: String = first_line.trim().chars().take(80).collect();
    if first_line.trim().chars().count() > 80 {
        title.push('…');
    }
    title
}

fn take_id(next: &mut u64) -> u64 {
    let id = (*next).max(1);
    *next = id.saturating_add(1);
    id
}

fn next_after(values: impl Iterator<Item = u64>) -> u64 {
    values.max().unwrap_or(0).saturating_add(1).max(1)
}

fn clean_required(value: &str, label: &str, maximum: usize) -> Result<String, StoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StoreError::Validation(format!("{label} cannot be empty")));
    }
    if value.len() > maximum {
        return Err(StoreError::Validation(format!(
            "{label} cannot exceed {maximum} bytes"
        )));
    }
    Ok(value.to_owned())
}

fn default_agent_profiles() -> [(&'static str, &'static str, &'static str, &'static str); 3] {
    [
        (
            "Planner",
            "planning",
            "Clarify scope, dependencies, risks, and an executable plan before code changes.",
            "Mission\nTurn the task into an executable plan that removes guesswork before implementation.\n\nStarting context\nRead the task information, expected outcome, dependencies, project instructions, and relevant code before planning.\n\nWorking method\nClarify unknowns, identify risks, sequence the work, and connect every step to concrete files and verification.\n\nFinish line\nThe plan names the changes, order, tests, risks, and acceptance criteria clearly enough for Rubyn to implement.\n\nGuardrails\nDo not edit code or invent product intent. Ask when a decision materially changes scope, behavior, or safety.",
        ),
        (
            "Builder",
            "implementation",
            "Implement the bounded task in its isolated worktree and verify the expected outcome.",
            "Mission\nImplement the bounded task completely in its isolated worktree.\n\nStarting context\nRead the task, expected outcome, dependencies, project instructions, existing implementation, and relevant tests before editing.\n\nWorking method\nMake the smallest coherent change, follow project conventions, test proportionally to risk, and keep the worktree reviewable.\n\nFinish line\nThe expected outcome is observable, relevant tests pass, and the resulting diff is focused and ready for independent review.\n\nGuardrails\nDo not broaden scope, hide failures, or integrate the work. Ask before destructive, ambiguous, or externally consequential actions.",
        ),
        (
            "Reviewer",
            "review",
            "Independently inspect the diff, tests, risks, and acceptance criteria; do not silently integrate it.",
            "Mission\nIndependently determine whether the proposed work is correct, safe, and complete.\n\nStarting context\nRead the task and acceptance criteria, inspect the full diff and affected code paths, and examine the available test evidence.\n\nWorking method\nTrace behavior and failure modes, prioritize findings by impact, and cite exact evidence without assuming the builder was correct.\n\nFinish line\nEvery material defect is actionable, residual risks are explicit, and the work has a clear review verdict.\n\nGuardrails\nDo not silently fix or integrate the work. Ask when product intent is required to judge correctness.",
        ),
    ]
}

fn clean_optional(value: &str, maximum: usize) -> Result<String, StoreError> {
    if value.len() > maximum {
        return Err(StoreError::Validation(format!(
            "value cannot exceed {maximum} bytes"
        )));
    }
    Ok(value.trim().to_owned())
}

fn json_u64(value: Option<&serde_json::Value>) -> Result<u64, StoreError> {
    value
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .ok_or_else(|| {
            StoreError::Validation("Harness task operation requires a numeric task_id".into())
        })
}

fn agent_task_status(status: &str) -> &str {
    match status {
        "pending" => "queued",
        "in_progress" => "doing",
        "completed" => "done",
        "blocked" => "blocked",
        "review" => "review",
        other => other,
    }
}

fn agent_todo_status(status: &str) -> &str {
    match status {
        "pending" => "queued",
        "in_progress" => "doing",
        "completed" => "done",
        other => other,
    }
}

fn todo_status(value: &str) -> Result<String, StoreError> {
    normalized_status(value, &["queued", "doing", "review", "done"])
}

fn unique_column_key(name: &str, columns: &[WorkflowColumn], project_id: u64) -> String {
    let base = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() { "column" } else { &base };
    let mut candidate = base.to_owned();
    let mut suffix = 2;
    while columns
        .iter()
        .any(|column| column.project_id == project_id && column.key == candidate)
    {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn normalized_status(value: &str, allowed: &[&str]) -> Result<String, StoreError> {
    let normalized = value.trim().to_ascii_lowercase();
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(StoreError::Validation(format!(
            "unsupported status {value:?}; expected one of {}",
            allowed.join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rubyn-store-{label}-{}-{}",
            std::process::id(),
            timestamp()
        ))
    }

    #[test]
    fn project_scoped_records_and_dag_survive_reopen() {
        let directory = test_directory("records");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        repository.record_project(&project_path).unwrap();
        let first = repository
            .create_task(
                &project_path,
                "Implement",
                "Focused slice",
                "Working feature",
                "doing",
                vec![],
            )
            .unwrap();
        let second = repository
            .create_task(
                &project_path,
                "Review",
                "Independent review",
                "Review evidence recorded",
                "queued",
                vec![first.id],
            )
            .unwrap();
        assert!(!second.ready);
        repository
            .update_task(first.id, None, None, None, Some("done"), None)
            .unwrap();
        repository
            .create_todo(&project_path, "Inspect diff", "You", "queued")
            .unwrap();
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let data = reopened.project_data(&project_path).unwrap();
        assert_eq!(data.project.name, "example-app");
        assert!(
            data.tasks
                .iter()
                .find(|task| task.id == second.id)
                .unwrap()
                .ready
        );
        assert_eq!(data.todos[0].title, "Inspect diff");
        assert_eq!(data.tasks[0].outcome, "Working feature");
        assert_eq!(data.tasks[1].outcome, "Review evidence recorded");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn dependency_cycles_are_rejected() {
        let directory = test_directory("cycle");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let first = repository
            .create_task(&project_path, "First", "", "", "queued", vec![])
            .unwrap();
        let second = repository
            .create_task(&project_path, "Second", "", "", "queued", vec![first.id])
            .unwrap();
        assert!(matches!(
            repository.update_task(first.id, None, None, None, None, Some(vec![second.id])),
            Err(StoreError::Validation(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn running_records_are_recovered_as_failed_without_losing_worktree() {
        let directory = test_directory("recovery");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Do work".into(),
                "prompt".into(),
            )
            .unwrap();
        repository.mark_run_started(run.id, Some(42)).unwrap();
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let recovered = reopened.run(run.id).unwrap();
        assert!(!recovered.running);
        assert_eq!(recovered.outcome, "failed");
        assert_eq!(recovered.lifecycle, "retained");
        assert!(reopened
            .events(run.id, 0)
            .unwrap()
            .iter()
            .any(|event| event.kind == "run/interrupted"));
        drop(reopened);
        let mut reopened = StateRepository::open(&directory).unwrap();
        let next = reopened
            .allocate_run(
                &project_path,
                &worktree_path,
                "def456".into(),
                "More work".into(),
                "prompt".into(),
            )
            .unwrap();
        assert!(next.id > run.id);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn cleanup_completion_preserves_the_recorded_disposition() {
        let directory = test_directory("cleanup-completion");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let integrated = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Integrate work".into(),
                "prompt".into(),
            )
            .unwrap();
        repository
            .mark_integrated(integrated.id, "def456", true)
            .unwrap();
        assert!(repository.mark_discarded(integrated.id, false).is_err());
        assert_eq!(
            repository
                .mark_cleanup_complete(integrated.id)
                .unwrap()
                .lifecycle,
            "integrated"
        );
        assert!(repository.mark_cleanup_complete(integrated.id).is_err());

        let discarded = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Discard work".into(),
                "prompt".into(),
            )
            .unwrap();
        repository.mark_discarded(discarded.id, true).unwrap();
        assert_eq!(
            repository
                .mark_cleanup_complete(discarded.id)
                .unwrap()
                .lifecycle,
            "discarded"
        );
        assert!(repository
            .events(discarded.id, 0)
            .unwrap()
            .iter()
            .any(|event| event.kind == "worktree/cleanup_completed"));
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        assert_eq!(reopened.run(integrated.id).unwrap().lifecycle, "integrated");
        assert_eq!(reopened.run(discarded.id).unwrap().lifecycle, "discarded");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalized_engine_events_are_durable_and_deduplicated() {
        let directory = test_directory("events");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Do work".into(),
                "prompt".into(),
            )
            .unwrap();
        let event = EngineEvent {
            run_id: run.id,
            sequence: 7,
            kind: "stream/text".into(),
            payload: serde_json::json!({"text": "hello", "final": false}),
            raw: "raw-frame".into(),
            created_at: timestamp(),
        };
        repository
            .append_engine_events(&[event.clone(), event.clone()])
            .unwrap();
        repository
            .append_engine_events(&[EngineEvent {
                sequence: event.sequence,
                created_at: event.created_at + 1,
                raw: "new-process-frame".into(),
                payload: serde_json::json!({"text": "continued", "final": false}),
                ..event
            }])
            .unwrap();
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let events: Vec<_> = reopened
            .events(run.id, 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.protocol_sequence == 7)
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "stream/text");
        assert_eq!(events[0].payload["text"], "hello");
        assert_eq!(events[1].payload["text"], "continued");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn edit_approvals_are_persisted_audited_and_expired_after_restart() {
        let directory = test_directory("edit-approvals");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Edit the model".into(),
                "prompt".into(),
            )
            .unwrap();
        repository.mark_run_started(run.id, Some(42)).unwrap();
        repository
            .append_engine_events(&[
                EngineEvent {
                    run_id: run.id,
                    sequence: 7,
                    kind: "file/edit".into(),
                    payload: serde_json::json!({"editId":"edit-7","path":"app/models/user.rb","content":"class User\nend\n","type":"modify"}),
                    raw: "edit-frame".into(),
                    created_at: timestamp(),
                },
                EngineEvent {
                    run_id: run.id,
                    sequence: 8,
                    kind: "file/create".into(),
                    payload: serde_json::json!({"editId":"edit-8","path":"app/services/export.rb","content":"class Export\nend\n"}),
                    raw: "create-frame".into(),
                    created_at: timestamp(),
                },
                EngineEvent {
                    run_id: run.id,
                    sequence: 9,
                    kind: "command/approval".into(),
                    payload: serde_json::json!({"editId":"command-9","path":"/work/example-app","content":"bundle exec rails test","type":"command","approvalKind":"commandExecution"}),
                    raw: "command-frame".into(),
                    created_at: timestamp(),
                },
            ])
            .unwrap();

        let approved = repository
            .resolve_edit_approval(run.id, "edit-7", true)
            .unwrap();
        assert_eq!(approved.status, "approved");
        assert!(repository
            .events(run.id, 0)
            .unwrap()
            .iter()
            .any(|event| event.kind == "approval/decision" && event.payload["accepted"] == true));
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let approvals = reopened.project_data(&project_path).unwrap().approvals;
        assert_eq!(approvals.len(), 3);
        assert_eq!(
            approvals
                .iter()
                .find(|item| item.edit_id == "edit-7")
                .unwrap()
                .status,
            "approved"
        );
        assert_eq!(
            approvals
                .iter()
                .find(|item| item.edit_id == "edit-8")
                .unwrap()
                .status,
            "expired"
        );
        let command = approvals
            .iter()
            .find(|item| item.edit_id == "command-9")
            .unwrap();
        assert_eq!(command.approval_kind, "commandExecution");
        assert_eq!(command.status, "expired");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_provider_diagnostics_are_scrubbed_from_primary_and_backup_state() {
        let directory = test_directory("diagnostic-scrub");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Inspect diagnostics".into(),
                "prompt".into(),
            )
            .unwrap();
        repository
            .sync_run(
                run.id,
                false,
                "failed",
                None,
                "provider stdout with credential-shaped data",
                "provider stderr with credential-shaped data",
            )
            .unwrap();
        repository
            .append_engine_events(&[EngineEvent {
                run_id: run.id,
                sequence: 7,
                kind: "mcpServer/startupStatus/updated".into(),
                payload: serde_json::json!({"error":"credential-shaped data"}),
                raw: "raw credential-shaped data".into(),
                created_at: timestamp(),
            }])
            .unwrap();
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let recovered = reopened.run(run.id).unwrap();
        assert!(recovered.stdout.is_empty());
        assert!(recovered.stderr.is_empty());
        let diagnostic = reopened
            .events(run.id, 0)
            .unwrap()
            .into_iter()
            .find(|event| event.kind == "mcpServer/startupStatus/updated")
            .unwrap();
        assert!(diagnostic.raw.is_empty());
        assert_eq!(diagnostic.payload, serde_json::json!({"withheld": true}));
        drop(reopened);

        for file in ["harness-database.json", "harness-database.backup.json"] {
            let contents = fs::read_to_string(directory.join(file)).unwrap();
            assert!(!contents.contains("credential-shaped data"));
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn diagnostic_summary_exposes_counts_without_project_or_conversation_content() {
        let directory = test_directory("diagnostic-summary");
        let project_path = directory.join("SOURCE_PATH_CANARY");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &directory.join("worktrees/run-1/workspace"),
                "base-canary".into(),
                "TITLE_CANARY".into(),
                "PROMPT_CANARY".into(),
            )
            .unwrap();
        repository
            .sync_run(
                run.id,
                false,
                "failed",
                None,
                "STDOUT_CANARY",
                "STDERR_CANARY",
            )
            .unwrap();

        let summary = repository.diagnostic_summary();
        assert_eq!(summary.run_count, 1);
        let serialized = serde_json::to_string(&summary).unwrap();
        assert!(serialized.contains("projectCount"));
        for canary in [
            "SOURCE_PATH_CANARY",
            "TITLE_CANARY",
            "PROMPT_CANARY",
            "STDOUT_CANARY",
            "STDERR_CANARY",
            "base-canary",
        ] {
            assert!(!serialized.contains(canary));
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn conversations_can_be_renamed_pinned_and_archived_without_becoming_runs() {
        let directory = test_directory("conversation-metadata");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let conversation = repository
            .allocate_run(
                &project_path,
                &directory.join("worktree"),
                "abc123".into(),
                "Investigate billing exports".into(),
                "prompt".into(),
            )
            .unwrap();
        assert_eq!(conversation.title, "Investigate billing exports");
        assert!(!conversation.background);

        let renamed = repository
            .update_conversation(
                conversation.id,
                Some("Billing export investigation"),
                Some(true),
                None,
            )
            .unwrap();
        assert_eq!(renamed.title, "Billing export investigation");
        assert!(renamed.pinned);

        let archived = repository
            .update_conversation(conversation.id, None, None, Some(true))
            .unwrap();
        assert!(archived.archived_at.is_some());
        assert!(!archived.pinned);
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        assert!(reopened.run(conversation.id).unwrap().archived_at.is_some());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn assigning_a_conversation_to_a_task_makes_it_a_background_run() {
        let directory = test_directory("background-run-classification");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let conversation = repository
            .allocate_run(
                &project_path,
                &directory.join("worktree"),
                "abc123".into(),
                "Implement billing exports".into(),
                "prompt".into(),
            )
            .unwrap();
        let task = repository
            .create_task(
                &project_path,
                "Implement exports",
                "",
                "Exports work",
                "backlog",
                vec![],
            )
            .unwrap();

        repository
            .assign_task(task.id, Some(conversation.id))
            .unwrap();

        assert!(repository.run(conversation.id).unwrap().background);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn event_history_is_bounded_per_run() {
        let directory = test_directory("event-retention");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Do work".into(),
                "prompt".into(),
            )
            .unwrap();
        let events: Vec<_> = (1..=5_020)
            .map(|sequence| EngineEvent {
                run_id: run.id,
                sequence,
                kind: "stream/text".into(),
                payload: serde_json::json!({"text": sequence}),
                raw: sequence.to_string(),
                created_at: timestamp(),
            })
            .collect();
        repository.append_engine_events(&events).unwrap();
        let retained = repository.events(run.id, 0).unwrap();
        assert_eq!(retained.len(), 5_000);
        assert_eq!(retained.first().unwrap().protocol_sequence, 21);
        assert_eq!(retained.last().unwrap().protocol_sequence, 5_020);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn codex_thread_identity_is_retained_for_conversation_continuation() {
        let directory = test_directory("conversation-backend-thread");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Do work".into(),
                "prompt".into(),
            )
            .unwrap();

        repository
            .append_engine_events(&[EngineEvent {
                run_id: run.id,
                sequence: 1,
                kind: "engine/thread".into(),
                payload: serde_json::json!({"threadId":"codex-thread-123"}),
                raw: String::new(),
                created_at: timestamp(),
            }])
            .unwrap();

        assert_eq!(
            repository.backend_thread_id(run.id).unwrap().as_deref(),
            Some("codex-thread-123")
        );

        let legacy = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Continue old Codex work".into(),
                "prompt".into(),
            )
            .unwrap();
        repository
            .sync_run(
                legacy.id,
                false,
                "waiting",
                None,
                "{\"id\":1,\"result\":{\"thread\":{\"id\":\"legacy-codex-thread\"}}}",
                "",
            )
            .unwrap();
        assert_eq!(
            repository.backend_thread_id(legacy.id).unwrap().as_deref(),
            Some("legacy-codex-thread")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn workflow_columns_are_configurable_and_assignments_are_project_scoped() {
        let directory = test_directory("workflow-columns");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        repository.record_project(&project_path).unwrap();
        let task = repository
            .create_task(&project_path, "Ship slice", "", "", "backlog", vec![])
            .unwrap();
        let todo = repository
            .create_todo(&project_path, "Confirm evidence", "You", "queued")
            .unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Implement the slice".into(),
                "prompt".into(),
            )
            .unwrap();
        assert_eq!(
            repository
                .assign_task(task.id, Some(run.id))
                .unwrap()
                .assigned_run_id,
            Some(run.id)
        );
        assert_eq!(
            repository
                .assign_todo(todo.id, Some(run.id))
                .unwrap()
                .assigned_run_id,
            Some(run.id)
        );

        let qa = repository
            .create_workflow_column(&project_path, "QA ready")
            .unwrap();
        let moved = repository
            .update_workflow_column(qa.id, Some("Verification"), Some(1), None)
            .unwrap();
        assert_eq!(moved.name, "Verification");
        assert_eq!(moved.position, 1);
        repository
            .update_task(task.id, None, None, None, Some(&qa.key), None)
            .unwrap();
        let backlog = repository.project_data(&project_path).unwrap().columns[0].clone();
        repository
            .delete_workflow_column(qa.id, backlog.id)
            .unwrap();
        let data = repository.project_data(&project_path).unwrap();
        assert_eq!(
            data.columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Backlog", "Planning", "Implementing", "Review", "Done"]
        );
        assert_eq!(data.tasks[0].status, "backlog");

        let planning = data.columns[1].clone();
        repository
            .delete_workflow_column(backlog.id, planning.id)
            .unwrap();
        let after_intake_delete = repository
            .create_task(&project_path, "Still creatable", "", "", "queued", vec![])
            .unwrap();
        assert_eq!(after_intake_delete.status, planning.key);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn column_policies_assign_agent_profiles_without_launching_runs() {
        let directory = test_directory("column-agent-policies");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        repository.record_project(&project_path).unwrap();
        let data = repository.project_data(&project_path).unwrap();
        assert_eq!(
            data.agents
                .iter()
                .map(|agent| agent.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Planner", "Builder", "Reviewer"]
        );
        for agent in &data.agents {
            for heading in [
                "Mission",
                "Starting context",
                "Working method",
                "Finish line",
                "Guardrails",
            ] {
                assert!(
                    agent.instructions.contains(heading),
                    "{} is missing {heading}",
                    agent.name
                );
            }
        }
        let planning = data
            .columns
            .iter()
            .find(|column| column.key == "planning")
            .unwrap();
        let planner_id = planning.agent_id.unwrap();

        let task = repository
            .create_task(&project_path, "Shape work", "", "", "backlog", vec![])
            .unwrap();
        assert_eq!(task.assigned_agent_id, None);
        let task = repository
            .update_task(task.id, None, None, None, Some("planning"), None)
            .unwrap();
        assert_eq!(task.assigned_agent_id, Some(planner_id));
        assert_eq!(
            repository.project_data(&project_path).unwrap().runs.len(),
            0
        );

        let custom = repository
            .create_agent_profile(
                &project_path,
                "Security",
                "review",
                "Check tenant boundaries.",
            )
            .unwrap();
        let custom = repository
            .update_agent_profile(
                custom.id,
                Some("Security reviewer"),
                Some("security review"),
                Some("Check authorization and tenant boundaries."),
            )
            .unwrap();
        assert_eq!(custom.name, "Security reviewer");
        assert_eq!(custom.role, "security review");
        let review = repository
            .project_data(&project_path)
            .unwrap()
            .columns
            .into_iter()
            .find(|column| column.key == "review")
            .unwrap();
        repository
            .update_workflow_column(review.id, None, None, Some(Some(custom.id)))
            .unwrap();
        let task = repository
            .update_task(task.id, None, None, None, Some("review"), None)
            .unwrap();
        assert_eq!(task.assigned_agent_id, Some(custom.id));
        repository.delete_agent_profile(custom.id).unwrap();
        assert_eq!(repository.task(task.id).unwrap().assigned_agent_id, None);
        assert_eq!(
            repository
                .project_data(&project_path)
                .unwrap()
                .columns
                .into_iter()
                .find(|column| column.key == "review")
                .unwrap()
                .agent_id,
            None
        );

        let planning = repository
            .project_data(&project_path)
            .unwrap()
            .columns
            .into_iter()
            .find(|column| column.key == "planning")
            .unwrap();
        repository
            .update_workflow_column(planning.id, None, None, Some(None))
            .unwrap();
        drop(repository);
        let reopened = StateRepository::open(&directory).unwrap();
        let planning = reopened
            .project_data(&project_path)
            .unwrap()
            .columns
            .into_iter()
            .find(|column| column.key == "planning")
            .unwrap();
        assert_eq!(planning.agent_id, None);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn upgrades_only_untouched_default_agent_instructions() {
        let directory = test_directory("agent-instruction-migration");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        repository.record_project(&project_path).unwrap();
        repository.database.version = 6;
        let planner = repository
            .database
            .agents
            .iter_mut()
            .find(|agent| agent.name == "Planner")
            .unwrap();
        planner.instructions =
            "Clarify scope, dependencies, risks, and an executable plan before code changes."
                .into();
        let builder = repository
            .database
            .agents
            .iter_mut()
            .find(|agent| agent.name == "Builder")
            .unwrap();
        builder.instructions = "Keep my custom builder instructions.".into();
        repository.save().unwrap();
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let data = reopened.project_data(&project_path).unwrap();
        assert!(data
            .agents
            .iter()
            .find(|agent| agent.name == "Planner")
            .unwrap()
            .instructions
            .contains("Finish line"));
        assert_eq!(
            data.agents
                .iter()
                .find(|agent| agent.name == "Builder")
                .unwrap()
                .instructions,
            "Keep my custom builder instructions."
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rubyn_harness_tool_calls_mutate_the_shared_board_and_are_audited() {
        let directory = test_directory("harness-tools");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Plan the work".into(),
                "prompt".into(),
            )
            .unwrap();
        repository.mark_run_started(run.id, Some(42)).unwrap();
        let event = EngineEvent {
            run_id: run.id,
            sequence: 9,
            kind: "tool/use".into(),
            payload: serde_json::json!({
                "requestId": "tool-9",
                "tool": "harness_task",
                "args": {"kind": "task", "action": "create", "title": "Add request spec", "description": "Cover tenancy"}
            }),
            raw: "tool-frame".into(),
            created_at: timestamp(),
        };
        repository.apply_harness_tool_events(&[event]).unwrap();
        let data = repository.project_data(&project_path).unwrap();
        assert_eq!(data.tasks[0].title, "Add request spec");
        assert_eq!(data.tasks[0].status, "backlog");
        assert!(repository
            .events(run.id, 0)
            .unwrap()
            .iter()
            .any(|event| event.kind == "harness/control_applied"));
        assert!(worktree_path
            .parent()
            .unwrap()
            .join("harness-control.json")
            .is_file());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rubyn_wayfinder_tool_calls_create_graph_nodes_that_materialize_as_tasks() {
        let directory = test_directory("harness-wayfinder-tools");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let map = repository
            .create_wayfinder_map(&project_path, "Choose tenant isolation", None)
            .unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Chart the map".into(),
                "prompt".into(),
            )
            .unwrap();
        repository.mark_run_started(run.id, Some(42)).unwrap();
        let event = |sequence, request_id: &str, args: serde_json::Value| EngineEvent {
            run_id: run.id,
            sequence,
            kind: "tool/use".into(),
            payload: serde_json::json!({
                "requestId": request_id,
                "tool": "wayfinder",
                "args": args
            }),
            raw: "tool-frame".into(),
            created_at: timestamp(),
        };
        repository
            .apply_harness_tool_events(&[
                event(
                    1,
                    "map",
                    serde_json::json!({"action":"update_map","map_id":map.map.id,"destination":"Reviewed tenant isolation design"}),
                ),
                event(
                    2,
                    "research",
                    serde_json::json!({"action":"create_node","map_id":map.map.id,"title":"Research tenant storage","question":"Which storage boundary is feasible?","node_type":"research"}),
                ),
                event(
                    3,
                    "code",
                    serde_json::json!({"action":"create_node","map_id":map.map.id,"title":"Implement tenant boundary","description":"Apply the selected storage boundary","outcome":"Isolation specs pass","node_type":"code","blocked_by":["Research tenant storage"]}),
                ),
            ])
            .unwrap();

        let generated = repository.wayfinder_map_data(map.map.id).unwrap();
        assert_eq!(
            generated.map.destination,
            "Reviewed tenant isolation design"
        );
        let research = generated
            .tickets
            .iter()
            .find(|ticket| ticket.title == "Research tenant storage")
            .unwrap();
        let code = generated
            .tickets
            .iter()
            .find(|ticket| ticket.title == "Implement tenant boundary")
            .unwrap();
        assert_eq!(code.depends_on, vec![research.id]);

        repository.activate_wayfinder_map(map.map.id).unwrap();
        repository
            .resolve_wayfinder_ticket(research.id, "Use separate schemas", &[], &[])
            .unwrap();
        let ready = repository.wayfinder_map_data(map.map.id).unwrap();
        let code = ready
            .tickets
            .iter()
            .find(|ticket| ticket.title == "Implement tenant boundary")
            .unwrap();
        assert!(code.linked_task_id.is_some());
        assert!(repository
            .project_data(&project_path)
            .unwrap()
            .tasks
            .iter()
            .any(|task| task.title == "Implement tenant boundary"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn codex_can_import_a_wayfinder_map_without_a_blank_bootstrap_node() {
        let directory = test_directory("codex-wayfinder-import");
        let project_path = directory.join("example-app");
        let worktree_path = directory.join("worktrees/run-1/workspace");
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let run = repository
            .allocate_run(
                &project_path,
                &worktree_path,
                "abc123".into(),
                "Import map".into(),
                "prompt".into(),
            )
            .unwrap();
        repository.mark_run_started(run.id, Some(42)).unwrap();
        let event = |sequence, request_id: &str, args: serde_json::Value| EngineEvent {
            run_id: run.id,
            sequence,
            kind: "tool/use".into(),
            payload: serde_json::json!({"requestId":request_id,"tool":"wayfinder","args":args}),
            raw: "codex-dynamic-tool".into(),
            created_at: timestamp(),
        };
        repository
            .apply_harness_tool_events(&[
                event(1, "import", serde_json::json!({"action":"import_map","title":"PO chaser pivot","idea":"Pivot the product","destination":"A validated direction"})),
                event(2, "node", serde_json::json!({"action":"create_node","map_id":"PO chaser pivot","title":"Validate buyer","node_type":"research","question":"Who pays?"})),
            ])
            .unwrap();

        let map = repository.wayfinder_maps(&project_path).unwrap().remove(0);
        let data = repository.wayfinder_map_data(map.id).unwrap();
        assert_eq!(data.map.title, "PO chaser pivot");
        assert_eq!(data.map.destination, "A validated direction");
        assert_eq!(
            data.tickets
                .iter()
                .filter(|ticket| ticket.status != "retired")
                .count(),
            1
        );
        assert!(data
            .tickets
            .iter()
            .any(|ticket| ticket.title == "Validate buyer"));
        assert!(data
            .tickets
            .iter()
            .any(|ticket| ticket.title == "Name the destination" && ticket.status == "retired"));
        fs::remove_dir_all(directory).unwrap();
    }

    fn wayfinder_ticket_request(
        map_id: u64,
        title: &str,
        ticket_type: &str,
        depends_on: Vec<u64>,
    ) -> CreateWayfinderTicketRequest {
        CreateWayfinderTicketRequest {
            map_id,
            title: title.into(),
            question: format!("What settles {title}?"),
            information: "Known project context".into(),
            outcome: format!("{title} is evidenced"),
            ticket_type: ticket_type.into(),
            depends_on,
            model_role: None,
            effort: None,
            budget_cents: None,
        }
    }

    #[test]
    fn wayfinder_maps_start_with_a_skill_driven_bootstrap_node_and_survive_reopen() {
        let directory = test_directory("wayfinder-bootstrap");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let created = repository
            .create_wayfinder_map(&project_path, "Choose a tenant boundary", None)
            .unwrap();
        assert!(created.questions.is_empty());
        assert_eq!(created.tickets.len(), 1);
        assert_eq!(created.tickets[0].title, "Name the destination");
        assert_eq!(created.tickets[0].ticket_type, "grill");
        drop(repository);

        let reopened = StateRepository::open(&directory).unwrap();
        let data = reopened.wayfinder_map_data(created.map.id).unwrap();
        assert!(data.questions.is_empty());
        assert!(data.map.destination.is_empty());
        assert!(data.events.iter().any(|event| event.kind == "map/created"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn wayfinder_rejects_cycles_and_materializes_code_only_at_the_frontier() {
        let directory = test_directory("wayfinder-dag");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let code_column = repository
            .create_workflow_column(&project_path, "Ready for code")
            .unwrap();
        let map = repository
            .create_wayfinder_map(&project_path, "Ship scoped exports", Some(&code_column.key))
            .unwrap();
        assert_eq!(map.map.code_task_status, code_column.key);
        repository
            .update_wayfinder_map(
                map.map.id,
                None,
                Some("Scoped exports work and are reviewed"),
                None,
            )
            .unwrap();
        let blocker = repository
            .create_wayfinder_ticket(
                &wayfinder_ticket_request(map.map.id, "Confirm retention", "user_action", vec![]),
                "user",
            )
            .unwrap();
        let code = repository
            .create_wayfinder_ticket(
                &wayfinder_ticket_request(map.map.id, "Build export", "code", vec![blocker.id]),
                "user",
            )
            .unwrap();
        let cycle = UpdateWayfinderTicketRequest {
            id: blocker.id,
            title: None,
            question: None,
            information: None,
            outcome: None,
            depends_on: Some(vec![code.id]),
            model_role: None,
            effort: None,
            budget_cents: None,
        };
        assert!(matches!(
            repository.update_wayfinder_ticket(&cycle),
            Err(StoreError::Validation(_))
        ));
        let active = repository.activate_wayfinder_map(map.map.id).unwrap();
        assert_eq!(
            active
                .tickets
                .iter()
                .find(|ticket| ticket.id == blocker.id)
                .unwrap()
                .status,
            "frontier"
        );
        assert!(repository
            .project_data(&project_path)
            .unwrap()
            .tasks
            .is_empty());
        let completed = repository
            .complete_wayfinder_user_action(blocker.id, "Retention is 90 days")
            .unwrap();
        let code = completed
            .tickets
            .iter()
            .find(|ticket| ticket.id == code.id)
            .unwrap();
        assert_eq!(code.status, "active");
        assert!(code.linked_task_id.is_some());
        let task = repository.project_data(&project_path).unwrap().tasks[0].clone();
        assert_eq!(task.title, "Build export");
        assert_eq!(task.outcome, "Build export is evidenced");
        assert_eq!(task.status, code_column.key);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn wayfinder_code_tasks_preserve_code_task_dependencies() {
        let directory = test_directory("wayfinder-task-dependencies");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let map = repository
            .create_wayfinder_map(&project_path, "Ship exports in ordered steps", None)
            .unwrap();
        repository
            .update_wayfinder_map(map.map.id, None, Some("Exports ship safely"), None)
            .unwrap();
        let schema = repository
            .create_wayfinder_ticket(
                &wayfinder_ticket_request(map.map.id, "Add export schema", "code", vec![]),
                "user",
            )
            .unwrap();
        let endpoint = repository
            .create_wayfinder_ticket(
                &wayfinder_ticket_request(
                    map.map.id,
                    "Add export endpoint",
                    "code",
                    vec![schema.id],
                ),
                "user",
            )
            .unwrap();

        let active = repository.activate_wayfinder_map(map.map.id).unwrap();
        let schema_task_id = active
            .tickets
            .iter()
            .find(|ticket| ticket.id == schema.id)
            .and_then(|ticket| ticket.linked_task_id)
            .unwrap();
        repository
            .resolve_wayfinder_ticket(schema.id, "Schema integrated", &[], &[])
            .unwrap();

        let data = repository.wayfinder_map_data(map.map.id).unwrap();
        let endpoint_task_id = data
            .tickets
            .iter()
            .find(|ticket| ticket.id == endpoint.id)
            .and_then(|ticket| ticket.linked_task_id)
            .unwrap();
        let endpoint_task = repository.task(endpoint_task_id).unwrap();
        assert_eq!(endpoint_task.depends_on, vec![schema_task_id]);
        assert!(!endpoint_task.ready);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejected_graph_delta_is_atomic_on_disk() {
        let directory = test_directory("wayfinder-atomic");
        let project_path = directory.join("example-app");
        fs::create_dir_all(&project_path).unwrap();
        let mut repository = StateRepository::open(&directory).unwrap();
        let map = repository
            .create_wayfinder_map(&project_path, "Choose authorization architecture", None)
            .unwrap();
        let grill = repository
            .create_wayfinder_ticket(
                &wayfinder_ticket_request(map.map.id, "Settle policy boundary", "grill", vec![]),
                "user",
            )
            .unwrap();
        let cross_map = repository
            .create_wayfinder_map(&project_path, "A different destination", None)
            .unwrap();
        let invalid = wayfinder_ticket_request(cross_map.map.id, "Wrong map", "research", vec![]);
        assert!(matches!(
            repository.resolve_wayfinder_ticket(grill.id, "Use policy objects", &[invalid], &[]),
            Err(StoreError::Validation(_))
        ));
        drop(repository);
        let reopened = StateRepository::open(&directory).unwrap();
        let persisted = reopened.wayfinder_map_data(map.map.id).unwrap();
        assert_ne!(
            persisted
                .tickets
                .iter()
                .find(|ticket| ticket.id == grill.id)
                .unwrap()
                .status,
            "resolved"
        );
        assert!(!persisted
            .events
            .iter()
            .any(|event| event.kind == "ticket/resolved"));
        fs::remove_dir_all(directory).unwrap();
    }
}
