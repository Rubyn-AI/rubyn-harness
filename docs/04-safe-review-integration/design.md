# Phase 04 — Safe Review & Integration Design

## Overview

Phase 04 extends the existing native Git review path with a shared readiness preflight, confirmation-first UI, and disposition-preserving cleanup recovery. The source checkout remains the integration target; managed worktrees remain the only disposable paths.

## Architecture

### Native integration readiness

**Responsibility:** Read source HEAD and status, compare them with the run's recorded base, and return structured blockers.

**Collaborators:** Existing Git command helpers, `inspect_run_worktree`, and `integrate_isolated_worktree`.

**Why not inline?** Review display and the authoritative integration operation must apply the same rules while integration still performs a fresh check.

### Worktree disposition service

**Responsibility:** Integrate or remove only validated managed worktrees and abort partial cherry-picks.

**Collaborators:** State repository lifecycle transitions and the Tauri command boundary.

**Why not inline?** Filesystem and Git containment remain native-only and already have two command callers.

### Review surface

**Responsibility:** Present diff, changed files, source readiness, confirmations, blockers, and cleanup recovery.

**Collaborators:** Typed bridge records and project refresh.

**Why not inline?** This remains the existing `Review` product surface; no new frontend service is introduced.

## Data model changes

`RunWorktreeInspection` gains structured integration readiness containing source HEAD, recorded base, source cleanliness, base match, and blocker messages. This evidence is ephemeral and recomputed; it is not persisted as truth.

Run lifecycle values remain compatible: `retained`, `integrating`, `integrated`, `integrated_cleanup_pending`, `discarded`, and `discard_cleanup_pending`. Cleanup retry transitions only the two pending values to their corresponding completed value and records an audit event.

## Test strategy

- Native service tests cover clean integration, dirty source, source drift, no-change worktrees, conflict/race containment, hook suppression, and managed-only discard.
- Store tests cover cleanup completion, disposition preservation, invalid retries, and interrupted integration recovery.
- Frontend tests cover readiness blockers, integration confirmation, no-change state, discard confirmation, and both cleanup-pending dispositions.
- Native smoke uses a disposable Rails clone to prove successful integration, drift blocking with retained evidence, no-change disposal, and cleanup recovery behavior.
- The full frontend, fixture, Rust, lint, typecheck, format, Clippy, app, and DMG matrix remains required.

## Migration / rollout

No destructive migration is required. Existing lifecycle strings remain valid. The new inspection fields are produced at runtime, and cleanup retry operates on existing cleanup-pending records.

## Future enhancements

- Guided rebase after source drift
- Partial integration
- Remote publishing and pull-request workflows
