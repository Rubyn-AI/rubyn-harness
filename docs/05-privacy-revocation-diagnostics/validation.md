# Phase 05 Validation

Validated on macOS on 2026-08-29 against the packaged 0.1.15 application.

## Automated checks

- Frontend: 63 tests passed, including preset and custom-provider revocation, exact diagnostic paths, and destructive local-data confirmation.
- Rust: 60 tests passed, including diagnostic canaries, managed-worktree inventory, and temporary app-data override safety.
- Rubyn Code: 2,872 examples passed; the focused provider-revocation suite passed 73 examples. Coverage was 91.39%.
- Fixture safety: 2 tests passed.
- ESLint, TypeScript, Vite production build, Rust formatting, strict Clippy, and `git diff --check` passed.
- The macOS application and `Rubyn Harness_0.1.15_aarch64.dmg` were built successfully.

## Native privacy smoke

The packaged application ran with `RUBYN_HARNESS_TEST_APP_DATA_DIR` pointing to the guarded temporary directory `/private/tmp/rubyn-harness-test-phase5-20260829`. The override rejected relative paths, traversal, unrelated leaf names, and symlink targets in unit tests.

The app generated a diagnostic report beneath that directory, displayed its exact path, and created it with mode `0600`. Inspection found only app/runtime versions, engine health, provider/model counts, and aggregate durable-state counts. It contained no source paths, prompts, model output, diffs, attachments, credential fields, or token values.

The local-data flow displayed a destructive confirmation, removed the diagnostic and both primary/backup state, left no staging directory, created only a fresh database, and returned the app to the first-launch trust disclosure.

## Native revocation smoke

A disposable custom provider named `rubyn-phase5-smoke` was connected with a fake key. Native testing exposed and fixed a missing custom-provider account card. The rebuilt app displayed the provider as connected, showed a confirmation naming it exactly, revoked it, refreshed the connected count from four to three, and removed its name from both `~/.rubyn-code/config.yml` and `~/.rubyn-code/tokens.yml`.

Codex logout was not invoked during acceptance because it would revoke the tester's real session. Its subprocess behavior and refreshed readiness are covered by native command code and frontend bridge tests.

## Source preservation

No source repository was selected during the destructive-data smoke. The original `/Users/fadedmaturity/rubyn-test` checkout remained at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3` with its pre-existing generated modifications unchanged (`Gemfile.lock`, `log/development.log`, `tmp/cache/bootsnap/load-path-cache`, and `.rubyn-code/`).
