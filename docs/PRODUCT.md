# Product thesis

Rubyn Harness is the Rails engineering control room that maximizes **confidence
per dollar**. It spends expensive reasoning only where it changes an
engineering decision and makes all other work scoped, cached, parallel, and
verifiable.

## North star

A Ruby or Rails company can take a product objective from intent to a reviewed,
merge-ready change faster than with a general-purpose coding harness, while
using less model context and retaining a clear account of ownership, cost,
evidence, and risk.

## Product principles

1. **Bounded delegation over opaque swarms.** Every background run has a role, project,
   worktree, write scope, budget, and completion condition.
2. **Evidence over activity.** Progress is passing tests, reviewed diffs, and
   resolved acceptance criteria—not tool-call volume.
3. **Parallel when independent.** The scheduler fans out independent work and
   keeps sequential or overlapping work ordered.
4. **Rails-native by default.** Project discovery, skills, review gates, and
   vocabulary understand Rails applications.
5. **Local and inspectable.** Durable state, transcripts, summaries, and
   artifacts live locally and remain auditable.
6. **Cheap first, deep when justified.** Scouts and validators use economical
   models; difficult implementation and independent review earn deeper models.

## Core journey

1. Add a Rails project. Rubyn discovers its Ruby/Rails versions, test stack,
   data services, job backend, auth, CI, deployment, and conventions.
2. State an objective. Wayfinder proposes a dependency graph, file scopes,
   agent roles, budgets, risks, and acceptance criteria.
3. Create and edit reusable Agent Profiles through five standing-instruction parts:
   Mission, Starting Context, Working Method, Finish Line, and Guardrails. Choose
   the profile for each board column. Moving a task selects Rubyn's
   guidance; a person explicitly launches any token-spending Background Run.
4. Follow execution in Tasks & todos. Wayfinder Code Tickets materialize as
   Board Tasks, while background runs attach code, test, review, and cost evidence.
5. Inspect semantic diffs. Rails-specific gates flag migrations, authorization,
   API contracts, jobs, and query-sensitive changes.
6. Integrate only reviewed work. Conflicts are presented; merges are never
   silently performed.

## Primary success metrics

- Lead time per accepted task
- Cost per accepted task
- Human review minutes per accepted diff
- Rework after review
- Percentage of background runs with passing focused and full-test evidence
- Merge conflict rate
- Compaction resume success
- Token and cost savings from model routing, caching, and compaction

## Explicit non-goals

- A generic unrestricted shell exposed to the webview
- An always-on, unbounded autonomous swarm
- Silent model fallback or silent context loss
- Automatic merging or closing human-owned work without review evidence
- Replacing Rubyn Code's agent loop with a second competing engine
