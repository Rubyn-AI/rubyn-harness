# Phase 01 — Rails Fixture & Wayfinder Proof Design

## Overview

Phase 01 adds a small repository script for creating a pinned, non-pushable acceptance clone and an offline integration test for its safety invariants. A documented proof scenario then drives the existing native product from Wayfinder through integration.

## Architecture

### Fixture preparation script

**Responsibility:** Resolve an explicit source and destination, clone the pinned commit, remove generated state and the fixture-only `../rubyn` path dependency, create an acceptance branch, disable pushes, and report the result.

**Collaborators:** `git`, the local filesystem, and the pinned fixture metadata.

**Why not inline?** The same safety-critical preparation must be repeatable by developers, CI setup, and every manual beta acceptance pass.

### Fixture metadata

**Responsibility:** Record the public source URL, pinned commit, expected Rails markers, and acceptance branch name in reviewable repository data.

**Collaborators:** Fixture preparation and verification.

**Why not inline?** A standalone record makes source upgrades auditable without editing shell logic.

### Offline safety test

**Responsibility:** Create a temporary local Git source, invoke fixture preparation without network access, and assert source immutability, exact revision, destination refusal, secret exclusion, clean state, and disabled pushes.

**Collaborators:** Fixture preparation script and a temporary Git repository.

**Why not inline?** It independently exercises destructive-safety boundaries that shell inspection alone cannot prove.

### Wayfinder proof record

**Responsibility:** Describe the Destination, boundaries, expected Tickets, regression evidence, and observed native outcome.

**Collaborators:** Rubyn Harness, the disposable acceptance clone, and the fixture test suite.

**Why not inline?** Later phases need a stable semantic acceptance anchor even as implementation details change.

## Data model changes

No Harness persistence schema changes are planned. The acceptance clone stores its fixture marker in local Git configuration so the worktree remains clean.

Fixture metadata is repository-controlled and contains no credentials or machine-specific source path.

## Native lifecycle and approval hardening

The proof exposed two gaps in the existing native journey, so Phase 01 also tightens the execution adapter without changing the persisted schema:

- A completed Codex turn that is waiting for another message can be explicitly finished. The engine records it as completed, making its isolated worktree reviewable rather than misclassifying the user action as cancellation.
- Codex runs use the app-server's on-request approval policy with a read-only sandbox. File-change approval requests are translated into the Harness edit-approval model and the user's decision is returned to Codex.
- Approval identities contain both the JSON-RPC request ID and the unique Codex item ID. This prevents a retried edit from disappearing when Codex reuses a request ID after a denial.

The native adapter intentionally keeps the approval translation at the engine boundary. Existing Review and approval UI semantics remain provider-independent.

## Test strategy

- Run the fixture-preparation safety test entirely against temporary local repositories.
- Run existing frontend and Rust suites to guard the Harness baseline.
- Prepare a real clone from the pinned GitHub source and run its Rails baseline when dependencies are available.
- Use the packaged native app for the Wayfinder proof because live Vite development mode is currently unreliable on this machine.
- Record each manual checkpoint: project detection, Map persistence, Graph Delta approval, task handoff, worktree isolation, edit approval, Review diff, integration, and regression test.
- Probe denial and retry independently from the feature change so a denied edit can be proven to leave the worktree clean.

## Migration / rollout

There is no data migration. The script only creates new directories and refuses to overwrite destinations. Acceptance clones are disposable; removal remains an explicit operator action.

Rollback consists of removing the new script, metadata, safety test, and documentation. No source fixture or Harness user data is mutated by rollback.

## Future enhancements

- CI-hosted acceptance clones once model-backed native testing is available.
- Multiple pinned Rails versions after the first beta journey is stable.
- Failure injection for provider, engine, restart, merge-conflict, and cleanup scenarios in later phases.
- Redacted, bounded provider diagnostics instead of persisting raw app-server transport.
- Codex command-approval bridging for write-producing verification commands under the read-only sandbox.
