# Phase 05 — Privacy, Revocation & Diagnostics Requirements

## Req 5.1 — Provider access is visibly revocable

- A connected API-key provider can be revoked from Models & accounts only after an explicit confirmation naming that provider.
- Revocation removes Rubyn-owned credentials and provider configuration, refreshes readiness immediately, and prevents future runs from using that provider.
- Codex revocation delegates to the Codex logout flow and reports whether the account is still connected.
- Revocation never prints, returns, audits, or persists credential material.

## Req 5.2 — Support diagnostics are sanitized by construction

- A tester can create a local diagnostic report from the app and see its exact output path.
- The report includes app/runtime versions, platform, state-schema health, aggregate record counts, lifecycle counts, and non-secret provider readiness.
- The report excludes prompts, responses, diffs, source content, repository and worktree paths, attachment paths, environment values, credentials, raw provider frames, stdout, and stderr.
- Diagnostic generation fails safely with actionable guidance and never weakens file permissions.

## Req 5.3 — Local application data can be removed deliberately

- The app explains which Rubyn-owned data will be removed and requires a destructive confirmation.
- Removal refuses while runs are active, cleans only validated Rubyn-managed worktrees, removes Harness state and diagnostics, and never mutates a source repository.
- A partial cleanup reports retained paths without claiming success; retry remains possible.
- Successful removal returns the app to first-launch state without requiring a restart.

## Req 5.4 — Privacy boundaries remain durable

- Provider and diagnostic failures use allowlisted summaries rather than raw subprocess output.
- Primary state, backup state, generated diagnostics, and audit events are covered by regression tests seeded with credential-shaped and source-shaped canaries.
- Compatible state survives the phase upgrade; existing encrypted provider keys remain usable until explicitly revoked or migrated to stronger platform storage.
