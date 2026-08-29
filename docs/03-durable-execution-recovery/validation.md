# Phase 03 Validation Record

## Automated checks

- 56 frontend tests passed.
- 53 Rust tests passed.
- 2 disposable-fixture tests passed.
- ESLint, TypeScript production build, Rust format, and Clippy with warnings denied passed.
- The macOS application bundle built successfully.

## Native Rails-fixture smoke

Fixture: `/private/tmp/rubyn-harness-acceptance-phase3-20260829`

- Trust inspection identified the canonical Git root and Rails project before opening it.
- A write-command probe ran in an isolated worktree. Codex's read-only sandbox blocked the command before emitting a command-approval callback; the probe file was not created and the source fixture stayed clean.
- Restart converted the still-open run to `failed`, retained its worktree, expired pending decisions, and presented Retry.
- Retry reused the retained worktree and completed a harmless working-directory request.
- The conversation displayed cumulative provider usage: 60.6K total tokens, 60.4K input, 132 output, and 19.2K cached input tokens reused (32%).
- Startup migration removed raw provider frames and process output from primary and backup application state while retaining normalized audit events.

## Remaining acceptance gap

The current Codex app-server did not emit `item/commandExecution/requestApproval` for a write attempted under the read-only sandbox; it rejected the write internally. Protocol normalization, durable decision handling, UI behavior, and response values are covered automatically, but native approve/deny clicking remains open until a provider scenario reliably emits that callback without weakening file-edit approval guarantees.

## Source preservation

The fixture source remains at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3`. Its pre-existing local modifications were not changed. The disposable Phase 03 clone remained clean.
