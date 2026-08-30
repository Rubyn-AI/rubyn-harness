import assert from "node:assert/strict";
import { mkdtempSync, cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { credentialMode, projectRoot, validateReleaseContract } from "./release-contract.mjs";

function fixture() {
  const root = mkdtempSync(path.join(tmpdir(), "rubyn-release-contract-"));
  for (const relativePath of ["package.json", ".nvmrc", ".ruby-version", "rust-toolchain.toml", "pnpm-lock.yaml", "src-tauri/Cargo.toml", "src-tauri/Cargo.lock", "src-tauri/tauri.conf.json", "src-tauri/Entitlements.plist", "engine/rubyn-code/Gemfile.lock"]) {
    const source = path.join(projectRoot, relativePath);
    const destination = path.join(root, relativePath);
    mkdirSync(path.dirname(destination), { recursive: true });
    cpSync(source, destination, { recursive: true });
  }
  return root;
}

test("the checked-in release contract is internally consistent", () => {
  assert.deepEqual(validateReleaseContract().errors, []);
});

test("version drift and unsafe release settings fail closed", () => {
  const root = fixture();
  const tauriPath = path.join(root, "src-tauri/tauri.conf.json");
  const tauri = JSON.parse(readFileSync(tauriPath, "utf8"));
  tauri.version = "9.9.9";
  tauri.bundle.macOS.hardenedRuntime = false;
  writeFileSync(tauriPath, JSON.stringify(tauri));
  writeFileSync(path.join(root, "src-tauri/Entitlements.plist"), "<dict><key>com.apple.security.get-task-allow</key><true/></dict>");

  const errors = validateReleaseContract(root).errors.join("\n");
  assert.match(errors, /same application version/);
  assert.match(errors, /Hardened runtime/);
  assert.match(errors, /must remain empty/);
});

test("notarization credentials must be complete without exposing their values", () => {
  assert.equal(credentialMode({}), "missing");
  assert.equal(credentialMode({ APPLE_NOTARY_KEYCHAIN_PROFILE: "rubyn-release" }), "keychain-profile");
  assert.equal(credentialMode({ APPLE_ID: "person@example.com" }), "unsafe-apple-id-environment");
  assert.equal(credentialMode({ APPLE_ID: "person@example.com", APPLE_PASSWORD: "secret", APPLE_TEAM_ID: "TEAM" }), "unsafe-apple-id-environment");
  assert.equal(credentialMode({ APPLE_API_ISSUER: "issuer", APPLE_API_KEY: "key", APPLE_API_KEY_PATH: "/private/key.p8" }), "app-store-connect-api");
});
