# Phase 02 Native Smoke Record

## Run identity

- Date: 2026-08-29
- Build: packaged native debug `.app` and DMG, version `0.1.15`
- Project: `/private/tmp/rubyn-harness-acceptance-20260829-v2`
- Project kind: Rails

## Native flow

1. Launched the upgraded packaged app with legacy local state that had no onboarding or trust fields.
2. Verified the versioned disclosure blocked the workspace and explained isolated worktrees, explicit edit approval, provider access, repository trust, and local state.
3. Acknowledged the disclosure and verified the Projects view reported the existing recent Rails project needed one trust confirmation after upgrade.
4. Opened the recent project and verified the confirmation displayed its canonical path, Rails kind, Git root, Rubyn-instruction status, native-engine readiness, and provider readiness.
5. Cancelled. Persisted `trustedProjectPaths` remained empty and the project did not open.
6. Reopened the confirmation and selected **Trust and open**. The canonical path became the only trusted path and the Rails workspace opened.
7. Quit and relaunched the packaged app. The disclosure and trust prompt did not repeat, and the trusted recent project reopened successfully.

## Automated evidence

- Frontend: 53 tests passed, including disclosure persistence, cancellation, trust-before-load ordering, and non-Git rejection.
- Rust: 48 tests passed, including legacy-state defaults, trust normalization, and exact-path matching.
- Fixture safety: 2 tests passed.
- ESLint, TypeScript, Rustfmt, and Clippy passed.
- Vite production build passed.
- Native `.app` and DMG packaging passed.
- Known performance note: the main JavaScript chunk is 831.05 kB after minification.

## Safety confirmation

The original `/Users/fadedmaturity/rubyn-test` checkout remained at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3` with its pre-existing modified and untracked files unchanged. The trust flow only stored the disposable clone's canonical path in Harness local application state.

## Phase result

Phase 02 passed. First contact is explicit, repository trust is inspectable and cancel-safe, native project loading and agent launch fail closed for untrusted paths, and acknowledged trust survives restart.
