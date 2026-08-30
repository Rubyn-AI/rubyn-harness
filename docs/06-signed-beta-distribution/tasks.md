# Phase 06 — Signed Beta Distribution Tasks

## [x] 1. Pin deterministic inputs

- [x] 1.1 Pin Node, pnpm, Rust, Cargo, and Ruby versions. (refs Req 6.1)
- [x] 1.2 Require matching application versions and committed dependency locks. (refs Req 6.1)
- [x] 1.3 Verify the exact Rubyn Code submodule revision and bundled engine resources. (refs Req 6.1)

## [x] 2. Harden the macOS bundle contract

- [x] 2.1 Set explicit macOS 13.0 metadata and hardened runtime. (refs Req 6.2)
- [x] 2.2 Add reviewed minimal entitlements and reject `get-task-allow`. (refs Req 6.2)
- [x] 2.3 Require universal arm64 and x86_64 output. (refs Req 6.2)

## [x] 3. Automate trusted release verification

- [x] 3.1 Refuse dirty, drifting, untagged, unsigned, or incompletely authorized releases. (refs Req 6.1, Req 6.3)
- [x] 3.2 Run the full validation matrix before packaging. (refs Req 6.4)
- [x] 3.3 Verify Developer ID, hardened runtime, stapling, and Gatekeeper. (refs Req 6.3)
- [x] 3.4 Emit checksum and machine-readable provenance. (refs Req 6.4)

## [ ] 4. Produce the external beta artifact

- [x] 4.1 Install and verify a valid Developer ID Application identity. (refs Req 6.3)
- [ ] 4.2 Configure one complete notarization credential set. (refs Req 6.3)
- [ ] 4.3 Tag the release commit and run the trusted release command. (refs Req 6.1, Req 6.3)
- [ ] 4.4 Verify the signed DMG on a clean macOS account and record acceptance. (refs Req 6.3, Req 6.4)

## [x] 5. Close runtime distribution decision

- [x] 5.1 Validate the beta with isolated HOME and PATH values that expose only stock macOS tools. (refs Req 6.4)
- [x] 5.2 Make the Ruby 4.0.6 and runtime-gem prerequisite explicit, actionable, and fail-closed in the app. (refs Req 6.4)
