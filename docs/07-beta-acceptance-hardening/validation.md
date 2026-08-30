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
- Rust: 62 tests passed with formatting and strict Clippy; the frontend recovery flow is included in the 68-test suite.
