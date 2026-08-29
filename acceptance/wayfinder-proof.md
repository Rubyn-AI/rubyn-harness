# Wayfinder Proof — Safe Post Search

## Destination

A maintainer can search posts using arbitrary text without SQL injection, while the HTML and JSON post listings retain their existing behavior.

## Boundaries

- Keep the existing Rails controller and model entry points.
- Add a regression test that supplies SQL metacharacters and proves they are treated as search text.
- Preserve ordinary title and body substring search.
- Do not redesign authentication, pagination, publishing, notifications, or reporting.
- Do not repair unrelated intentionally poor code in the fixture.

## Expected Wayfinder shape

The Map should expose the unsafe query construction as Fog, settle the intended matching behavior, and produce at least one Code Ticket covering the smallest safe implementation plus regression evidence.

## Evidence required before integration

1. The Background Run uses an isolated worktree.
2. Every proposed repository edit receives an explicit approval decision.
3. The Review view shows only the scoped implementation and regression test.
4. The targeted regression test passes.
5. The disposable acceptance clone receives the integrated change while the fixture source revision and status remain unchanged.

## Explicit non-goals

- Broad Post model or controller refactoring
- Performance tuning unrelated to safe query construction
- Changes to production deployment or credentials
- Publishing changes to `Rubyn-AI/rubyn-test`
