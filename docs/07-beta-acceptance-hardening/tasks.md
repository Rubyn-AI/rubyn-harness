# Phase 07 — Beta Acceptance & Hardening Tasks

## [x] 1. Establish the acceptance harness

- [x] 1.1 Record exact app, engine, fixture, schema, and clean-state identifiers. (refs Req 7.1, Req 7.4)
- [x] 1.2 Make Acceptances A–G repeatable with isolated app data and disposable clones. (refs Req 7.1)
- [x] 1.3 Prove the original fixture and real application data remain unchanged. (refs Req 7.1)

## [ ] 2. Enforce accessibility

- [x] 2.1 Add automated accessibility checks for first launch and the primary empty-project shell. (refs Req 7.2)
- [ ] 2.2 Cover runtime failure, trust, provider, conversation, review, and confirmation surfaces. (refs Req 7.2)
- [ ] 2.3 Complete keyboard, contrast, reduced-motion, and macOS assistive-technology acceptance. (refs Req 7.2)

## [ ] 3. Enforce performance budgets

- [x] 3.1 Add deterministic production-asset budgets. (refs Req 7.3)
- [ ] 3.2 Record clean-launch and project-open latency on supported hardware. (refs Req 7.3)
- [ ] 3.3 Exercise bounded retained history and polling without unbounded growth. (refs Req 7.3)

## [ ] 4. Prove manual upgrade safety

- [x] 4.1 Create a sanitized prior-version state fixture. (refs Req 7.4)
- [x] 4.2 Prove compatible state survives replacement and newer/invalid state fails with recovery guidance. (refs Req 7.4)
- [ ] 4.3 Complete the native A–G matrix and publish the final beta checklist. (refs Req 7.1, Req 7.4)

## [x] 5. Make token efficiency auditable

- [x] 5.1 Persist cumulative provider token usage as privacy-minimized run events. (refs Req 7.5)
- [x] 5.2 Measure Rubyn tool-output and context-compaction savings per chat. (refs Req 7.5)
- [x] 5.3 Show usage and savings in both the chat header and retained run Review. (refs Req 7.5)
