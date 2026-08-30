# Phase 06 Validation — Distribution Tooling

Validated on macOS on 2026-08-29. Phase 06 remains open until notarization credentials are available and a signed, notarized beta is accepted on a clean account.

## Deterministic contract

- `pnpm release:check` passed with Node 22.22.2, pnpm 10.14.0, Rust 1.97.1, and Ruby 4.0.6.
- Three release-contract tests passed, including version/hardened-runtime/custom-entitlement drift and partial Apple credential rejection.
- The exact Rust 1.97.1 toolchain is installed with both `aarch64-apple-darwin` and `x86_64-apple-darwin` targets; the moving `stable` alias is not used by the repository.
- The release command refuses immediately when its Developer ID identity or notarization credential set is absent or incomplete.

## Universal signed candidate

A Developer ID-signed build completed at `src-tauri/target/universal-apple-darwin/release/bundle`. Its executable contains both x86_64 and arm64 slices, `LSMinimumSystemVersion` is 13.0, and the bundle version is 0.1.15. Both the app and DMG satisfy their designated requirements under `Developer ID Application: Matthew Suttles (8MWPMGK3YN)`, and the app signature carries the hardened-runtime flag without a custom entitlement blob.

## Regression matrix

- Frontend: 74 tests passed.
- Rust: 67 tests passed with formatting and strict Clippy.
- Fixture and acceptance safety: 10 tests passed.
- Release contract: 3 tests passed.
- Runtime preflight includes the provider-key bridge and fails before repository selection when its Ruby dependencies cannot load.
- ESLint, TypeScript, and the Vite production build passed.
- Rubyn Code is pinned to pushed revision `94f932d`. Its full 2,887-example suite passed under Ruby 4.0.6 with 0 failures and 91.19% line coverage.

## External gates

`security find-identity -v -p codesigning` reports a valid Developer ID Application identity. No complete App Store Connect API-key or Apple-ID notarization credential set is available to the release process, so the release commit must not be tagged and `pnpm release:macos` remains correctly unavailable.

The rebuilt universal app was launched with isolated HOME and PATH values exposing no rbenv or Homebrew runtime. It reported Rubyn as unavailable, replaced repository selection with a prominent “Finish Rubyn runtime setup” preflight, showed the Ruby 4.0.6 / `gem install rubyn-code` / relaunch sequence, and prevented conversation launch. The public Rubyn Code 0.4.0 gem was installed into an isolated gem home; its dependency set successfully loaded the pinned bundled 0.9.0 engine. The runtime prerequisite decision is therefore closed for this beta, while a standalone runtime remains roadmap work.

The universal application and DMG rebuilt successfully with Developer ID signatures. `codesign --verify --deep --strict` accepts the app, and direct DMG signature verification also passes. Gatekeeper rejects the app with `source=Unnotarized Developer ID`, which is the expected fail-closed result until Apple notarization and stapling succeed. The artifacts are not eligible for external distribution.
