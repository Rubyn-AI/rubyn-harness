# Phase 07 — Beta Acceptance & Hardening Requirements

## Goal

Exercise every approved beta journey from clean or deliberately degraded state, and turn accessibility, performance, and upgrade expectations into repeatable release gates.

## Requirements

### 7.1 Clean-state acceptance

- Acceptance uses disposable, non-pushable Rails clones and isolated Harness application data.
- The native app must cover approved Acceptances A–G without modifying the source fixture or depending on prior tester state.
- Every destructive or account-affecting step remains explicit and reversible.

### 7.2 Accessibility

- First launch, runtime failure, repository trust, provider setup, primary navigation, conversation, review, and destructive confirmations must expose meaningful names, roles, states, and keyboard paths.
- Automated accessibility checks must fail the frontend suite on detected structural violations.
- Color contrast and macOS assistive-technology behavior require native manual verification in addition to automation.

### 7.3 Performance budgets

- Clean launch, project open, navigation, and retained-history rendering must have recorded budgets on supported hardware.
- Production asset sizes must be measured and regressions must fail a deterministic check rather than relying on a bundler warning.
- Runtime polling and retained event bounds must not grow work or memory without limit.

### 7.4 Upgrade compatibility

- Compatible persisted state must survive a manual beta replacement.
- Invalid or newer state must fail closed with recovery guidance and preserve recoverable evidence.
- The acceptance record must identify the exact app, engine, fixture, and state-schema revisions used.

### 7.5 Usage and efficiency evidence

- Every conversation and retained run review must show cumulative provider-reported input, output, reasoning, and total token usage when the provider supplies it.
- Rubyn must report measured context savings separately from provider cache reuse; the UI must not present cache reuse or an invented price estimate as Rubyn-created token savings.
- Usage and savings events must remain durable with the run while obeying the existing bounded event-retention policy.
