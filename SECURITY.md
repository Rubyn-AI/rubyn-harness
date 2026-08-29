# Security policy

## Design boundary

Rubyn Harness treats the Tauri webview as unprivileged. Filesystem, Git, and
process operations are implemented by typed Rust commands with explicit path
and argument validation. The UI is never given a generic shell command.

Write-capable agents should run in task-specific Git worktrees. The root
checkout is treated as read-only, secrets are referenced rather than copied
into prompts, and dangerous external actions remain approval-gated.

The current runner creates a detached worktree for every runtime execution,
rejects project MCP/hook configuration until a trust ceremony exists, rejects
worktrees containing symbolic links, auto-denies non-file tool approvals, and
disables bypass mode. File edits are confined to the generated worktree. A
platform OS sandbox and richer trust policy remain required before untrusted
repositories should be enabled by default.

## Reporting a vulnerability

Please open a private security advisory in the GitHub repository. Include the
affected version, reproduction steps, impact, and any suggested mitigation.
Do not open a public issue before a fix is available.

## Supported versions

Until the first stable release, security fixes are applied to the latest main
branch only.
