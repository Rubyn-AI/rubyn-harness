import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import path from "node:path";
import { assertReleaseContract, credentialMode, projectRoot } from "./release-contract.mjs";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd || projectRoot,
    encoding: "utf8",
    env: options.env || process.env,
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.status !== 0) throw new Error(`${command} failed with exit code ${result.status}.`);
  return `${result.stdout || ""}${result.stderr || ""}`.trim();
}

function requireReleaseAuthority(versions) {
  if (process.platform !== "darwin") throw new Error("Signed macOS releases must be built on macOS.");
  const identity = process.env.APPLE_SIGNING_IDENTITY?.trim();
  if (!identity?.startsWith("Developer ID Application:")) throw new Error("APPLE_SIGNING_IDENTITY must name a Developer ID Application certificate.");
  const identities = run("security", ["find-identity", "-v", "-p", "codesigning"], { capture: true });
  if (!identities.includes(identity)) throw new Error("APPLE_SIGNING_IDENTITY is not available in the active keychain.");

  const credentials = credentialMode(process.env);
  if (credentials === "missing") throw new Error("Notarization credentials are missing. Configure an App Store Connect API key or notarytool Keychain profile.");
  if (credentials === "incomplete") throw new Error("Notarization credentials are incomplete.");
  if (credentials === "unsafe-apple-id-environment") throw new Error("Apple ID passwords are not accepted from the environment. Store notarization credentials in Keychain and set APPLE_NOTARY_KEYCHAIN_PROFILE.");
  if (credentials === "app-store-connect-api" && !existsSync(process.env.APPLE_API_KEY_PATH)) throw new Error("APPLE_API_KEY_PATH does not point to a readable key file.");

  const status = run("git", ["status", "--porcelain"], { capture: true });
  if (status) throw new Error("Release builds require a clean worktree.");
  const submodules = run("git", ["submodule", "status", "--recursive"], { capture: true });
  if (submodules.split("\n").some((line) => /^[+-U]/.test(line))) throw new Error("Release builds require initialized submodules at the recorded commits.");
  const tag = run("git", ["describe", "--tags", "--exact-match", "HEAD"], { capture: true });
  if (tag !== `v${versions.app}`) throw new Error(`Release commit must be tagged v${versions.app}.`);
  return { credentials, identity, tag };
}

function notarize(file, authority) {
  const args = ["notarytool", "submit", file];
  if (authority.credentials === "keychain-profile") {
    args.push("--keychain-profile", process.env.APPLE_NOTARY_KEYCHAIN_PROFILE);
  } else {
    args.push("--key", process.env.APPLE_API_KEY_PATH, "--key-id", process.env.APPLE_API_KEY, "--issuer", process.env.APPLE_API_ISSUER);
  }
  args.push("--wait", "--timeout", "30m");
  run("xcrun", args);
}

function signAndNotarize(appPath, dmgPath, dmgDirectory, authority) {
  run("codesign", ["--force", "--deep", "--strict", "--options", "runtime", "--timestamp", "--entitlements", path.join(projectRoot, "src-tauri/Entitlements.plist"), "--sign", authority.identity, appPath]);

  const temporaryRoot = mkdtempSync(path.join(tmpdir(), "rubyn-harness-release-"));
  try {
    const archivePath = path.join(temporaryRoot, "Rubyn Harness.zip");
    run("ditto", ["-c", "-k", "--keepParent", appPath, archivePath]);
    notarize(archivePath, authority);
    run("xcrun", ["stapler", "staple", appPath]);

    const dmgSource = path.join(temporaryRoot, "dmg-source");
    mkdirSync(dmgSource);
    run("ditto", [appPath, path.join(dmgSource, path.basename(appPath))]);
    rmSync(dmgPath, { force: true });
    run("bash", [
      path.join(dmgDirectory, "bundle_dmg.sh"),
      "--volname", "Rubyn Harness",
      "--icon", path.basename(appPath), "180", "170",
      "--hide-extension", path.basename(appPath),
      "--app-drop-link", "480", "170",
      "--codesign", authority.identity,
      dmgPath,
      dmgSource,
    ]);
    notarize(dmgPath, authority);
    run("xcrun", ["stapler", "staple", dmgPath]);
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
}

function sha256(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function verifyArtifact(appPath, dmgPath) {
  run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath]);
  const signature = run("codesign", ["-dvvv", appPath], { capture: true });
  if (!signature.includes("Authority=Developer ID Application:")) throw new Error("The application is not signed with Developer ID Application.");
  if (!/flags=.*runtime/.test(signature)) throw new Error("The application signature does not enable hardened runtime.");
  const executable = path.join(appPath, "Contents/MacOS/rubyn-harness");
  const architectures = run("file", [executable], { capture: true });
  if (!architectures.includes("arm64") || !architectures.includes("x86_64")) throw new Error("The release executable is not universal (arm64 and x86_64).");
  run("xcrun", ["stapler", "validate", appPath]);
  run("xcrun", ["stapler", "validate", dmgPath]);
  run("spctl", ["--assess", "--type", "execute", "--verbose=4", appPath]);
}

function main() {
  const contract = assertReleaseContract();
  const authority = requireReleaseAuthority(contract.versions);
  console.log(`Building signed, notarized ${authority.tag} with ${authority.credentials} credentials.`);

  run("pnpm", ["install", "--frozen-lockfile"]);
  run("pnpm", ["run", "lint"]);
  run("pnpm", ["run", "test", "--", "--run"]);
  run("pnpm", ["run", "test:fixture"]);
  run("pnpm", ["run", "test:release"]);
  run("pnpm", ["run", "build"]);
  run("pnpm", ["run", "test:performance"]);
  run("pnpm", ["run", "performance:check"]);
  run("cargo", ["fmt", "--manifest-path", "src-tauri/Cargo.toml", "--check"]);
  run("cargo", ["clippy", "--manifest-path", "src-tauri/Cargo.toml", "--all-targets", "--all-features", "--", "-D", "warnings"]);
  run("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml"]);
  run("bundle", ["exec", "rspec"], { cwd: path.join(projectRoot, "engine/rubyn-code") });
  const unsignedBuildEnvironment = { ...process.env };
  for (const name of ["APPLE_SIGNING_IDENTITY", "APPLE_NOTARY_KEYCHAIN_PROFILE", "APPLE_API_ISSUER", "APPLE_API_KEY", "APPLE_API_KEY_PATH", "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"]) delete unsignedBuildEnvironment[name];
  run("pnpm", ["tauri", "build", "--target", "universal-apple-darwin", "--bundles", "app,dmg"], { env: unsignedBuildEnvironment });

  const bundleRoot = path.join(projectRoot, "src-tauri/target/universal-apple-darwin/release/bundle");
  const appPath = path.join(bundleRoot, "macos/Rubyn Harness.app");
  const dmgDirectory = path.join(bundleRoot, "dmg");
  const dmgs = readdirSync(dmgDirectory).filter((name) => name.endsWith(".dmg"));
  if (!existsSync(appPath) || dmgs.length !== 1) throw new Error("Expected exactly one release application and DMG.");
  const dmgPath = path.join(dmgDirectory, dmgs[0]);
  signAndNotarize(appPath, dmgPath, dmgDirectory, authority);
  verifyArtifact(appPath, dmgPath);

  const commit = run("git", ["rev-parse", "HEAD"], { capture: true });
  const engineCommit = run("git", ["-C", "engine/rubyn-code", "rev-parse", "HEAD"], { capture: true });
  const digest = sha256(dmgPath);
  const checksumPath = `${dmgPath}.sha256`;
  writeFileSync(checksumPath, `${digest}  ${path.basename(dmgPath)}\n`, { mode: 0o644 });
  const manifestPath = path.join(dmgDirectory, `Rubyn-Harness-${contract.versions.app}-release.json`);
  writeFileSync(manifestPath, JSON.stringify({
    schemaVersion: 1,
    version: contract.versions.app,
    tag: authority.tag,
    commit,
    engineCommit,
    artifact: path.basename(dmgPath),
    sha256: digest,
    architectures: ["arm64", "x86_64"],
    minimumMacOS: "13.0",
    toolchains: contract.tools,
  }, null, 2) + "\n", { mode: 0o644 });
  console.log(`Release verified: ${dmgPath}`);
  console.log(`Checksum: ${checksumPath}`);
  console.log(`Provenance: ${manifestPath}`);
}

try {
  main();
} catch (error) {
  console.error(`Release refused: ${error.message}`);
  process.exitCode = 1;
}
