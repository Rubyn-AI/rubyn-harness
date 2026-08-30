# Phase 05 — Privacy, Revocation & Diagnostics Design

## Provider revocation

Rubyn Code remains the credential owner for API-key providers. Add a narrow `providers/remove` RPC that validates a provider identifier, deletes only that provider's stored key and configuration, and selects a safe remaining model when the active provider is removed. Harness exposes a separate Codex logout command because Codex owns its OAuth session. The account UI always reconfirms the provider name and refreshes the catalog from native truth after the operation.

## Sanitized diagnostics

Harness builds diagnostics from an allowlist rather than redacting a raw database export. The native report contains version/platform fields and aggregate state facts only. Project identity is represented by counts, never names or paths. Run data is represented by lifecycle counts; event payloads, prompts, model output, command output, Git evidence, and attachments are never traversed into the report. Reports are written with user-only permissions beneath the app-data diagnostics directory.

## Local-data removal

The native command first proves there are no active runs, then validates each worktree against the app-managed root before cleanup. It does not use broad recursive targets derived from frontend input. State files and generated diagnostics are removed only after managed worktree cleanup succeeds. The in-memory repository is reinitialized and the frontend clears its project, catalog, conversations, and onboarding state.

## Failure behavior

Revocation and cleanup return structured, non-secret summaries. A failed provider removal leaves catalog state unchanged. A failed managed-worktree cleanup leaves application state intact and identifies only the Rubyn-owned path that needs retry. Diagnostic failures return the destination category without embedding raw operating-system errors that may contain local paths.
