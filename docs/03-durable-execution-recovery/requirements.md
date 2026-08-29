# Phase 03 — Durable Execution & Recovery Requirements

## Overview

Phase 03 makes execution decisions visible, one-shot, durable, and recoverable. A beta tester must see exactly what a command will do, approve or deny it without granting broader authority, and recover cleanly when a run or the app stops unexpectedly.

## Journey coverage

- Journey 5 — Execute safely and recover
- Product Rule 2 — Agent work begins in an isolated worktree
- Product Rule 4 — Every consequential action is attributable and reviewable
- Acceptance B — Command approval and denial
- Acceptance C — Stop, retry, and restart recovery
- Acceptance D — Concurrency and engine-failure containment

## Glossary

**Execution approval:** A single native decision for one provider-requested command or file change.

**Pending decision:** An approval request whose provider callback has not received an answer.

**Interrupted run:** A run that was active when its supervising process or Harness stopped.

## Requirements

### Req 3.1 — Approve commands explicitly

As a beta tester, I want to inspect a command before it runs so I can prevent unintended execution.

1. Harness SHALL surface Codex command-execution approval requests in the native conversation UI.
2. The decision SHALL display the exact command, working directory, and provider-supplied reason when present.
3. Approve SHALL authorize only the displayed command request; Harness SHALL NOT grant session-wide or persistent policy authority.
4. Deny SHALL return a decline decision and SHALL NOT execute the command.
5. Each decision SHALL be uniquely correlated to its provider request and recorded in the local audit trail.

### Req 3.2 — Preserve and expire decisions safely

As a beta tester, I want pending decisions to fail closed across restart so stale provider callbacks cannot execute later.

1. Pending execution approvals SHALL be persisted with the run before they are presented.
2. Restart SHALL mark approvals belonging to interrupted runs as expired.
3. Expired decisions SHALL remain visible in the audit record but SHALL NOT be answerable.
4. Completed approval decisions SHALL survive restart.

### Req 3.3 — Recover interrupted work

As a beta tester, I want failed or interrupted work to retain its isolated worktree so I can inspect or retry without losing evidence.

1. Stopping a turn SHALL not delete its worktree or recorded events.
2. Runs active during restart SHALL become non-running with an actionable interrupted outcome.
3. A failed or interrupted conversation SHALL support retry using the retained conversation context and worktree.
4. Engine launch or transport failure SHALL remain contained to the affected run and produce actionable status.

### Req 3.4 — Contain concurrent execution

As a project owner, I want concurrent work isolated and bounded so one run cannot silently affect another.

1. Harness SHALL enforce its configured live-run limit before launch.
2. Concurrent runs SHALL use distinct worktrees and maintain distinct events and approval identities.
3. One run's failure, stop, or decision SHALL NOT change another run's lifecycle or worktree.

### Req 3.5 — Report token efficiency honestly

As a beta tester, I want token usage and verified efficiency signals per conversation so I can understand what Rubyn consumed and reused.

1. Harness SHALL display cumulative provider-reported input, output, reasoning, and total tokens for the selected conversation when available.
2. Harness SHALL display cached input tokens reused and their share of reported input tokens as Rubyn efficiency.
3. Harness SHALL identify the metrics as provider-reported and SHALL NOT invent monetary savings.
4. A provider or run without usage telemetry SHALL report usage as unavailable rather than zero.
5. Harness SHALL retain only normalized numeric usage fields and SHALL scrub legacy raw provider diagnostics from primary and backup state.

## Out of scope

- Session-wide command approval
- Persistent execution-policy or network-policy amendments
- Automatic merging after successful execution
- Signed builds and release distribution, assigned to Phase 06
