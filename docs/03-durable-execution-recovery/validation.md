# Phase 03 Validation Record

## Automated checks

- 56 frontend tests passed.
- 53 Rust tests passed.
- 2 disposable-fixture tests passed.
- ESLint, TypeScript production build, Rust format, and Clippy with warnings denied passed.
- The macOS application bundle and DMG built successfully.

## Native Rails-fixture smoke

Fixture: `/private/tmp/rubyn-harness-acceptance-phase3-20260829`

- Trust inspection identified the canonical Git root and Rails project before opening it.
- A direct app-server probe proved that the `untrusted` approval policy emits a command callback while retaining the read-only sandbox. Declining the request did not create the probe file.
- Packaged native deny returned a one-shot decline and did not create its marker. Packaged native approve returned a one-shot accept and created its marker only in the run's managed worktree.
- The approval card visibly presented the exact command, managed working directory, Deny, and Run command controls together; a separate layout probe was denied.
- Restart converted the still-open run to `failed`, retained its worktree, expired pending decisions, and presented Retry.
- Retry reused the retained worktree and completed a harmless working-directory request.
- The conversation displayed cumulative provider usage: 60.6K total tokens, 60.4K input, 132 output, and 19.2K cached input tokens reused (32%).
- Startup migration removed raw provider frames and process output from primary and backup application state while retaining normalized audit events.

## Source preservation

The fixture source remains at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3`. Its pre-existing local modifications were not changed. The disposable Phase 03 clone remained clean; all approved writes were confined to Rubyn-managed worktrees.
