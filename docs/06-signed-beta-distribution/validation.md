# Phase 06 Validation — Distribution Tooling

Validated on macOS on 2026-08-29. Phase 06 remains open until Apple release authority is available and a signed, notarized beta is accepted on a clean account.

## Deterministic contract

- `pnpm release:check` passed with Node 22.22.2, pnpm 10.14.0, Rust 1.97.1, and Ruby 4.0.6.
- Three release-contract tests passed, including version/hardened-runtime/entitlement drift and partial Apple credential rejection.
- The exact Rust 1.97.1 toolchain is installed with both `aarch64-apple-darwin` and `x86_64-apple-darwin` targets; the moving `stable` alias is not used by the repository.
- The release command refused immediately when no Developer ID Application identity was configured.

## Universal internal build

An unsigned internal build completed at `src-tauri/target/universal-apple-darwin/release/bundle`. Its executable contains both x86_64 and arm64 slices, `LSMinimumSystemVersion` is 13.0, and the bundle version is 0.1.15. Its ad-hoc linker signature is expected for an internal build and is explicitly insufficient for release.

## Regression matrix

- Frontend: 63 tests passed.
- Rust: 60 tests passed with formatting and strict Clippy.
- Fixture safety: 2 tests passed.
- Release contract: 3 tests passed.
- ESLint, TypeScript, and the Vite production build passed.
- Rubyn Code remains at the pushed `f505028` revision. Its full 2,872-example suite passed again under the pinned Ruby 4.0.6 runtime with 0 failures and 91.39% line coverage.

## External gates

`security find-identity -v -p codesigning` reports zero valid identities on this machine. A Developer ID Application certificate and one complete notarization credential set are required before tagging and running `pnpm release:macos`.

The application currently packages Rubyn Code source but not a standalone Ruby runtime and gem set. Stock beta-machine acceptance without rbenv remains a Phase 06 release gate; the artifact must not be sent to external testers until that decision is closed.
