# Phase 05 — Privacy, Revocation & Diagnostics Tasks

## [x] 1. Revoke model access

- [x] 1.1 Add Rubyn Code provider removal and stored-key deletion with regression tests. (refs Req 5.1, Req 5.4)
- [x] 1.2 Add native API-key and Codex revocation commands that return refreshed readiness. (refs Req 5.1)
- [x] 1.3 Add named confirmation and disconnected-state UX in Models & accounts. (refs Req 5.1)

## [x] 2. Generate safe support diagnostics

- [x] 2.1 Define an allowlisted diagnostic schema with aggregate state and runtime health. (refs Req 5.2, Req 5.4)
- [x] 2.2 Write user-only diagnostic files beneath app data and return the exact path. (refs Req 5.2)
- [x] 2.3 Add a diagnostics control and clear explanation of excluded content. (refs Req 5.2)

## [x] 3. Remove local Rubyn data

- [x] 3.1 Add managed-worktree inventory and active-run preflight. (refs Req 5.3)
- [x] 3.2 Add fail-closed native state removal and in-memory reinitialization. (refs Req 5.3, Req 5.4)
- [x] 3.3 Add destructive confirmation, partial-failure guidance, and first-launch reset. (refs Req 5.3)

## [x] 4. Prove privacy boundaries

- [x] 4.1 Seed credential, path, prompt, output, and source canaries across primary and backup state. (refs Req 5.2, Req 5.4)
- [x] 4.2 Prove generated diagnostics contain allowlisted facts and none of the canaries. (refs Req 5.2, Req 5.4)
- [x] 4.3 Prove revocation blocks future work and data removal preserves source repositories. (refs Req 5.1, Req 5.3)

## [x] 5. Validation

- [x] 5.1 Run frontend, engine, fixture, Rust, lint, typecheck, format, Clippy, and production build checks. (refs Req 5.1, Req 5.2, Req 5.3, Req 5.4)
- [x] 5.2 Run revocation, diagnostic-generation, and local-data-removal native smoke flows against disposable state. (refs Req 5.1, Req 5.2, Req 5.3)
- [x] 5.3 Build the macOS app and DMG, record findings, and confirm source preservation. (refs Req 5.3, Req 5.4)
