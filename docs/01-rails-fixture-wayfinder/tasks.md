# Phase 01 — Rails Fixture & Wayfinder Proof Tasks

## [x] 1. Acceptance contract

- [x] 1.1 Record the approved journeys and beta roadmap. (refs Req 1.5)
- [x] 1.2 Record pinned fixture metadata without machine-specific paths or secrets. (refs Req 1.1, Req 1.5)
- [x] 1.3 Define the bounded Rails proof scenario and required evidence. (refs Req 1.3)

## [x] 2. Disposable fixture tooling

- [x] 2.1 Implement a strict fixture-preparation command that refuses existing destinations. (refs Req 1.1)
- [x] 2.2 Pin the exact source revision, create an acceptance branch, and disable pushes. (refs Req 1.1)
- [x] 2.3 Add actionable readiness verification and optional Rails baseline execution. (refs Req 1.2)

## [x] 3. Safety automation

- [x] 3.1 Add an offline integration test using a temporary local source repository. (refs Req 1.5)
- [x] 3.2 Prove source immutability, secret exclusion, exact revision, clean clone state, and disabled pushes. (refs Req 1.1, Req 1.2)
- [x] 3.3 Add the fixture safety test to the normal repository validation command or CI. (refs Req 1.5)

## [x] 4. Native Wayfinder proof

- [x] 4.1 Prepare and verify a real disposable `rubyn-test` clone. (refs Req 1.1, Req 1.2)
- [x] 4.2 Create and approve the proof Map and Graph Delta in the packaged native app. (refs Req 1.3, Req 1.4)
- [x] 4.3 Launch the Code Ticket, resolve edit approvals, and verify worktree isolation. (refs Req 1.4)
- [x] 4.4 Inspect the diff, integrate it into the disposable clone, and run its regression test. (refs Req 1.4)

## [x] 5. Validation

- [x] 5.1 Run fixture safety automation, frontend tests, Rust tests, lint, typecheck, format, Clippy, and production build. (refs Req 1.5)
- [x] 5.2 Record the manual smoke flow and classify every observed failure. (refs Req 1.5)
- [x] 5.3 Confirm `/Users/fadedmaturity/rubyn-test` remains at its original status and revision. (refs Req 1.1)
