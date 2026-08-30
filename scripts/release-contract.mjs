import { existsSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

export const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function readJson(root, relativePath) {
  return JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));
}

function cargoVersion(contents) {
  return contents.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
}

function cargoRustVersion(contents) {
  return contents.match(/^rust-version\s*=\s*"([^"]+)"/m)?.[1];
}

function toolchainChannel(contents) {
  return contents.match(/^channel\s*=\s*"([^"]+)"/m)?.[1];
}

export function validateReleaseContract(root = projectRoot) {
  const errors = [];
  const packageJson = readJson(root, "package.json");
  const tauri = readJson(root, "src-tauri/tauri.conf.json");
  const cargo = readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8");
  const rustToolchain = readFileSync(path.join(root, "rust-toolchain.toml"), "utf8");
  const nvmVersion = readFileSync(path.join(root, ".nvmrc"), "utf8").trim();
  const rubyVersion = readFileSync(path.join(root, ".ruby-version"), "utf8").trim();
  const entitlements = readFileSync(path.join(root, "src-tauri/Entitlements.plist"), "utf8");
  const versions = {
    app: packageJson.version,
    node: packageJson.engines?.node,
    pnpm: packageJson.engines?.pnpm,
    ruby: rubyVersion,
    rust: cargoRustVersion(cargo),
  };

  if (tauri.version !== versions.app || cargoVersion(cargo) !== versions.app) {
    errors.push("package.json, Cargo.toml, and tauri.conf.json must use the same application version.");
  }
  if (nvmVersion !== versions.node) errors.push(".nvmrc must exactly match package.json engines.node.");
  if (toolchainChannel(rustToolchain) !== versions.rust) errors.push("rust-toolchain.toml must exactly match Cargo.toml rust-version.");
  for (const target of ["aarch64-apple-darwin", "x86_64-apple-darwin"]) {
    if (!rustToolchain.includes(`"${target}"`)) errors.push(`rust-toolchain.toml must install ${target}.`);
  }
  if (packageJson.packageManager?.split("+")[0] !== `pnpm@${versions.pnpm}`) errors.push("packageManager must exactly match engines.pnpm.");
  if (tauri.bundle?.active !== true) errors.push("Tauri bundling must be enabled.");
  if (!Array.isArray(tauri.bundle?.targets) || !["app", "dmg"].every((target) => tauri.bundle.targets.includes(target))) errors.push("Tauri bundle targets must include app and dmg.");
  if (tauri.bundle?.macOS?.minimumSystemVersion !== "13.0") errors.push("The beta deployment target must remain explicit at macOS 13.0.");
  if (tauri.bundle?.macOS?.hardenedRuntime !== true) errors.push("Hardened runtime must be explicitly enabled.");
  if (tauri.bundle?.macOS?.entitlements !== "Entitlements.plist") errors.push("The reviewed entitlements file must be configured.");
  if (!/<dict\s*\/>/i.test(entitlements)) errors.push("Release entitlements must remain empty.");
  if (!/^[a-zA-Z0-9.-]+$/.test(tauri.identifier || "")) errors.push("The macOS bundle identifier is invalid.");

  for (const relativePath of ["pnpm-lock.yaml", "src-tauri/Cargo.lock", "engine/rubyn-code/Gemfile.lock", "src-tauri/Entitlements.plist"]) {
    if (!existsSync(path.join(root, relativePath))) errors.push(`${relativePath} must be committed for deterministic releases.`);
  }
  for (const resource of ["../engine/rubyn-code/lib", "../engine/rubyn-code/exe", "../engine/rubyn-code/skills", "../engine/rubyn-code/db"]) {
    if (!Object.hasOwn(tauri.bundle?.resources || {}, resource)) errors.push(`Bundled engine resource ${resource} is missing.`);
  }

  return { errors, versions };
}

function commandVersion(command, args, matcher) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.status !== 0) return { error: `${command} is unavailable.` };
  const output = `${result.stdout || ""}${result.stderr || ""}`.trim();
  return { value: output.match(matcher)?.[1], output };
}

export function validateInstalledToolchains(expected) {
  const errors = [];
  const actual = { node: process.versions.node };
  const pnpm = commandVersion("pnpm", ["--version"], /^(\S+)/);
  const rust = commandVersion("rustc", ["--version"], /^rustc\s+(\S+)/);
  const ruby = commandVersion("ruby", ["--version"], /^ruby\s+(\S+)/);
  actual.pnpm = pnpm.value;
  actual.rust = rust.value;
  actual.ruby = ruby.value;

  for (const tool of ["node", "pnpm", "rust", "ruby"]) {
    if (actual[tool] !== expected[tool]) errors.push(`${tool} ${expected[tool]} is required; found ${actual[tool] || "unavailable"}.`);
  }
  return { errors, actual };
}

export function credentialMode(environment) {
  const apiFields = ["APPLE_API_ISSUER", "APPLE_API_KEY", "APPLE_API_KEY_PATH"];
  const appleIdFields = ["APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"];
  const complete = (fields) => fields.every((field) => Boolean(environment[field]?.trim()));
  const partial = (fields) => fields.some((field) => Boolean(environment[field]?.trim()));
  if (environment.APPLE_NOTARY_KEYCHAIN_PROFILE?.trim()) return "keychain-profile";
  if (complete(apiFields)) return "app-store-connect-api";
  if (partial(apiFields)) return "incomplete";
  if (partial(appleIdFields)) return "unsafe-apple-id-environment";
  return "missing";
}

export function assertReleaseContract(root = projectRoot) {
  const contract = validateReleaseContract(root);
  const tools = validateInstalledToolchains(contract.versions);
  const errors = [...contract.errors, ...tools.errors];
  if (errors.length) throw new Error(errors.map((error) => `- ${error}`).join("\n"));
  return { versions: contract.versions, tools: tools.actual };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const result = assertReleaseContract();
    console.log(`Release contract ready for Rubyn Harness ${result.versions.app}.`);
    console.log(`Pinned toolchains: Node ${result.tools.node}, pnpm ${result.tools.pnpm}, Rust ${result.tools.rust}, Ruby ${result.tools.ruby}.`);
  } catch (error) {
    console.error("Release contract failed:\n" + error.message);
    process.exitCode = 1;
  }
}
