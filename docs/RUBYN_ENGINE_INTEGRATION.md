# Rubyn Code Engine Integration

**Engine snapshot:** vendored gitlink `engine/rubyn-code` at `531dd08` (inspected 2026-08-21). This document describes the implementation in that snapshot, rather than treating the README as an API contract.

## Decision summary

Rubyn Code is a strong Ruby/Rails agent runtime to embed behind an adapter. Its `--ide` mode is a newline-delimited JSON-RPC 2.0 process protocol, with streaming text, tool visibility, review findings, editable-file approvals, configuration, planning, and basic in-memory multi-session support.

It is **not yet a complete multi-project engine API**. In particular, the IDE boot path does not initialise SQLite migrations, `SessionPersistence`, `BudgetEnforcer`, skills autoloading, task/team services, or MCP clients. Those are wired by the CLI REPL instead. Rubyn Harness must own the product-facing project/agent/task model and introduce or upstream the missing engine bridge deliberately.

## Source of truth

| Concern | Authoritative source |
| --- | --- |
| CLI entry and flags | `engine/rubyn-code/exe/rubyn-code`; `lib/rubyn_code/cli/app.rb` |
| Stdio JSON-RPC server | `lib/rubyn_code/ide/server.rb`; `lib/rubyn_code/ide/protocol.rb` |
| IDE request handlers | `lib/rubyn_code/ide/handlers.rb`; `lib/rubyn_code/ide/handlers/*.rb` |
| Wire schemas and lifecycle fixtures | `protocol/schema.json`; `protocol/fixtures/*.json` |
| Approval and edit gating | `lib/rubyn_code/ide/adapters/tool_output.rb` |
| REPL-only composition root | `lib/rubyn_code/cli/repl_setup.rb` |
| Durable sessions | `lib/rubyn_code/memory/session_persistence.rb` |
| Tasks and dependencies | `lib/rubyn_code/tasks/manager.rb`; `lib/rubyn_code/tasks/dag.rb`; `lib/rubyn_code/tools/task.rb` |
| Teams and mailbox | `lib/rubyn_code/teams/*.rb`; `lib/rubyn_code/tools/spawn_teammate.rb` |
| Budget/cost | `lib/rubyn_code/observability/budget_enforcer.rb`; `lib/rubyn_code/observability/usage_reporter.rb` |
| Skills | `lib/rubyn_code/skills/*.rb`; `lib/rubyn_code/cli/commands/skill.rb` |
| Compaction | `lib/rubyn_code/context/{manager,compactor,micro_compact,auto_compact,decision_compactor}.rb` |

## Launch and shutdown contract

Launch a long-lived child process with an explicit workspace:

```text
rubyn-code --ide --dir /absolute/path/to/project --permission-mode default
```

For the checked-in source during development, the equivalent entry point is:

```text
bundle exec ruby -Ilib exe/rubyn-code --ide --dir /absolute/path/to/project --permission-mode default
```

`--ide` overrides the normal REPL command. `--dir` is accepted only when the path exists, then calls `Dir.chdir`; an `initialize` request may also call `Dir.chdir` when it receives an existing `workspacePath`. `--permission-mode` accepts `default`, `accept_edits`, `plan_only`, `auto`, `dont_ask`, or `bypass`; `--yolo` resolves to `bypass`.

The server reads exactly one JSON object per stdin line and writes exactly one JSON object plus `\n` per stdout message. It flushes stdout; diagnostics are written to stderr. A client disconnect (stdin EOF), `shutdown`, `SIGTERM`, or `SIGINT` ends the read loop. Send the `shutdown` request, wait for `{ "shutdown": true }`, close stdin, then allow a short grace period before escalating to `SIGTERM`. Note that shutdown attempts `session_persistence.save`; the shipped `Memory::SessionPersistence` exposes `save_session`, not `save`, so this is best-effort and currently logs an error if that object is installed unchanged.

### Safe process model for Rubyn Harness

Use a supervised **engine actor process per active agent execution**; never share one Rubyn process across different projects. Keep the app's project registry, task board, run state, budgets, and audit log outside the engine process. This is required because Rubyn uses process-global `Dir.chdir` and its `IDE::Server#tool_output_adapter` is one mutable server attribute, replaced each time a prompt builds an agent loop.

For a request, persist the Harness run first, spawn the actor with an absolute executable and `--dir`, perform `initialize`, then route every JSON-RPC notification into the run event stream. Serialize prompts within an actor until the upstream adapter is made session-keyed. Parallel fan-out means multiple actors, each with a separate working tree/sandbox and an explicit budget reservation—not concurrent `prompt` calls to one actor.

Do not send terminal text, logs, or shell output to the engine's stdin. Do not parse stderr as protocol. Cap stderr retention, associate it with the run, redact secrets, and treat unexpected stdout/non-JSON as a protocol failure. Use OS process groups so cancellation can terminate child commands spawned by the engine; Rubyn's own cancellation only raises `Interrupt` in its agent thread.

## JSON-RPC 2.0 API

Every request is `{ "jsonrpc":"2.0", "id": <string|number>, "method": <string>, "params": <object> }`. Responses preserve the id and contain either `result` or `error`. Notifications have no `id`. Parsing enforces the JSON-RPC version and object/array params, but handlers frequently perform only presence checks; Harness must validate inputs before sending them.

Standard errors are `-32700` parse, `-32600` invalid request, `-32601` method not found, `-32602` invalid params, and `-32603` internal. Engine-defined constants include `-1` agent busy, `-2` session not found, and `-3` budget exceeded, though the current IDE handlers do not raise the latter three. Planning adds `-32001`, `-32010`, and `-32011`.

### Client → engine methods

| Method | Params | Result / asynchronous output |
| --- | --- | --- |
| `initialize` | `workspacePath?`, `extensionVersion?`, `capabilities?` | `{serverVersion, protocolVersion:"1.0", workspacePath, capabilities:{tools,skills,streaming:true,review:true,memory:true,teams:true,toolApproval:true,editApproval:true}}`. Source accepts missing `extensionVersion`, although the schema marks it required. |
| `prompt` | `text` (defaults to empty), `sessionId?`, `context?` with `workspacePath?`, `provider?`, `model?`, `activeFile?`, `selection? {startLine,endLine,text}`, `openFiles?` | Immediate `{accepted:true,sessionId}`. Later `agent/status`, `stream/text`, and tool/edit notifications. Supplying the same session id cancels its current thread and retains its in-memory conversation. Provider/model are explicit per Harness actor so parallel runs do not race through global defaults. |
| `cancel` | `sessionId` | `{cancelled:true,sessionId}`; missing id returns `{cancelled:false,error}`. The implementation reports success even when no live session exists. |
| `review` | `baseBranch?` (defaults `main`), `focus?` (defaults `all`), `sessionId?` | Immediate `{accepted:true,sessionId}`; emits reviewing, zero or more findings, then done/error. |
| `approveToolUse` | `requestId`, `approved` | `{resolved:true,requestId}` or `{resolved:false,error}`. Reply only to the matching `tool/use` request id. |
| `acceptEdit` | `editId`, `accepted` | `{applied:<accepted>}` or `{applied:false,error}`. Reply only to the matching `file/edit` / `file/create` edit id. |
| `config/get` | `key?` | With a key: `{key,value,source}`; otherwise `{settings:{key:{value,default}},providers}`. Exposes provider/model/iteration/output/context/budget/permission settings. |
| `config/set` | `key`, `value` | `{updated,key,value}` or `{updated:false,error}` and, on success, `config/changed`. `permission_mode` changes this process only; other exposed keys are persisted to `~/.rubyn-code/config.yml`. |
| `models/list` | none | `{models:[{provider,model,tier}],activeProvider,activeModel,modelMode}`. |
| `providers/upsert` | `name`, `baseUrl`, `apiFormat`, `models`, optional `envKey`, `apiKey` | Validates and persists an OpenAI- or Anthropic-compatible provider. A supplied key is sent directly to Rubyn's encrypted token store and is never returned. |
| `session/reset` | `sessionId` | `{reset:true,sessionId}` after cancelling and dropping only the in-memory conversation. |
| `session/list` | `projectPath?`, `limit?` | `{sessions:[{id,title,updatedAt,messageCount}]}` when a persistence object has been installed; otherwise `{sessions:[]}`. |
| `session/resume` | `sessionId` | `{resumed:true,sessionId,messages}` only with a persistence object; failure is a successful result containing `resumed:false,error`. |
| `session/fork` | `sessionId`, `messageIndex` | `{forked:true,newSessionId}` only with persistence; it preserves the source session and saves messages before `messageIndex`. Failure is `{forked:false,error}`. |
| `plan/propose` | `feature` (nonblank) | Synchronous `{slug,feature,phases:[{number,slug?,name,summary,requirements_md,design_md,tasks_md}]}`. It may block 5–30 seconds. |
| `plan/interview/start` | none | `{sessionId}` plus `plan/interview/question` or `plan/interview/done`. This method is implemented but missing from `protocol/schema.json`'s method registry. |
| `plan/interview/answer` | `sessionId`, `questionId`, `answer` | `{}` plus next question or final plan; errors are JSON-RPC `-32010/-32011/-32602`. |
| `plan/interview/cancel` | `sessionId` | Intended as a notification; deleting an unknown interview is a no-op. If sent as a request the generic server returns a `null` result. |
| `recover_ci` | CI context in camelCase: `planId`, `phaseNumber`, `branch`, `prNumber`, `trimmedLog`, `attemptNumber`, `maxAttempts`, optional log/check/commit/phase/session fields | Immediate `{accepted:true,sessionId}`, then recovery events and final status. The handler only checks that params is a hash, so Harness must apply the stricter schema. |
| `shutdown` | none | `{shutdown:true}` and stops the read loop. It attempts a `session_persistence.save` first, but that method is absent on the shipped persistence implementation. |

### Engine → client notifications

| Notification | Params actually emitted |
| --- | --- |
| `agent/status` | `{sessionId,status}` where prompt uses `thinking`, `streaming`, `done`, `cancelled`, or `error`; review uses `reviewing`; CI recovery uses `recovering`. May include `error`, `summary`, `phaseNumber`, `attemptNumber`. |
| `stream/text` | `{sessionId,text,final:false}` for streamed chunks and `{sessionId,response,final:true}` at prompt completion. The final full response can overlap prior deltas; render it as a terminal replacement/authoritative completion rather than blindly append it. |
| `tool/use` | `{requestId,tool,args,requiresApproval}`. |
| `tool/result` | `{requestId,tool,success,summary}`. Summaries are capped at 500 characters. |
| `file/edit` | `{editId,path,content,type}` for a modification; `type` is currently the preview type. |
| `file/create` | `{editId,path,content}` for a new file. |
| `review/finding` | `{sessionId,index,severity,message,file,line}`; source only extracts findings tagged `[critical]`, `[warning]`, `[suggestion]`, or `[nitpick]`. |
| `config/changed` | `{key,value}`. |
| `plan/interview/question` | `{sessionId,questionId,text,options}`. |
| `plan/interview/done` | `{sessionId,plan}`. |
| `plan/interview/error` | `{sessionId,message}`. |
| `recovery/outcome` | `{sessionId,planId,phaseNumber,kind:"fixed"|"no_fix"|"errored",commitSha?,summary?}`. |

The schema additionally declares `session/cost` and outbound IDE requests (`ide/readSelection`, `ide/getDiagnostics`, `ide/getWorkspaceSymbols`, etc.). `IDE::Client` supports arbitrary server-initiated requests with ids beginning at 1000 and a 30-second wait, but the prompt adapter does not currently emit `session/cost`, and those IDE request names are used only when IDE-aware tools call the client. Treat them as optional capability, not a required Harness dependency.

### Minimal happy path

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"workspacePath":"/work/acme","extensionVersion":"rubyn-harness/0.1","capabilities":{"streaming":true,"inlineDiff":true}}}
{"jsonrpc":"2.0","id":2,"method":"prompt","params":{"sessionId":"run_01","text":"Add a Rails request spec","context":{"workspacePath":"/work/acme"}}}
```

The second response merely acknowledges acceptance. Finish a Harness run only after a terminal state: `error`/`cancelled`, or both `agent/status:done` and `stream/text.final:true`. The current prompt handler emits `done` immediately before its final text notification, so the adapter needs that order-independent latch rather than treating `done` alone as completion.

## Permission and diff model

`ToolOutput` emits a `tool/use` notification for every tool. Its hard-coded read-only list includes `read_file`, `glob`, `grep`, Git inspection/commit, memory search, web fetch/search, and `run_specs`; all other tools are considered non-read-only. `write_file` and `edit_file` are special: the engine computes proposed content first, emits a file notification, and waits indefinitely for `acceptEdit` in `default` mode. Other non-read-only tools wait indefinitely for `approveToolUse`.

Permission modes behave as follows:

| Mode | Behaviour in source |
| --- | --- |
| `default` | edit acceptance for files; approval for other non-read-only tools |
| `accept_edits` | emits file diff/create and applies it automatically; asks for other non-read-only tools |
| `plan_only` | blocks all non-read-only tools |
| `auto`, `bypass` | executes all tools without waiting |
| `dont_ask` | denies all non-read-only tools |

Harness should default to `default`, put a time-bounded approval policy above Rubyn's indefinite waits, and always record both the proposed content and the acceptance decision. Do not label `git_commit` as harmless just because it is in Rubyn's read-only list; the Harness policy must be stricter than the engine's display policy.

## Existing capabilities and limitations

### Sessions

Prompt conversations are held in `PromptHandler`'s process-local hash. `session/reset` clears that memory. Durable list/resume/fork needs `server.session_persistence`, but `IDE::Server` initialises it to `nil` and no IDE handler creates it. Unlike `CLI::ReplSetup`, IDE startup does not obtain `DB::Connection`, migrate it, or build `Memory::SessionPersistence`; therefore the shipped IDE mode currently returns empty/unavailable persistence results.

### Tasks, todo board, teams, and fan-out

`Tasks::Manager` is durable SQLite CRUD with dependency DAG support, and `TodoWrite` provides an in-turn checklist only. `Teams::Manager`, `Mailbox`, and `AgentRegistry` provide durable teammate metadata and messages. `spawn_teammate` creates a thread and a teammate record, but it requires both an injected LLM client and DB. The IDE prompt path constructs `Tools::Executor` without assigning `llm_client`, `db`, or a background worker; `spawn_teammate` therefore cannot work there as composed today. No IDE RPC methods expose tasks, agent registry, mailbox, or todo state. The Harness task and todo boards must be app-owned until a new engine service/endpoint exists.

### Skills

The engine ships extensive Ruby, Rails, testing, SOLID, and design-pattern skills under `engine/rubyn-code/skills`. `initialize` counts core/project/user skill directories, and review loads best-effort pack context. The IDE prompt path does **not** build `Skills::Loader`, matcher, registry autoload, or pass them to `Agent::Loop`; REPL does. Thus use Rubyn's checked-in skills as a Harness-managed catalog or add a bridge before promising automatic skill activation in IDE runs.

### Budget and cost

Rubyn has SQLite-backed `BudgetEnforcer`, defaults of $5 per session and $10 daily, and a cost reporter. The REPL constructs it and passes it into `Agent::Loop`; IDE `PromptHandler` does not. `config/get`/`config/set` expose budget values, but that does not make enforcement active in IDE mode. Reserve and enforce fan-out budgets in Harness, and reconcile engine usage only after an explicit usage notification/API is added.

### Compaction

`Agent::Loop` in IDE mode does construct `Context::Manager`, so automatic compaction is active: it estimates roughly four characters per token; it micro-compacts old tool results near 70% of threshold for cached providers or 50% for OpenAI-compatible providers; then attempts context collapse, then LLM summary compaction. Default threshold is 80,000 tokens. The explicit `compact` tool is not wired with the context manager in `Tools::Executor`, so it reports compaction unavailable; there is no IDE `compact` RPC. App-level manual compact must be an engine extension (or use a new short-lived run) rather than rely on that tool.

## Staged adapter plan

1. **Foundation:** retain Rubyn as a pinned submodule/release artifact; implement a Rust `EngineProcess` supervisor with absolute command, per-run working directory, bounded stdout/stderr capture, cancel/shutdown escalation, and contract tests replaying `protocol/fixtures`. The current harness ships discovery, isolated worktree allocation, JSON-RPC initialize/prompt framing, deny-by-default non-file approvals, persisted interactive file-edit decisions, lifecycle polling, graceful shutdown escalation, bounded output retrieval, and typed streamed UI events.
2. **Usable execution:** map the verified `initialize`, `prompt`, approval/edit, review, config, models, planning, and shutdown methods to a typed Harness port. Persist raw engine events alongside normalised run events. Run one prompt per actor.
3. **Harness-owned orchestration:** build projects, agent definitions/model selection, task board/DAG, user+agent todo board, fan-out scheduling, worktree allocation, budget reservation, diff inspection, and review inspection in Tauri. Treat engine status as an execution signal, not as the product database.
4. **Engine bridge:** upstream or maintain a small Rubyn integration layer that performs the REPL-equivalent boot for IDE mode: DB migration, `SessionPersistence`, `BudgetEnforcer`, skill loader/matcher/autoload, task/team/mailbox services, MCP discovery, and session-scoped tool-output adapters. Add versioned RPCs for `tasks/*`, `teams/*`, `skills/*`, `budget/status`, `context/compact`, `todo/changed`, and `session/cost`.
5. **Parallel production:** replace the process-global workspace and adapter state with an `EngineSession` keyed by Harness run/session. Then allow multiple sessions per engine only after race/cancellation/approval tests pass. Until then retain one actor per execution.

## Acceptance tests for the adapter

- Start with `--dir`, issue `initialize`, and assert every stdout line is valid JSON-RPC while stderr has no protocol data.
- Run prompt → terminal status; confirm the response acknowledgement never completes the run.
- Exercise a non-file approval and a file acceptance/rejection; assert pending approval rejects on cancellation and no edit is written after rejection.
- Run review and preserve all structured `review/finding` events.
- Validate every supported request and notification against the vendored `protocol/schema.json`, with explicit exceptions for the three implemented-but-unregistered interview methods.
- Verify two concurrent Harness agents use separate process IDs, worktrees, budgets, and event streams.
- Mark IDE persistence/tasks/teams/budget/skill autoload tests as expected failures until the bridge is present; do not represent them as shipped engine capabilities.
