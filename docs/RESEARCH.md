# Harness research notes

Research date: 2026-08-21

## What developers want

The consistent signal is not “more autonomous agents.” It is **bounded,
inspectable delegation**: quality signals early, explicit authority, provenance,
uncertainty disclosure, and least privilege. Microsoft Research reached this
conclusion across 860 developers in
[To Copilot and Beyond](https://www.microsoft.com/en-us/research/publication/to-copilot-and-beyond-22-ai-systems-developers-want-built/).

Parallel work helps when tasks are independent—research, separate modules,
review, and competing diagnostic hypotheses. It performs poorly for sequential
work or agents editing the same files, and consumption grows with the number of
active teammates. Claude's documentation makes both tradeoffs explicit in its
[agent teams guide](https://code.claude.com/docs/en/agent-teams).

## Patterns worth carrying forward

- Claude Code isolates focused subagents from the coordinator's context and
  supports scoped tools, models, turns, memory, and worktrees. See
  [subagents](https://code.claude.com/docs/en/sub-agents) and
  [worktrees](https://code.claude.com/docs/en/worktrees).
- OpenCode gives agent definitions explicit read, write, shell, and delegation
  permissions plus hard step limits. See
  [agent configuration](https://dev.opencode.ai/docs/agents/).
- Hermes emphasizes portable skills, isolated subagents, provider routing, and
  programmatic tool pipelines. See the
  [Hermes documentation](https://hermes-agent.nousresearch.com/docs/).
- GitHub normalized model, token, cache, and timestamp records across several
  coding agents before optimizing its own workflows. Cost needs a normalized
  first-class data model, not a provider-specific afterthought. See
  [Improving token efficiency in GitHub agentic workflows](https://github.blog/ai-and-ml/github-copilot/improving-token-efficiency-in-github-agentic-workflows/).

## Product consequences

1. Fan-out is proposed from dependency and file ownership analysis, not enabled
   indiscriminately.
2. Every run has a cost/turn/context ceiling, explicit permissions, and a
   visible model-routing explanation.
3. Write agents use isolated worktrees. Integration and review are separate
   nodes with recorded evidence.
4. Cost is normalized per task, run, project, and accepted outcome; estimates
   are labeled separately from provider-reported usage.
5. Compaction preserves a continuation brief, immutable raw evidence, and an
   inspectable checkpoint. It never silently drops constraints or identity.
6. Rails differentiation lives in discovery, skills, and review gates:
   reversible/lock-aware migrations, authorization, tenancy, jobs, query
   behavior, test isolation, and public contracts.

## Why Rubyn Code is the backbone

[Rubyn Code](https://github.com/MatthewSuttles/rubyn-code) already supplies the
Rails-aware agent loop, 112 on-demand skills, JSON-RPC IDE mode, codebase
indexing, MCP, teams, task storage, checkpoints, usage/cost tracking, budgets,
and context compaction. A second engine would waste effort and split behavior.

Rubyn Harness therefore owns cross-project coordination, worktree isolation,
graph scheduling, visual inspection, policy, and review evidence. Rubyn Code
owns the coding-agent runtime. The boundary is a typed adapter so other engines
can be added without binding product state to their internal formats.
