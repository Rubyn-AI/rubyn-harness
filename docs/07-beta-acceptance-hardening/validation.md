# Phase 07 Validation — In Progress

Validated on macOS on 2026-08-29.

The reusable native release procedure is published in `beta-checklist.md`. Its A–G, VoiceOver, rendered-contrast, and signed/notarized distribution items remain intentionally unchecked until they are exercised on one exact release candidate.

## Accessibility baseline

- Added pinned `axe-core` 4.13.0 to the frontend test environment.
- First-launch disclosure: no automated structural accessibility violations.
- Primary empty-project shell: no automated structural accessibility violations.
- Missing-runtime preflight: no automated structural accessibility violations.
- Repository trust dialog: no automated structural accessibility violations.
- Provider setup: no automated structural accessibility violations.
- Active conversation and retained Review token summaries: no automated structural accessibility violations.
- Review worktree-discard confirmation: no automated structural accessibility violations.
- Local-data removal confirmation: no automated structural accessibility violations.
- Upgrade-state recovery screen: no automated structural accessibility violations.
- Modal keyboard focus is contained within the active dialog and wraps in both directions; the app also mirrors the macOS reduced-motion media preference into its explicit session control.
- Frontend suite: 74 tests passed.

Color contrast is intentionally excluded from jsdom automation because rendered color calculation is unavailable there. Native contrast, full keyboard traversal, reduced-motion observation, and macOS assistive-technology passes remain open.

The packaged app could be launched with isolated state, but the current console session exposed neither window accessibility metadata nor screen pixels. No native visual or VoiceOver result is inferred from that environment; those checklist items remain open.

## Production asset baseline

- JavaScript: 840,963 raw bytes; 232,104 gzip bytes.
- CSS: 95,424 raw bytes; 19,661 gzip bytes.
- Release budgets: 900,000 / 250,000 JavaScript raw/gzip bytes and 110,000 / 25,000 CSS raw/gzip bytes.
- Three budget-contract tests passed, including exact over-budget reporting and missing-build failure.
- The signed release command now runs the contract tests and checks the built production assets before native packaging.
- A non-interactive `pnpm install --frozen-lockfile` passed under the pinned pnpm 10.14.0 after adding the accessibility dependency.

## Bounded runtime work

- The native event repository retains at most 5,000 events per run; its regression contract proves a 5,020-event append retains protocol sequences 21–5,020.
- The UI cache retains at most the newest 500 unique events per run while advancing the durable polling cursor.
- Project polling is guarded by a reusable exclusive-task primitive. Regression contracts prove overlapping timer ticks are skipped, polling resumes after completion, and a failed tick releases the guard.

## Native latency baseline

- The universal macOS app at Harness commit `83bdd022a2bfebab8543132433f7e2164357b32c` was measured on an Apple M5 with 10 logical CPUs and 32 GiB of memory, macOS/Darwin 25.6.0 arm64.
- Three isolated clean launches measured 503.5 ms median and 1,110.3 ms maximum process-to-ready latency against a 3,000 ms maximum budget; median frontend boot work was 282 ms.
- Three isolated launches restoring the trusted `rubyn-test` acceptance clone measured 534.0 ms median and 551.2 ms maximum process-to-project-ready latency against a 4,000 ms maximum budget; median frontend boot work was 317 ms.
- The reusable native runner fails when the packaged frontend does not report readiness, when project restoration does not complete, or when the slowest sample exceeds its explicit budget. It writes only to host-validated `rubyn-harness-test-*` temporary app-data directories; ordinary app launches retain no performance telemetry.
- Full measurements and hardware evidence are retained at `/private/tmp/rubyn-harness-test-performance-83bdd02-native/native-performance-report.json`.

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
- Frontend: 74 tests passed with lint, production build, and asset-budget verification. Production assets remain within budget at 843,381 raw / 232,874 gzip JavaScript bytes and 95,795 raw / 19,741 gzip CSS bytes.
- Engine: 2,875 examples passed; the changed engine files passed RuboCop. The full repository RuboCop baseline still reports unrelated pre-existing offenses in provider and Wayfinder tooling.
- Native startup probes now prefer direct installed rbenv Ruby binaries over shims and terminate a hung candidate plus its process group after two seconds instead of blocking the AppKit thread indefinitely.
- Rust: 66 tests passed with formatting and strict Clippy, including direct-rbenv ordering plus successful and timed-out command-probe contracts.

## Distribution status

- The current source builds both a universal `Rubyn Harness.app` and `Rubyn Harness_0.1.15_universal.dmg`.
- The local app is linker/ad-hoc signed only. `spctl` rejects it with `source=no usable signature`, as required for an unsigned artifact.
- External beta distribution remains blocked on a real Apple Developer ID Application identity and notarization credentials. The release command is fail-closed and must not be bypassed.
