# Phase 02 — Beta Onboarding & Trust Gate Requirements

## Overview

Phase 02 makes the first contact with Rubyn Harness explicit and safe. A beta tester must understand the local trust boundary, see prerequisite readiness, and deliberately trust each repository before Harness persists or operates on it.

## Journey coverage

- Journey 1 — Install and leave cleanly
- Journey 2 — Open a trusted project
- Journey 3 — Connect and revoke a model provider
- Product Rule 2 — Agent work begins in an isolated worktree
- Product Rule 6 — Trusted repositories only
- Acceptance A — Clean install through trusted project selection and provider readiness

## Glossary

**First-launch disclosure:** The versioned explanation of repository access, isolated worktrees, provider requirements, and local state shown before the workspace UI.

**Trust decision:** A deliberate, locally persisted confirmation for one canonical Git repository path.

**Readiness check:** An observable prerequisite result with a state and actionable remediation.

## Requirements

### Req 2.1 — Explain the beta trust boundary

As a first-time beta tester, I want a concise disclosure so I understand what Rubyn can access before selecting a repository.

1. Harness SHALL show the disclosure before the normal workspace on first launch.
2. The disclosure SHALL explain local repository access, isolated worktrees, explicit edit approval, model-provider access, trusted repositories, and local application state.
3. Continuing SHALL persist the disclosure version locally so the same version is not shown on every launch.
4. A newer disclosure version SHALL require acknowledgement again.

### Req 2.2 — Inspect before trusting a repository

As a project owner, I want to inspect a selected path before trusting it so an accidental folder selection cannot immediately become an execution workspace.

1. Harness SHALL canonicalize and inspect a selected path before presenting a trust decision.
2. The confirmation SHALL display the canonical path, detected project kind, Git root, and whether Rubyn instructions are present.
3. Harness SHALL NOT persist or open the project until the owner confirms trust.
4. Cancelling SHALL leave the active project and recent-project list unchanged.
5. Trust SHALL be scoped to the canonical repository path and persisted locally.
6. Previously trusted recent projects MAY reopen without a repeated confirmation.

### Req 2.3 — Report actionable readiness

As a beta tester, I want setup failures classified so I know what to fix next.

1. Harness SHALL report native engine readiness independently from project and provider readiness.
2. A selected project SHALL report Git, Ruby/Rails detection, and provider readiness.
3. Missing or inaccessible paths SHALL produce guidance that identifies the failed path.
4. A project that is not a Git repository SHALL be rejected before trust can be confirmed.
5. Agent launch SHALL remain unavailable when the native engine or selected provider is not ready.

## Out of scope

- Sandboxing repositories the tester does not trust
- Automatic provider credential creation
- Credential revocation and diagnostic export, assigned to Phase 05
- Signed installer presentation, assigned to Phase 06
- Removing all local application data, assigned to Phase 05
