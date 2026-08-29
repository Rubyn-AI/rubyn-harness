# Phase 04 Validation Record

## Automated checks

- 57 frontend tests passed.
- 57 Rust tests passed.
- 2 disposable-fixture tests passed.
- ESLint, TypeScript production build, Rust format, Clippy with warnings denied, and `git diff --check` passed.
- The macOS application bundle and `Rubyn Harness_0.1.15_aarch64.dmg` built successfully.

## Native Rails-fixture smoke

Fixture: `/private/tmp/rubyn-harness-acceptance-phase4-20260829`

- A retained one-file run showed its unified diff, changed-file count, source HEAD, source cleanliness, and matching run base before integration.
- Integration required a second review-specific confirmation that named the destination source and repeated that native checks would run before mutation.
- Advancing the disposable source after another run finished produced a visible source-drift blocker and disabled integration. Discard removed that run's managed worktree without changing the source.
- A read-only run displayed zero changed files and disabled integration.
- An intentionally obstructed discard cleanup entered `discard_cleanup_pending`, survived restart, exposed Retry cleanup, and recovered after the obstruction was removed. The durable disposition remained discarded.
- Native testing exposed and fixed orphaned managed-directory recovery after Git had already deregistered a partially removed worktree.
- Native testing exposed and fixed the integration result identity: run 13 persisted source commit `a5bfa00f86ad53f494d8e18dc34164e6a1d3e126`, exactly matching the fixture source HEAD after cherry-pick.
- Native testing exposed and fixed a confirmation placement that overlapped the WebView scrollbar. The final packaged flow placed the confirmation above the changed-file list and accepted a pointer click.
- Run 13 displayed 78.6K total tokens and 57.6K cached input tokens reused (73%), then integrated its single approved file and removed the managed workspace.

## Source preservation

The original `/Users/fadedmaturity/rubyn-test` checkout remains at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3`; its pre-existing generated-file modifications were not changed. All destructive smoke work used the disposable fixture. The fixture is clean at `a5bfa00f86ad53f494d8e18dc34164e6a1d3e126` after its explicit acceptance-only integration and drift commits.
