# Phase 03 — Durable Execution & Recovery Design

## Overview

Phase 03 extends the existing durable approval ledger from file changes to command execution. The provider protocol remains isolated in the engine adapter; the store records normalized approval events; and the conversation UI presents one approval component with kind-specific safety language.

## Architecture

### Codex command-approval adapter

**Responsibility:** Normalize `item/commandExecution/requestApproval` into an auditable execution request and return a one-shot `accept` or `decline` response.

**Collaborators:** Codex app-server transport, engine event stream, and the native approval command.

**Why not inline?** Protocol identity may use both a JSON-RPC request ID and an optional callback approval ID. Encoding and decoding that identity in one adapter prevents cross-request decisions.

### Durable approval ledger

**Responsibility:** Persist command and file-change decisions with a discriminator while preserving compatibility with existing beta state.

**Collaborators:** Run event ingestion, restart recovery, and project-data serialization.

**Why not inline?** Decisions must be persisted before UI rendering, survive restart, and expire consistently with interrupted runs.

### Kind-aware approval card

**Responsibility:** Display commands as commands, file changes as diffs, and give each kind accurate impact and button labels.

**Collaborators:** Conversation selection, native resolution bridge, and refresh polling.

**Why not inline?** Safety language must not imply that a command leaves the worktree unchanged or that approving a file diff executes a shell command.

### Normalized usage telemetry

**Responsibility:** Retain only numeric, provider-reported cumulative token totals and expose cache reuse as a non-monetary efficiency signal.

**Collaborators:** Codex event normalization and the selected conversation header.

**Why not inline?** Provider transport events may contain unrelated account or environment data. An allowlisted numeric payload prevents the usage feature from reopening raw-transport privacy risk.

## Data model changes

`EditApprovalRecord` gains an `approvalKind` discriminator with a backward-compatible default of `fileChange`. Existing field names remain serialized for beta-state compatibility: command approvals store the working directory in `path`, the exact command plus optional reason in `content`, and `command` in `editType`.

The encoded `editId` remains the opaque provider callback identity. For Codex command approvals it contains the JSON-RPC request ID, item ID, and optional approval ID. The resolver never trusts UI-provided protocol fields outside this encoded identity.

## Protocol decisions

- Harness presents only single-request approve and deny actions.
- Approve maps to `accept`; deny maps to `decline`.
- `acceptForSession`, execution-policy amendments, and network-policy amendments are intentionally not exposed in this phase.
- Missing optional command details remain visible as explicit unavailable values rather than invented summaries.

## Test strategy

- Rust engine tests prove normalization and exact callback responses for command and file approvals.
- Store tests prove kind persistence, uniqueness, decision audit, and restart expiry.
- Frontend tests prove command-specific labels, content, and resolution behavior.
- Engine and frontend tests prove numeric-only usage normalization, per-conversation display, cache-reuse math, and unavailable state.
- Native acceptance uses a disposable clone of the Rails fixture to prove deny leaves it unchanged, approve executes only the displayed command, and restart expires an unanswered request.
- Existing frontend, fixture, Rust, lint, typecheck, format, Clippy, and production build checks remain required.

## Migration / rollout

Existing approvals deserialize as `fileChange`. New clients can render both kinds, while rollback ignores the additional field. No selected repository is mutated during migration.

## Future enhancements

- Friendly rendering of parsed command actions.
- Carefully scoped session approvals after policy and revocation UX exists.
- Diagnostic export and complete local-data removal in Phase 05.
