# Phase 07 Validation — In Progress

Validated on macOS on 2026-08-29.

## Accessibility baseline

- Added pinned `axe-core` 4.13.0 to the frontend test environment.
- First-launch disclosure: no automated structural accessibility violations.
- Primary empty-project shell: no automated structural accessibility violations.
- Missing-runtime preflight: no automated structural accessibility violations.
- Repository trust dialog: no automated structural accessibility violations.
- Local-data removal confirmation: no automated structural accessibility violations.
- Upgrade-state recovery screen: no automated structural accessibility violations.
- Frontend suite: 68 tests passed.

Color contrast is intentionally excluded from jsdom automation because rendered color calculation is unavailable there. Native contrast, keyboard, reduced-motion, and macOS assistive-technology passes remain open.

## Production asset baseline

- JavaScript: 840,963 raw bytes; 232,104 gzip bytes.
- CSS: 95,424 raw bytes; 19,661 gzip bytes.
- Release budgets: 900,000 / 250,000 JavaScript raw/gzip bytes and 110,000 / 25,000 CSS raw/gzip bytes.
- Three budget-contract tests passed, including exact over-budget reporting and missing-build failure.
- The signed release command now runs the contract tests and checks the built production assets before native packaging.
- A non-interactive `pnpm install --frozen-lockfile` passed under the pinned pnpm 10.14.0 after adding the accessibility dependency.

## Upgrade-state safety

- A state schema newer than the supported schema 9 is rejected before migration or save; primary and backup bytes remain unchanged.
- A corrupt primary with a valid backup restores the primary without rotating corrupt bytes over the known-good backup.
- Runtime health and application-state health now initialize independently. A state failure keeps a healthy Rubyn runtime marked ready and renders dedicated, actionable recovery guidance.
- A checked-in, path-free schema-6 fixture upgrades to schema 9, retains its model preference, and preserves the original fixture byte-for-byte as the backup.
- Rust: 63 tests passed with formatting and strict Clippy; the frontend recovery flow is included in the 68-test suite.

## Isolated acceptance run

- `pnpm acceptance:create` creates a non-pushable Rails clone, an isolated and host-validated Harness app-data directory, and a mode-0600 manifest with A–G evidence slots.
- Five fixture/acceptance contract tests passed, including existing-destination refusal and refusal outside system temporary storage.
- The real `rubyn-test` checkout remained at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3` with the same five pre-existing status entries after preparation.
- The first clean manifest recorded Harness 0.1.15, schema 9, the exact Harness and engine commits, the prepared fixture commit, disabled push URL, and source-status SHA-256.

## Token usage and efficiency

- Codex cumulative usage remains provider-reported and privacy-minimized; account metadata and thread identity are not retained in usage events.
- Rubyn-provider chats now emit cumulative input, cached-input, cache-write, output, and total tokens.
- Rubyn measures tokens removed by tool-output compression and context compaction. The UI reports these as Rubyn savings and reports provider cache reuse separately.
- The same durable summary renders in an active conversation and its retained Review screen; unavailable provider telemetry is labeled unavailable.
- Frontend: 68 tests passed with lint, production build, and asset-budget verification. Production assets remain within budget at 842,014 raw / 232,394 gzip JavaScript bytes and 95,795 raw / 19,741 gzip CSS bytes.
- Engine: 2,875 examples passed; the changed engine files passed RuboCop. The full repository RuboCop baseline still reports unrelated pre-existing offenses in provider and Wayfinder tooling.
- Native startup probes now terminate a hung Ruby candidate and its process group after five seconds instead of blocking the AppKit thread indefinitely.
- Rust: 65 tests passed with formatting and strict Clippy, including successful and timed-out command-probe contracts.
