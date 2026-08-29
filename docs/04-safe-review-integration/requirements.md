# Phase 04 — Safe Review & Integration Requirements

## Overview

Phase 04 gives every retained worktree an explicit, evidence-backed disposition. A beta tester can distinguish an unchanged run from a changed one, see whether the source repository is still safe to update, deliberately integrate or discard, and recover cleanup without losing the integration record.

## Journey coverage

- Journey 6 — Review and dispose of work
- Product Rule 2 — Agent work begins in an isolated worktree
- Product Rule 4 — Destructive actions require confirmation and an audit record
- Product Rule 5 — Durable state survives restart
- Acceptance A — Successful reviewed integration
- Acceptance E — Source drift or conflict blocks integration
- Acceptance F — Confirmed discard and recoverable cleanup failure

## Glossary

**Source drift:** The source repository HEAD no longer equals the revision from which the run worktree was created.

**Integration readiness:** Current source cleanliness and revision evidence evaluated against the run's recorded base.

**Cleanup pending:** Integration or discard has completed logically, but the managed worktree could not yet be removed.

## Requirements

### Req 4.1 — Present current review evidence

As a project owner, I want review evidence tied to current Git state so I do not act on a stale or misleading diff.

1. Harness SHALL inspect the managed worktree's real status and unified diff, including untracked file contents.
2. Harness SHALL display the recorded base revision, current source revision, source cleanliness, and whether the source still matches the base.
3. Harness SHALL identify truncated diffs and no-change runs explicitly.
4. Harness SHALL recompute integration readiness when review is opened or refreshed.

### Req 4.2 — Integrate only a safe reviewed change

As a project owner, I want integration to fail closed when either repository has changed unexpectedly.

1. Harness SHALL require an explicit confirmation after showing the reviewed file count and source target.
2. Harness SHALL refuse integration when the source index or worktree is dirty, source HEAD differs from the recorded base, the worktree is no longer derived from the base, or the run has no changes.
3. Harness SHALL recheck every safety condition in the native integration operation rather than trusting webview state.
4. A failed or conflicting integration SHALL abort any partial source operation, retain the managed worktree and review evidence, and present an actionable reason.
5. A successful integration SHALL record the resulting commit before attempting cleanup.

### Req 4.3 — Dispose and recover cleanup deliberately

As a project owner, I want discard and cleanup recovery to be explicit so evidence is not silently lost.

1. Discard SHALL require confirmation that identifies the managed worktree and changed-file count.
2. Discard SHALL remove only the Harness-managed worktree and SHALL NOT modify the source repository.
3. Cleanup failure after integration or discard SHALL preserve the completed disposition with a cleanup-pending state.
4. Harness SHALL offer an explicit retry-cleanup action that preserves whether the run was integrated or discarded.
5. Successful cleanup retry SHALL be idempotent from the user's perspective and SHALL create a durable audit event.

### Req 4.4 — Preserve disposition across restart

As a beta tester, I want disposition state to survive interruption so I can safely resume review or cleanup.

1. Interrupted integration SHALL return to retained review with an audit event.
2. Integrated, discarded, and cleanup-pending states SHALL survive restart.
3. Terminal disposition SHALL prevent later integration or discard from being applied again.

## Out of scope

- Automatic rebasing or merging across source drift
- Partial-file or partial-hunk integration
- Running repository hooks during integration
- Remote branch pushes or pull-request creation
