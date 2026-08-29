# External macOS Beta User Journeys

Approved by the product owner on 2026-08-29.

## Journey 1 — Install and leave cleanly

**Actor:** Beta tester

1. The tester receives a signed and notarized macOS beta.
2. They install and open Rubyn Harness without bypassing operating-system security warnings.
3. Rubyn explains its local repository access, model-provider requirements, and trusted-repository limitation.
4. The tester can quit, uninstall, and remove Rubyn's local application data.

**Outcome:** The tester understands the trust boundary and can leave without unexplained retained data.

## Journey 2 — Open a trusted project

**Actor:** Rails or Ruby project owner

1. The owner selects a local Git repository.
2. Rubyn validates that the path is accessible and identifies whether it is Ruby or Rails.
3. A valid project opens with its persisted workspace. Invalid, missing, or inaccessible paths produce actionable guidance.
4. The owner can switch projects without mixing their project-scoped state.

**Outcome:** Rubyn operates only on an explicitly selected, understood workspace.

## Journey 3 — Connect and revoke a model provider

**Actor:** Beta tester

1. The tester connects Codex or configures a supported API-key provider.
2. Rubyn reports whether the provider and selected model are ready.
3. The tester can select a connected model and run work.
4. The tester can revoke a provider; future work is blocked until another ready model is selected.

**Outcome:** Model access is understandable and reversible without exposing credentials in project data or logs.

## Journey 4 — Shape work with Wayfinder

**Actor:** Project owner

1. The owner creates a Wayfinder Map with an observable Destination.
2. Rubyn and the owner resolve Fog through Grill, Research, Prototype, Code, and User Action Tickets as needed.
3. Graph Deltas require approval before they change the Map.
4. The owner edits, archives, or resumes Maps and their Tickets after restart.
5. An unblocked Code Ticket becomes a Board Task with an immutable Brief Version.

**Outcome:** The owner can turn uncertain product intent into explicit, reviewable execution work.

## Journey 5 — Execute safely and recover

**Actor:** Project owner supervising Rubyn

1. The owner launches a Conversation or ready Board Task in an isolated worktree.
2. Rubyn streams understandable progress and requests approval for each proposed repository edit.
3. The owner approves or denies edits, attaches relevant files, queues guidance, or stops the current turn.
4. Provider failure, engine failure, cancellation, application restart, and stale state preserve enough context to retry or dispose of the work safely.

**Outcome:** Agent execution is inspectable, interruptible, and recoverable without silently changing the source checkout.

## Journey 6 — Review and dispose of work

**Actor:** Project owner

1. The owner sees the real Git status and unified diff for a retained worktree.
2. Rubyn prevents integration when the source repository or worktree is unsafe or stale.
3. The owner deliberately integrates acceptable changes or confirms discard.
4. Conflicts and cleanup failures retain evidence and present a recoverable next action.

**Outcome:** Every worktree reaches an explicit, audited disposition without damaging the source repository.

## Journey 7 — Diagnose and upgrade

**Actor:** Beta tester or support operator

1. The tester can distinguish project, runtime, provider, and application failures.
2. They can produce sanitized diagnostics that contain no credentials or unrelated source content.
3. They install a newer signed beta manually.
4. Compatible project and application state survives the upgrade; incompatible state fails with recovery guidance.

**Outcome:** Beta failures can be supported and upgrades do not strand the tester.

## Product rules

1. Repository edits require explicit human approval.
2. Agent work begins in an isolated worktree, never the source checkout.
3. Credentials remain in operating-system-protected storage and never appear in project data or diagnostics.
4. Destructive actions require confirmation and create an audit record.
5. Durable state survives restart; stale state is detected and recoverable.
6. The first beta is single-user, local-first, macOS-only, and limited to repositories the tester trusts.
7. Acceptance tests use disposable clones and never modify their source repository.

## Acceptance journeys

- **Acceptance A:** Clean install, trusted Rails project selection, provider readiness, Wayfinder Map creation, approved Code Ticket, Background Run, edit approval, diff review, and successful integration.
- **Acceptance B:** Denied edit followed by continued or safely failed execution with an audit record.
- **Acceptance C:** Forced application restart with a retained Conversation, worktree, events, and correctly expired or restored approval state.
- **Acceptance D:** Provider failure and retry with a connected model.
- **Acceptance E:** Source-repository drift or merge conflict that blocks integration while retaining the worktree and evidence.
- **Acceptance F:** Confirmed discard and recoverable worktree-cleanup failure.
- **Acceptance G:** Credential revocation, sanitized diagnostic creation, and manual beta upgrade with compatible state retained.

## Explicit non-goals for the first beta

- Windows or Linux distribution
- Execution against untrusted repositories
- Multi-user collaboration or remote workspace ownership
- Unattended scheduling or autonomous background operation
- Plugin marketplace distribution
- Automatic application updates
