# Phase 06 — Signed Beta Distribution Design

## Toolchain contract

`.nvmrc`, `.ruby-version`, and `rust-toolchain.toml` pin the build runtimes. Package metadata pins pnpm and repeats the exact Node version; Cargo repeats the exact Rust version. `scripts/release-contract.mjs` treats drift among these files, application versions, lockfiles, engine resources, deployment metadata, or signing policy as a release-blocking error.

## macOS bundle policy

Tauri produces only application and DMG bundles. Its macOS configuration explicitly sets macOS 13.0 and hardened runtime. The application requires no custom entitlements, so the release contract rejects any configured entitlement blob; this avoids both unnecessary privileges and invalid empty-entitlement signatures. The release command targets `universal-apple-darwin` and inspects the final executable for both arm64 and x86_64 slices.

## Trusted release command

`scripts/release-macos.mjs` is the only documented beta release entrypoint. Before building it verifies a clean exact-tag source state, clean submodule pin, Developer ID identity, and one complete Tauri-supported notarization credential set. It then runs every release check, builds through Tauri, and verifies code signing, hardened runtime, stapling, and Gatekeeper acceptance.

The script reads credential presence but never logs values. It builds without Apple credentials, then uses Apple's native `codesign`, `notarytool`, and `stapler` commands so universal signatures are independently verifiable. App Store Connect API credentials are supported directly. Apple ID credentials must first be stored with `notarytool store-credentials`; the release receives only the Keychain profile name, never an app-specific password.

## Provenance

After verification, the command writes a checksum file beside the DMG and a JSON release manifest. The manifest deliberately contains no machine path, identity, account, or secret.

## Runtime prerequisite

The first beta packages the pinned Rubyn Code 0.9.0 source but deliberately does not package Ruby. A tester installs Ruby 4.0.6 and runs `gem install rubyn-code` with that Ruby; the public gem supplies the compatible runtime dependency set while Harness continues to execute its pinned bundled engine source.

Harness validates both the Ruby version and the ability to load the bundled engine. With no compatible runtime it blocks the empty-project call to action, disables conversation launch for remembered projects, and displays the setup sequence before model-backed work can begin.

## Known external gate

This machine has a valid Developer ID Application identity, but no complete notarization credential set is available. Developer ID-signed local candidates can be verified here; an externally distributable beta remains blocked until an App Store Connect API key or `notarytool` Keychain profile is configured and the trusted release command succeeds.
