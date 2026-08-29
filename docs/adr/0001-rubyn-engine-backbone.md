# ADR 0001: Adopt Rubyn Code behind a supervised engine adapter

- Status: Accepted
- Date: 2026-08-21

## Context

Rubyn Harness is a Tauri product for Ruby and Rails teams that needs low-cost parallel agent execution, reviewable diffs, task/wayfinding boards, durable progress, skills, and model choice. The repository includes Rubyn Code at `engine/rubyn-code` (`531dd08`), which provides a Ruby/Rails-focused agent loop, tools, skills, review, compaction, and a stdio JSON-RPC IDE server.

Its IDE interface is valuable but its runtime composition is incomplete: `--ide` does not initialise REPL services for SQLite sessions, budgets, skills autoloading, MCP, tasks, or teams. It also changes process-global working directory and has a server-global edit/approval adapter.

## Decision

Use Rubyn Code as the default coding-engine backbone, accessed only through a typed, supervised adapter. Rubyn Harness remains the system of record for product state and orchestration.

Each active agent execution receives a dedicated engine actor process and isolated working directory/worktree. The adapter speaks newline-delimited JSON-RPC over stdio, maps Rubyn messages into stable Harness run events, and persists the raw message for debugging. Harness owns project selection, agent/model configuration, task DAG, todos, concurrency scheduling, budgets, approvals, diffs, reviews, and user-visible history.

The adapter exposes verified IDE capabilities: initialization, persistent in-process prompt conversations, streaming/cancel, tool and edit approvals, review, configuration/model listing, planning, CI recovery, and shutdown. Durable run/event history and the project board remain Harness-owned. The audited `harness_task` tool bridges Rubyn create/update/complete requests into that native board; teams, budget enforcement, and inspectable manual compaction remain unimplemented.

## Consequences

Positive:

- Ruby/Rails skills, tools, review logic, and compaction can be used immediately without coupling UI code to a CLI transcript.
- Process isolation gives credible cancellation, prevents project-directory cross-talk, and makes parallel work predictable.
- A port/adapter boundary permits future Codex, Claude, OpenCode, and local-model engines without changing product state.
- Harness can enforce a global spend policy even while an individual engine lacks IDE budget enforcement.

Costs:

- Process startup and event normalisation add implementation work.
- Parallel work consumes separate processes and worktrees by design.
- Some attractive Rubyn capabilities require an upstream/forked IDE composition bridge before they can be product features.

## Guardrails

- Never pass a shell string composed from user input; launch an absolute executable with an argv array.
- Treat stdout as protocol only and stderr as bounded diagnostic output.
- Send `shutdown`, wait briefly, then terminate the process group only when necessary.
- Use `default` permission mode inside the disposable isolated worktree. Persist
  every proposed file edit and require an explicit decision in the conversation;
  expire unresolved proposals when the bounded run ends. Continue denying
  non-file tool approval requests.
- One prompt at a time per actor until tool approvals are session-scoped upstream.
- Reserve budget before fan-out and stop scheduling when the Harness budget is exhausted.

## Follow-up

Create a Rubyn IDE bridge that bootstraps DB/migrations, durable session persistence, budget enforcement, skills, MCP, tasks, teams, and session-keyed approval state. Add versioned RPC methods for those services and contract-test them before enabling their corresponding Harness UI features.
