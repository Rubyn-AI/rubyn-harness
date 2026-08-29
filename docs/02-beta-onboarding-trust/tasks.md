# Phase 02 — Beta Onboarding & Trust Gate Tasks

## [x] 1. Durable disclosure and trust state

- [x] 1.1 Add versioned onboarding acknowledgement with backward-compatible defaults. (refs Req 2.1)
- [x] 1.2 Add normalized, bounded canonical trusted-project paths. (refs Req 2.2)
- [x] 1.3 Cover state migration and normalization in Rust tests. (refs Req 2.1, Req 2.2)

## [x] 2. First-launch disclosure

- [x] 2.1 Block the workspace behind the disclosure when its version is unacknowledged. (refs Req 2.1)
- [x] 2.2 Explain repository, worktree, approval, provider, and local-state boundaries. (refs Req 2.1)
- [x] 2.3 Persist acknowledgement and add frontend coverage. (refs Req 2.1)

## [x] 3. Inspect and confirm project trust

- [x] 3.1 Share a two-step native inspection and confirmation flow across project entry points. (refs Req 2.2)
- [x] 3.2 Show canonical identity and readiness without mutating recent or active project state. (refs Req 2.2, Req 2.3)
- [x] 3.3 Persist confirmation, support cancellation, and reopen trusted recent projects. (refs Req 2.2)

## [x] 4. Readiness and failure guidance

- [x] 4.1 Reject non-Git selections before trust confirmation. (refs Req 2.3)
- [x] 4.2 Present engine, project, and provider readiness independently. (refs Req 2.3)
- [x] 4.3 Add actionable invalid, missing, and inaccessible path tests. (refs Req 2.3)

## [x] 5. Validation

- [x] 5.1 Run frontend, fixture, Rust, lint, typecheck, format, Clippy, and production build checks. (refs Req 2.1, Req 2.2, Req 2.3)
- [x] 5.2 Run the native first-launch and disposable Rails trust smoke flow. (refs Req 2.1, Req 2.2, Req 2.3)
- [x] 5.3 Record findings and confirm the fixture source remains unchanged. (refs Req 2.2)
