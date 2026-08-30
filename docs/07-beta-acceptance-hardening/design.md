# Phase 07 — Beta Acceptance & Hardening Design

## Acceptance matrix

The approved journeys in `docs/user-journeys.md` remain the product contract. Phase 07 records one reproducible matrix for Acceptances A–G, with each native pass using a fresh app-data directory and disposable fixture clone. External account or Apple-signing actions are never simulated as successful.

## Accessibility gate

`axe-core` runs inside the existing jsdom product-flow suite so accessibility assertions share the same native bridge states as functional tests. Automated checks begin with first launch and the empty-project shell, then expand to runtime failure, repository trust, provider setup, conversation, review, and destructive confirmation surfaces. Color contrast is disabled in jsdom because it cannot calculate rendered colors; native visual and assistive-technology checks remain separate acceptance work.

## Performance evidence

Asset budgets will inspect built artifacts, while native timing captures clean launch and representative project/history loads. Rubyn Harness is a local Tauri app, so compressed transfer size is only supporting evidence; JavaScript parse size and observable interaction latency are the primary budgets.

## Upgrade evidence

Upgrade tests operate only on isolated application data. A prior-version fixture is copied, the current application opens it, and schema migration or recovery behavior is recorded before the fixture is discarded.

