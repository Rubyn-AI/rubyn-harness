# Phase 04 — Safe Review & Integration Tasks

## [x] 1. Make review evidence freshness visible

- [x] 1.1 Add a shared native integration-readiness preflight. (refs Req 4.1, Req 4.2)
- [x] 1.2 Return source revision, cleanliness, base match, and blockers with worktree inspection. (refs Req 4.1)
- [x] 1.3 Present readiness, truncation, and no-change states in Review. (refs Req 4.1)

## [x] 2. Fail closed during integration

- [x] 2.1 Require a review-specific integration confirmation. (refs Req 4.2)
- [x] 2.2 Recheck source cleanliness, source drift, worktree ancestry, and changes natively. (refs Req 4.2)
- [x] 2.3 Retain evidence and return actionable errors after blocked or aborted integration. (refs Req 4.2, Req 4.4)

## [x] 3. Recover worktree cleanup

- [x] 3.1 Include worktree identity and changed-file count in discard confirmation. (refs Req 4.3)
- [x] 3.2 Add disposition-preserving retry cleanup for integrated and discarded runs. (refs Req 4.3)
- [x] 3.3 Persist cleanup completion and reject duplicate or invalid disposition actions. (refs Req 4.3, Req 4.4)

## [x] 4. Prove durable dispositions

- [x] 4.1 Cover successful integration, dirty source, drift, no-change, and hook suppression. (refs Req 4.2)
- [x] 4.2 Cover cleanup-pending recovery and interrupted integration lifecycle. (refs Req 4.3, Req 4.4)
- [x] 4.3 Cover confirmation, blocker, no-change, and cleanup recovery UI. (refs Req 4.1, Req 4.2, Req 4.3)

## [x] 5. Validation

- [x] 5.1 Run frontend, fixture, Rust, lint, typecheck, format, Clippy, and production build checks. (refs Req 4.1, Req 4.2, Req 4.3, Req 4.4)
- [x] 5.2 Run successful integration, source-drift blocker, no-change discard, and cleanup-recovery native smoke flows on the disposable Rails fixture. (refs Req 4.1, Req 4.2, Req 4.3, Req 4.4)
- [x] 5.3 Build the macOS app and DMG, record findings, and confirm the fixture source contract. (refs Req 4.2, Req 4.3)
