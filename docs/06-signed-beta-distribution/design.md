# Phase 06 — Signed Beta Distribution Design

## Toolchain contract

`.nvmrc`, `.ruby-version`, and `rust-toolchain.toml` pin the build runtimes. Package metadata pins pnpm and repeats the exact Node version; Cargo repeats the exact Rust version. `scripts/release-contract.mjs` treats drift among these files, application versions, lockfiles, engine resources, deployment metadata, or entitlements as a release-blocking error.

## macOS bundle policy

Tauri produces only application and DMG bundles. Its macOS configuration explicitly sets macOS 13.0, hardened runtime, and a reviewed minimal entitlements file. The release command targets `universal-apple-darwin` and inspects the final executable for both arm64 and x86_64 slices.

## Trusted release command

`scripts/release-macos.mjs` is the only documented beta release entrypoint. Before building it verifies a clean exact-tag source state, clean submodule pin, Developer ID identity, and one complete Tauri-supported notarization credential set. It then runs every release check, builds through Tauri, and verifies code signing, hardened runtime, stapling, and Gatekeeper acceptance.

The script reads credential presence but never logs values. Tauri receives credentials through the inherited environment and handles notarization. App Store Connect API credentials are preferred; Apple ID plus app-specific password remains supported by Tauri.

## Provenance

After verification, the command writes a checksum file beside the DMG and a JSON release manifest. The manifest deliberately contains no machine path, identity, account, or secret.

## Known external gate

This machine currently has no Developer ID Application identity, so a genuinely signed and notarized artifact cannot be produced locally until Apple release authority is installed. The ordinary DMG remains an internal unsigned test build. Phase 07 must also decide whether the first beta requires Ruby 4.0.2 as a prerequisite or packages a standalone runtime.
