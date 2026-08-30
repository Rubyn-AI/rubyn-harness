# Phase 06 — Signed Beta Distribution Requirements

## Goal

Produce one reproducible universal macOS beta artifact whose version, source, engine revision, signature, notarization ticket, supported OS, and checksum can be verified before it reaches a tester.

## Requirements

### 6.1 Deterministic source and toolchains

- Node, pnpm, Rust, Cargo, Ruby, npm dependencies, Rust dependencies, and Rubyn Code must be pinned.
- Application versions must match across npm, Cargo, and Tauri metadata.
- A release must refuse a dirty worktree, an uninitialized or drifting submodule, or a commit without the exact version tag.

### 6.2 Universal hardened macOS bundle

- The beta supports Apple Silicon and Intel in one application and DMG.
- The minimum supported version is macOS 13.0 and is recorded in bundle metadata.
- Hardened runtime is explicit and release entitlements cannot enable `get-task-allow`.

### 6.3 Developer ID signing and notarization

- A release requires a locally available Developer ID Application identity.
- Notarization requires one complete Apple credential set and never prints credential values.
- The completed app and DMG must carry valid stapled tickets and pass Gatekeeper assessment.

### 6.4 Release evidence

- The release path runs the full frontend, fixture, release-contract, Rust, and Rubyn Code checks.
- It emits a SHA-256 checksum and a machine-readable manifest with version, tag, Harness commit, engine commit, architectures, minimum OS, and toolchains.
- Unsigned or ad-hoc local bundles are never described as beta release artifacts.

## Out of scope

- Publishing to a public download service
- Automatic updates
- Apple Developer enrollment or certificate issuance
- Bundling a standalone Ruby runtime
