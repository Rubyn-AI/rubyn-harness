# Contributing

Thank you for helping build Rubyn Harness.

Keep changes narrow and reviewable. Product code should preserve the boundary
between domain behavior, application orchestration, adapters, and presentation.
Prefer small interfaces and composition; use design patterns only when they
make variability or lifecycle rules clearer.

Before opening a pull request, run the frontend and Rust verification commands
listed in the README. Changes to the Rubyn Code engine belong in its repository;
update the pinned submodule revision in a separate harness change after the
engine revision is reviewed.

New agent capabilities must document permissions, cost behavior, cancellation,
failure semantics, and whether usage is provider-reported or estimated.
