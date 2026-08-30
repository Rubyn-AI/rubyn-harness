# Rubyn Harness Beta Roadmap

Rubyn Harness is targeting an external macOS beta for trusted local Ruby and Rails repositories. Every phase is a mergeable vertical slice and must leave the application usable.

## Phases

- [x] **01 — Rails Fixture & Wayfinder Proof:** Prepare a pinned disposable clone of `Rubyn-AI/rubyn-test` and prove a complete Wayfinder-to-integration journey.
- [x] **02 — Beta Onboarding & Trust Gate:** Add first-launch trust disclosure, repository confirmation, prerequisite checks, and actionable setup failures.
- [x] **03 — Durable Execution & Recovery:** Prove edit approvals, stop/retry, concurrency, restart, engine failure, and stale-state recovery.
- [x] **04 — Safe Review & Integration:** Handle repository drift, conflicts, no-change runs, integration, discard, and cleanup recovery.
- [x] **05 — Privacy, Revocation & Diagnostics:** Add credential revocation, local-data removal, secret-safe logs, and sanitized diagnostics.
- [ ] **06 — Signed Beta Distribution:** Pin toolchains and produce versioned, signed, notarized macOS application and DMG releases.
- [ ] **07 — Beta Acceptance & Hardening:** Run every approved journey from a clean install and close accessibility, performance, and release-checklist gaps.

## Working agreement

- The approved product contract is [user-journeys.md](user-journeys.md).
- Only the current phase is fully scaffolded; later phases remain one-line outcomes until promoted.
- Each phase includes automated validation and a manual native smoke flow.
- Acceptance runs use disposable clones. The source `rubyn-test` checkout is never modified.
