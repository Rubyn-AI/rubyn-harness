# Keep Wayfinder canonical, local, and approval-driven

Wayfinder Maps are the canonical project-scoped decision graph in Rubyn Harness, while the task board owns only execution state for materialized Code Tickets. Every accepted resolution and graph mutation is recorded as an immutable local event, and Rubyn-authored deltas require explicit user approval; this avoids split ownership, makes crash recovery and audit possible, and leaves external trackers as future adapters rather than sources of truth.
