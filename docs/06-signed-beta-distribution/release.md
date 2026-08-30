# macOS Beta Release

Use Node 22.22.2 (for example, `nvm use`), then run:

```bash
pnpm release:check
```

For an actual beta, install a valid `Developer ID Application` certificate, tag the clean release commit as `v<version>`, and configure either App Store Connect API credentials (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or Apple ID credentials (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`). Set `APPLE_SIGNING_IDENTITY` to the exact certificate name, then run:

```bash
pnpm release:macos
```

The command refuses partial credentials, dirty or untagged source, submodule drift, non-universal output, ad-hoc signatures, missing hardened runtime, unstapled artifacts, or Gatekeeper rejection. Successful output includes the universal DMG, a `.sha256` file, and a JSON provenance manifest beneath the universal release bundle directory.

Beta testers must install Ruby 4.0.6 with rbenv or Homebrew and, using that Ruby, run:

```bash
gem install rubyn-code
```

The installed gem supplies the runtime dependencies for the pinned Rubyn Code source inside Harness. A clean machine without this prerequisite is expected to show “Finish Rubyn runtime setup” and prevent repository selection or conversation launch until setup is complete.

Never share Apple credentials, `.p8` keys, `.p12` certificates, or keychain passwords in issues, diagnostics, release manifests, or application data.
