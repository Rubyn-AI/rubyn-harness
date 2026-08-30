# Rubyn Harness

**A Rails-native engineering control room for safe, economical agent teams.**

Rubyn Harness is an open-source Tauri desktop application built around
[Rubyn Code](https://github.com/MatthewSuttles/rubyn-code). This repository is
a native application for running bounded Rubyn agents in isolated Git
worktrees, recording their execution, and integrating reviewed changes.

The product is designed for Ruby and Rails teams that want higher throughput
without turning agent coordination into another full-time job.

## What works now

- A conversation-first native shell with project-grouped history, sticky composers, and rename, pin, archive, and restore controls
- Real local project and Git inspection through typed Tauri commands
- Rubyn Code pinned as a submodule and launched through its IDE JSON-RPC mode
- Persistent multi-turn Rubyn conversations that can resume in the same retained worktree after a worker exits, with streaming replies and follow-up messages in the same actor context
- Up to three concurrent actors, each in a detached isolated Git worktree
- Runtime cancellation, a 30-minute ceiling, bounded output capture, and process-group cleanup on Unix
- Durable project-scoped task DAGs, shared todos, conversations, background runs, normalized events, and restart recovery
- Project-owned task-board columns with Backlog, Planning, Implementing, Review, and Done defaults; columns can be added, renamed, reordered, or deleted
- A dedicated Agents area with guided five-part create and edit flows—Mission, Starting Context, Working Method, Finish Line, and Guardrails—plus column handoffs that select how Rubyn should work without silently launching a paid model run
- Rubyn conversation assignment for tasks and todos; task-based launches claim the task automatically
- Explicit fan-out preflight with task selection, capacity disclosure, and isolated worktrees, bounded by the global three-background-run ceiling
- An audited Rubyn `harness_task` tool that reads and mutates the same native task/todo board as the user
- Live inspection of each retained runtime worktree
- Clean-tree guarded, hook-disabled integration and explicit discard actions with managed cleanup
- Bundled Rubyn skill discovery and real project-local skill creation
- Bounded durable event retention and full textual review of untracked files
- Non-file tool requests denied; file edits persist as proposals with explicit approve or deny controls in the conversation
- Atomic local persistence with a backup and versioned schema

The browser build deliberately shows only a desktop-required screen. It never
simulates agents, repository state, reviews, usage, or task progress.

## Roadmap, not shipped

- Rails-specific review gates
- Provider adapters beyond Rubyn Code
- Cost, budget, and compaction projections beyond the shipped per-conversation token and cache-reuse telemetry
- A continuously running portfolio scheduler, notarized releases, and an updater
- Bundled Ruby runtime and gems, plus Windows process-tree containment

## Architecture

```text
React control room
       │ typed Tauri commands + events
       ▼
Rust application boundary
  ├── project and Git inspection
  ├── process lifecycle and policy
  ├── local persisted state
  └── agent-driver interface
              │ JSON-RPC / structured CLI
              ▼
      Provider-aware agent drivers
        ├── Rubyn Code (Anthropic/OpenAI-compatible APIs)
        └── Codex app server (ChatGPT OAuth)
```

The renderer never receives generic shell access. The Rust host validates
project paths and invokes allowlisted engine operations. Bypass/YOLO launch is
disabled until an auditable approval policy is implemented. Direct API
providers use Rubyn Code adapters and its encrypted token store. Codex runs
through the installed Codex app server and reuses the account established by
`codex login`; the harness never reads or copies its OAuth token. Cross-project
orchestration, durable review evidence, and cost projections remain roadmap
work.

See [the engine integration guide](docs/RUBYN_ENGINE_INTEGRATION.md) and the
[backbone decision](docs/adr/0001-rubyn-engine-backbone.md).

## Development

Prerequisites:

- Node.js 22.22.2
- pnpm 10.14.0
- Rust 1.97.1
- Tauri 2 platform prerequisites
- Ruby 4.0.6 for running and releasing the bundled Rubyn Code source

```bash
git clone --recurse-submodules https://github.com/Rubyn-AI/rubyn-harness.git
cd rubyn-harness
pnpm install
pnpm tauri dev
```

## macOS beta prerequisite

The current external beta packages Rubyn Code but does not package Ruby itself. Before opening the beta, install Ruby 4.0.6 with rbenv or Homebrew, use that Ruby to run `gem install rubyn-code`, and then launch Rubyn Harness. The installed gem supplies the runtime dependency set; Harness runs its pinned bundled Rubyn Code source.

If setup is incomplete, Harness blocks new work before repository selection and shows these steps in the app. A standalone bundled Ruby runtime remains roadmap work.

The browser-only development surface verifies that non-native launches fail
closed. It does not simulate the app:

```bash
pnpm dev
```

## Verification

```bash
pnpm lint
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm test:fixture
pnpm test:release
pnpm release:check
```

## Engine development

`engine/rubyn-code` is a pinned Git submodule. Clone with
`--recurse-submodules`, or initialize an existing checkout with:

```bash
git submodule update --init --recursive
```

Harness changes and engine changes should be reviewed independently. Update the
submodule pin only after the matching Rubyn Code revision passes its own tests.

## Project status

Rubyn Harness is in active development. The current desktop workflow is real:
choose a repository with the native folder picker, create a Wayfinder Map and
workflow board, assign conversations to tasks and todos, talk to persistent
Rubyn actors, let Rubyn update the shared board, fan out ready work, create
project skills, inspect durable events and complete worktree diffs, then
integrate or discard. New conversations can choose a Codex subscription model,
Anthropic, OpenAI API, or a configured OpenAI/Anthropic-compatible endpoint
such as MiniMax. Unsupported cost and compaction surfaces are not rendered. The security
limitations below remain release gates for arbitrary untrusted repositories.

Conversations are durable chat history and do not count as runs. A conversation
becomes a background run only when it is linked to a Board Task or Wayfinder
Ticket; activity and history counters use that background classification.

Project runtime settings can turn Rubyn Chisel on or off. The harness maps On
to Chisel's `full` mode and preserves any existing `lite` or `ultra` mode as an
enabled state. Chisel affects Rubyn Code prompts; Codex app-server runs retain
their own behavior.

## License

MIT. Rubyn Code is included as a submodule and retains its own MIT license.
