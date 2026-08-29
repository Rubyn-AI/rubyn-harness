# Phase 02 — Beta Onboarding & Trust Gate Design

## Overview

Phase 02 adds a versioned first-launch disclosure and a two-step inspect/confirm project flow. Trust remains local application state; canonical project inspection remains native so rendered UI cannot invent repository identity.

## Architecture

### Versioned onboarding state

**Responsibility:** Persist the latest acknowledged disclosure version with the existing local application preferences.

**Collaborators:** Native state repository, bridge types, and the root application shell.

**Why not inline?** Versioning is the durable contract that determines whether a future disclosure must be shown again.

### Project trust registry

**Responsibility:** Persist canonical trusted project paths and answer whether a recently selected project may open directly.

**Collaborators:** Native path inspection and local application state.

**Why not inline?** Trust must survive restart and stay separate from project-owned files.

### Trust confirmation dialog

**Responsibility:** Present native inspection results and require an explicit decision before project state is loaded or persisted.

**Collaborators:** Project picker, recent projects, and readiness presentation.

**Why not inline?** The same confirmation is used by the Projects page and global project switcher and must enforce identical semantics.

## Data model changes

`LocalAppState` gains `onboardingVersion` and `trustedProjectPaths`. Deserialization defaults preserve existing beta state. Normalization trims, deduplicates, and bounds trusted paths.

No project database schema changes are required. Trust is device-local and never written into a selected repository.

## Test strategy

- Rust unit tests prove legacy state defaults and trusted-path normalization.
- Frontend tests prove first-launch disclosure, version persistence, inspect-before-open, cancellation, confirmation, and trusted recent-project reopening.
- Existing native, frontend, fixture, lint, typecheck, and packaging checks remain required.
- Manual smoke starts with cleared local state and uses the disposable Rails acceptance clone.

## Migration / rollout

Existing state without onboarding or trust fields loads with safe defaults. Existing recent projects require one explicit trust decision after upgrade.

Rollback ignores the additional JSON fields. No repository or credential data is changed.

## Future enhancements

- Trust revocation and complete local-data removal in Phase 05.
- Richer detection of repository-defined execution surfaces once command approvals are bridged.
- Signed-build identity and notarization details in the disclosure during Phase 06.
