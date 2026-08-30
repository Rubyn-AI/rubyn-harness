# Rubyn Harness macOS Beta Release Checklist

Use one fresh `pnpm acceptance:create` directory for the release candidate. Never run acceptance against the source `rubyn-test` checkout or real Harness application data.

After each native journey, record the result with `pnpm acceptance:record -- --run <acceptance-directory> --checkpoint A --status passed --evidence "what was directly observed"`. The recorder requires evidence, refuses a stale Harness or engine commit, writes atomically with mode 0600, and marks the run passed only after A–G all pass.

## Record the candidate

- [ ] `acceptance-run.json` identifies the exact Harness commit, engine commit, state schema, fixture revision, and isolated paths.
- [ ] The fixture push URL is `disabled://rubyn-harness-acceptance`.
- [ ] Source fixture HEAD and status hash match before and after acceptance.
- [ ] Universal `.app` and `.dmg` were produced from the recorded Harness commit.
- [ ] `pnpm release:check` passes with the intended Developer ID identity and notarization credentials.

## Automated release gates

- [ ] `pnpm test`
- [ ] `pnpm test:fixture`
- [ ] `pnpm test:release`
- [ ] `pnpm test:performance`
- [ ] `pnpm lint`
- [ ] `pnpm build && pnpm performance:check`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] Full Rubyn Code RSpec suite and changed-file RuboCop pass.
- [ ] `pnpm performance:native -- --project=<isolated-project> --output=<safe-temp-output> --runs=3`

## Native Acceptance A — successful Wayfinder delivery

- [ ] Complete first-launch disclosure, inspect the canonical Rails path, and explicitly trust it.
- [ ] Connect a real provider account and select a ready model without recording credentials.
- [ ] Create a Map and observable Destination, resolve its bootstrap questions, and approve the Graph Delta.
- [ ] Activate an unblocked Code Ticket and verify its immutable Brief Version becomes a Board Task.
- [ ] Launch the task into a distinct worktree, approve one exact edit, and retain the approval audit event.
- [ ] Review real Git status and diff, integrate deliberately, and verify the source commit only after confirmation.

## Native Acceptance B — denied edit

- [ ] Launch isolated work, deny an exact requested edit, and verify the source checkout is unchanged.
- [ ] Confirm the denial remains in the run audit and execution either continues safely or ends with an actionable failure.

## Native Acceptance C — restart recovery

- [ ] Force-quit with a retained Conversation/worktree and pending or recent events.
- [ ] Relaunch and verify the Conversation, worktree, messages, usage summary, and disposition are retained.
- [ ] Verify pending approvals expire safely or restore only when their exact authorization remains valid.

## Native Acceptance D — provider failure and retry

- [ ] Cause a real provider failure without exposing a credential in UI, state, logs, or diagnostics.
- [ ] Verify the Conversation becomes failed rather than appearing indefinitely busy.
- [ ] Retry in the same Conversation with a connected model and verify cumulative usage remains understandable.

## Native Acceptance E — unsafe integration

- [ ] Move the source HEAD or create a real merge conflict after the worktree begins.
- [ ] Verify integration is blocked before source mutation and the worktree, diff, and recovery evidence remain available.

## Native Acceptance F — discard and cleanup recovery

- [ ] Open discard confirmation, cancel it once, then deliberately confirm discard.
- [ ] Exercise cleanup failure and verify the terminal cleanup-pending state retains evidence and offers retry.
- [ ] Retry cleanup and verify only the Harness-managed worktree is removed.

## Native Acceptance G — revoke, diagnose, and upgrade

- [ ] Revoke the connected provider and verify future work is blocked until another model is ready.
- [ ] Create diagnostics and inspect the file for versions/counts only—no credentials, prompts, source, paths, diffs, command output, or attachments.
- [ ] Replace the app with the release candidate and verify compatible state survives.
- [ ] Open deliberately newer/invalid state and verify fail-closed recovery guidance without modified evidence.

## Native accessibility and presentation

- [ ] Traverse first launch, project trust, provider setup, Conversation, Review, and confirmations using only the keyboard.
- [ ] Verify modal focus starts inside the dialog, cannot escape behind it, and Escape performs a safe dismissal where offered.
- [ ] Verify macOS Increase Contrast and Reduce Motion produce readable, stable UI.
- [ ] Verify VoiceOver announces headings, navigation, dialogs, controls, state, live progress, token usage, and errors meaningfully.
- [ ] Inspect the app at minimum supported window size and at 200% zoom without hidden required actions.

## Ship decision

Ship only when every required item and Acceptance A–G is passed on the recorded candidate, the source fixture is unchanged, no unexplained retained test processes/worktrees remain, and the distributed DMG passes Gatekeeper verification without bypass instructions.
