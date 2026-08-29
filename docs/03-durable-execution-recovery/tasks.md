# Phase 03 — Durable Execution & Recovery Tasks

## [x] 1. Normalize command approvals

- [x] 1.1 Parse Codex command-execution approval requests and preserve unique callback identity. (refs Req 3.1)
- [x] 1.2 Emit exact command, working directory, and optional reason without broadening authority. (refs Req 3.1)
- [x] 1.3 Return one-shot accept or decline decisions through the existing engine supervisor. (refs Req 3.1)

## [x] 2. Persist and audit execution decisions

- [x] 2.1 Add a backward-compatible approval-kind discriminator. (refs Req 3.1, Req 3.2)
- [x] 2.2 Persist command requests from normalized events and reject duplicate or stale resolution. (refs Req 3.1, Req 3.2)
- [x] 2.3 Prove completed decisions persist and interrupted pending decisions expire. (refs Req 3.2)

## [x] 3. Present kind-aware native decisions

- [x] 3.1 Render exact command context and command-specific risk language. (refs Req 3.1)
- [x] 3.2 Keep file-change decision behavior and language intact. (refs Req 3.1)
- [x] 3.3 Cover approval, denial, and failure feedback in frontend tests. (refs Req 3.1, Req 3.2)

## [x] 4. Prove recovery and containment

- [x] 4.1 Prove stopped and interrupted runs retain worktrees and events. (refs Req 3.3)
- [x] 4.2 Prove retry and engine failure remain scoped to the affected run. (refs Req 3.3)
- [x] 4.3 Prove live-run limits and concurrent worktree/approval isolation. (refs Req 3.4)

## [x] 5. Validation

- [x] 5.1 Run frontend, fixture, Rust, lint, typecheck, format, Clippy, and production build checks. (refs Req 3.1, Req 3.2, Req 3.3, Req 3.4)
- [x] 5.2 Run native deny, approve, restart-expiry, retry, and concurrency smoke flows on the disposable Rails fixture. (refs Req 3.1, Req 3.2, Req 3.3, Req 3.4)
- [x] 5.3 Record findings and confirm the fixture source remains unchanged. (refs Req 3.3, Req 3.4)

## [x] 6. Token usage and efficiency

- [x] 6.1 Normalize numeric cumulative token usage without retaining raw account telemetry. (refs Req 3.5)
- [x] 6.2 Show selected-conversation usage and cached-token reuse with honest unavailable states. (refs Req 3.5)
- [x] 6.3 Cover usage isolation and efficiency calculations in native and frontend tests. (refs Req 3.5)
- [x] 6.4 Withhold raw provider frames and migrate legacy diagnostics out of primary and backup state. (refs Req 3.5)
