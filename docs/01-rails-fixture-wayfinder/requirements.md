# Phase 01 — Rails Fixture & Wayfinder Proof Requirements

## Overview

Phase 01 creates a reproducible Rails acceptance project and uses it to prove the current native Wayfinder-to-integration journey. The source `Rubyn-AI/rubyn-test` repository remains unchanged; each run starts from a pinned disposable clone.

## Journey coverage

- Journey 2 — Open a trusted project
- Journey 4 — Shape work with Wayfinder
- Journey 5 — Execute safely and recover
- Journey 6 — Review and dispose of work
- Product Rule 7 — Disposable acceptance clones
- Acceptance A — Successful Wayfinder-to-integration flow

## Glossary

**Fixture source:** The read-only `Rubyn-AI/rubyn-test` repository at an explicitly pinned commit.

**Acceptance clone:** A newly created local clone used for exactly one manual or automated acceptance run.

**Proof scenario:** A bounded improvement to the fixture with an observable test and review outcome.

## Requirements

### Req 1.1 — Prepare a safe acceptance clone

As a Harness developer, I want a deterministic fixture-preparation command so that acceptance runs never contaminate the source repository.

1. The command SHALL clone `Rubyn-AI/rubyn-test` at a recorded commit into a newly created destination.
2. The command SHALL refuse an existing or ambiguous destination.
3. The command SHALL create a local acceptance branch suitable for Harness worktrees and integration.
4. The command SHALL disable pushing from the acceptance clone.
5. The command SHALL NOT copy untracked source files, `.env`, logs, caches, local databases, or prior Rubyn state.
6. The command SHALL print the created project path and pinned source commit.
7. Repeated invocations SHALL create independent clones with the same tracked baseline.
8. The acceptance baseline SHALL remove fixture-only local path dependencies that cannot resolve inside Harness worktrees.

### Req 1.2 — Verify fixture readiness

As a beta operator, I want the fixture checked before opening it in Harness so failures are attributable to Rubyn rather than a broken baseline.

1. Verification SHALL confirm that the clone is a Git repository on the acceptance branch and has no push-capable remote.
2. Verification SHALL confirm Rails application markers and the expected pinned commit.
3. Verification SHALL fail if tracked environment-secret files are present.
4. Verification SHALL run the fixture's baseline Rails tests when the required Ruby dependencies are available.
5. Missing local dependencies SHALL produce an actionable skipped result rather than silently passing.

### Req 1.3 — Define one deterministic Wayfinder proof

As a product owner, I want one bounded scenario so that Wayfinder behavior can be evaluated consistently.

1. The scenario SHALL begin with a user-observable Destination rather than an implementation instruction.
2. The scenario SHALL contain enough known Rails defects to require at least one Code Ticket and an automated regression test.
3. The expected scope and explicit non-goals SHALL be documented before agent execution.
4. The scenario SHALL identify the evidence required before integration.

### Req 1.4 — Complete the native vertical journey

As a beta tester, I want to move from a Wayfinder Destination to an integrated Rails change so that the product's central promise is proven.

1. Harness SHALL open the prepared acceptance clone as a Rails project.
2. The tester SHALL be able to create and approve the proof scenario's Wayfinder Map and Graph Delta.
3. An unblocked Code Ticket SHALL create or link to a Board Task with its Brief Version preserved.
4. The resulting Background Run SHALL execute in an isolated worktree.
5. Proposed file changes SHALL require explicit approval.
6. The Review view SHALL show the resulting Git status and diff.
7. Integration SHALL update only the disposable acceptance clone and SHALL retain an auditable run disposition.
8. The fixture regression test SHALL pass after integration.

### Req 1.5 — Preserve repeatable evidence

As a release operator, I want durable proof artifacts so later phases can rerun and compare the journey.

1. Phase documentation SHALL record the fixture source, pinned commit, scenario, commands, and manual observations.
2. Automated checks SHALL cover clone safety and fixture validation without network access.
3. The manual smoke record SHALL distinguish product defects, environment failures, and fixture failures.

## Out of scope

- Editing or cleaning `/Users/fadedmaturity/rubyn-test`
- Publishing changes to `Rubyn-AI/rubyn-test`
- General-purpose fixture management
- Fully deterministic model output
- Failure and restart matrices assigned to Phases 03 and 04
