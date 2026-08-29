# Phase 01 Native Smoke Record

## Run identity

- Date: 2026-08-29
- App: packaged native debug build of Rubyn Harness
- Fixture: `Rubyn-AI/rubyn-test`
- Pinned source revision: `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3`
- Disposable clone: `/private/tmp/rubyn-harness-acceptance-20260829-v2`
- Acceptance branch: `rubyn-acceptance`
- Acceptance baseline: `30cd8e4`
- Integrated acceptance commit: `374e967`

The clone path is evidence from this local run, not portable fixture configuration.

## Successful journey

1. Prepared a fresh clone with the fixture script. The clone was clean, pinned, on `rubyn-acceptance`, and had pushing disabled.
2. Opened the clone in the native app. Harness identified the project as Rails.
3. Created the Safe Post Search Destination and Wayfinder Map, approved its Graph Delta, and materialized the scoped Code Ticket.
4. Launched the ticket as a Background Run in an isolated Git worktree.
5. The run changed only `app/controllers/posts_controller.rb` and added `test/controllers/posts_controller_test.rb`.
6. Finished the waiting conversation, opened Review, inspected the two-file diff, and integrated it into the disposable clone.
7. Ran the fixture Rails suite after integration: `5 runs, 6 assertions, 0 failures, 0 errors, 0 skips`.
8. Ran a separate native approval probe. A proposed README edit produced an approval card while the worktree was clean; denying it left the worktree clean. A retry produced a second distinct file-change event, proving reused JSON-RPC request IDs no longer hide approval retries.
9. Discarded the approval-probe worktrees. The successful feature run remains auditable as integrated.

## Repository validation

- Frontend tests: 49 passed.
- Fixture safety tests: 2 passed.
- Rust tests: 46 passed.
- Rust format check: passed.
- Clippy with warnings denied: passed.
- ESLint: passed.
- TypeScript/Vite production build: passed.
- Native debug app bundle and DMG build: passed.
- Known build note: Vite reports an 825.80 kB JavaScript chunk warning; this is a performance follow-up, not a functional failure.

## Safety confirmation

The source checkout `/Users/fadedmaturity/rubyn-test` remained at `ab0f6b10bfebadf5c5f401cf237ce3f347db1ce3` with its pre-existing local status unchanged:

- modified `Gemfile.lock`
- modified `log/development.log`
- modified `tmp/cache/bootsnap/load-path-cache`
- untracked `.rubyn-code/`

No acceptance change was applied to or pushed from the source checkout.

## Findings and classification

### Product defects / beta blockers

1. **Wayfinder mutation acknowledgement:** `create_node` applied the node but remained live without an `item/completed` event. Stopping the conversation and reloading preserved the Map and allowed activation. Before beta, mutation calls need bounded completion or an explicit recovery state.
2. **Provider-log privacy:** run persistence currently retains excessive raw Codex app-server output. Provider transport must be bounded and redacted before external beta diagnostics can be considered safe.
3. **Command approvals:** the new read-only Codex sandbox correctly gates file edits, but Harness does not yet bridge app-server command approval requests. Write-producing verification commands need a clear approval path or an intentionally scoped execution policy.

### Product defects fixed during this phase

1. **Waiting-turn lifecycle:** stopping a successfully waiting conversation used to mark it cancelled and block Review. The UI now offers Finish conversation and the engine records the outcome as completed.
2. **Silent Codex file edits:** Codex had been launched with approvals disabled. The engine now uses on-request approvals, a read-only sandbox, and translates file-change requests into native Harness approval cards.
3. **Approval retry collision:** Codex can reuse a JSON-RPC request ID after denial. Harness now keys an approval by request ID plus unique item ID and returns the decision using the original request ID.
4. **Frontend test discovery:** Vitest attempted to execute the Node fixture test. The frontend command now excludes `scripts/**`; the fixture test remains a separate required command.

### Environment observations

- The packaged app was used because the live Vite development window rendered blank on this machine.
- Bundler emitted a temporary-home warning during fixture execution. It did not affect the Rails result and is not classified as a Harness failure.

## Phase result

The central Rails Wayfinder-to-integration proof passed, including isolated execution, scoped review, integration, regression coverage, denial safety, and source immutability. Phase 01 is complete. The unresolved items above are explicit entry blockers for external beta and feed the execution, recovery, and privacy phases.
