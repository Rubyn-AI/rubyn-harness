#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { existsSync, lstatSync, readFileSync, realpathSync, renameSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const checkpoints = new Set("ABCDEFG".split(""));
const statuses = new Set(["passed", "failed", "blocked"]);

function fail(message) {
  throw new Error(`Acceptance evidence refused: ${message}`);
}

function git(args, cwd = repositoryRoot) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function beneath(root, candidate) {
  const pathFromRoot = relative(root, candidate);
  return pathFromRoot === "" || (!pathFromRoot.startsWith("..") && !isAbsolute(pathFromRoot));
}

function assertSafeRunRoot(runRoot) {
  if (!isAbsolute(runRoot)) fail("--run must be an absolute path");
  if (!runRoot.split("/").at(-1)?.startsWith("rubyn-harness-test-")) fail("run directory name must start with rubyn-harness-test-");
  if (!existsSync(runRoot) || !lstatSync(runRoot).isDirectory()) fail("run directory does not exist");
  if (lstatSync(runRoot).isSymbolicLink()) fail("run directory cannot be a symlink");
  const canonical = realpathSync(runRoot);
  const allowedRoots = [tmpdir(), "/private/tmp"].filter(existsSync).map((root) => realpathSync(root));
  if (!allowedRoots.some((root) => beneath(root, canonical))) fail("run directory must be beneath system temporary storage");
  return canonical;
}

function parseArguments(argv) {
  const options = { evidence: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (key === "--") continue;
    if (!["--run", "--checkpoint", "--status", "--evidence"].includes(key)) fail(`unknown argument ${key}`);
    const value = argv[index + 1];
    if (!value) fail(`${key} requires a value`);
    if (key === "--evidence") options.evidence.push(value);
    else options[key.slice(2)] = value;
    index += 1;
  }
  return options;
}

function repositoryIdentity() {
  return {
    harnessCommit: git(["rev-parse", "HEAD"]),
    engineCommit: git(["-C", "engine/rubyn-code", "rev-parse", "HEAD"]),
    harnessStatus: git(["status", "--porcelain=v1", "--untracked-files=all"]),
    engineStatus: git(["-C", "engine/rubyn-code", "status", "--porcelain=v1", "--untracked-files=all"]),
  };
}

function validateManifest(manifest, runRoot, identity) {
  if (manifest.schemaVersion !== 1) fail(`unsupported manifest schema ${manifest.schemaVersion}`);
  if (!manifest.paths || !existsSync(manifest.paths.project) || realpathSync(manifest.paths.project) !== realpathSync(join(runRoot, "project"))) fail("manifest project path does not match the run directory");
  if (!existsSync(manifest.paths.appData) || realpathSync(manifest.paths.appData) !== realpathSync(join(runRoot, "rubyn-harness-test-app-data"))) fail("manifest app-data path does not match the run directory");
  if (manifest.fixture?.pushUrl !== "disabled://rubyn-harness-acceptance") fail("fixture push protection is missing");
  if (!manifest.checkpoints || [...checkpoints].some((name) => !manifest.checkpoints[name])) fail("manifest does not contain checkpoints A–G");
  if (manifest.harnessCommit !== identity.harnessCommit) fail(`manifest Harness commit ${manifest.harnessCommit} does not match current ${identity.harnessCommit}`);
  if (manifest.engineCommit !== identity.engineCommit) fail(`manifest engine commit ${manifest.engineCommit} does not match current ${identity.engineCommit}`);
  if (identity.harnessStatus) fail("Harness worktree is not clean");
  if (identity.engineStatus) fail("engine worktree is not clean");
}

export function recordAcceptanceCheckpoint(options, identity = repositoryIdentity()) {
  const runRoot = assertSafeRunRoot(resolve(options.run || ""));
  const checkpoint = String(options.checkpoint || "").toUpperCase();
  if (!checkpoints.has(checkpoint)) fail("--checkpoint must be one of A–G");
  if (!statuses.has(options.status)) fail("--status must be passed, failed, or blocked");
  const evidence = (options.evidence || []).map((note) => String(note).trim()).filter(Boolean);
  if (!evidence.length) fail("at least one non-empty --evidence note is required");
  const manifestPath = join(runRoot, "acceptance-run.json");
  if (!existsSync(manifestPath) || lstatSync(manifestPath).isSymbolicLink()) fail("acceptance-run.json is missing or unsafe");
  const original = readFileSync(manifestPath, "utf8");
  let manifest;
  try { manifest = JSON.parse(original); } catch { fail("acceptance-run.json is not valid JSON"); }
  validateManifest(manifest, runRoot, identity);

  const recordedAt = new Date().toISOString();
  const previousEvidence = Array.isArray(manifest.checkpoints[checkpoint].evidence) ? manifest.checkpoints[checkpoint].evidence : [];
  manifest.checkpoints[checkpoint] = {
    status: options.status,
    recordedAt,
    evidence: [...previousEvidence, ...evidence.map((note) => ({ recordedAt, note }))],
  };
  const allPassed = [...checkpoints].every((name) => manifest.checkpoints[name].status === "passed");
  manifest.status = allPassed ? "passed" : "in-progress";
  if (allPassed) manifest.completedAt = recordedAt;
  else delete manifest.completedAt;

  const temporaryPath = join(runRoot, `.acceptance-run.${randomUUID()}.tmp`);
  writeFileSync(temporaryPath, `${JSON.stringify(manifest, null, 2)}\n`, { mode: 0o600, flag: "wx" });
  renameSync(temporaryPath, manifestPath);
  return manifest;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const options = parseArguments(process.argv.slice(2));
    const manifest = recordAcceptanceCheckpoint(options);
    process.stdout.write(`Acceptance ${options.checkpoint.toUpperCase()}: ${options.status}\nRun status: ${manifest.status}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
