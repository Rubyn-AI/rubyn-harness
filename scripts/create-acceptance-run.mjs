#!/usr/bin/env node

import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, parse, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const fixtureScript = join(repositoryRoot, "scripts", "prepare-beta-fixture.mjs");
const fixtureMetadata = JSON.parse(readFileSync(join(repositoryRoot, "acceptance", "rubyn-test.json"), "utf8"));

function fail(message) {
  throw new Error(`Acceptance run refused: ${message}`);
}

function argumentsFor(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (key === "--") continue;
    if (!["--destination", "--source", "--revision"].includes(key)) fail(`unknown argument ${key}`);
    if (!argv[index + 1]) fail(`${key} requires a value`);
    options[key.slice(2)] = argv[index + 1];
    index += 1;
  }
  if (!options.destination) fail("--destination is required");
  return options;
}

function git(args, cwd = repositoryRoot) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function localSourceSnapshot(source) {
  if (!existsSync(source) || !lstatSync(source).isDirectory()) return undefined;
  const head = git(["rev-parse", "HEAD"], source);
  const status = git(["status", "--porcelain=v1", "--untracked-files=all"], source);
  return { head, statusSha256: sha256(status), statusEntryCount: status ? status.split("\n").length : 0 };
}

function assertSafeRunRoot(destination) {
  if (!isAbsolute(destination)) fail("--destination must be an absolute path");
  if (destination === parse(destination).root) fail("--destination cannot be a filesystem root");
  if (!destination.split("/").at(-1)?.startsWith("rubyn-harness-test-")) fail("destination name must start with rubyn-harness-test-");
  if (existsSync(destination)) fail(`destination already exists: ${destination}`);
  const parent = dirname(destination);
  mkdirSync(parent, { recursive: true });
  const canonicalParent = realpathSync(parent);
  const allowedRoots = [tmpdir(), "/private/tmp"].filter(existsSync).map((root) => realpathSync(root));
  if (!allowedRoots.some((root) => {
    const pathFromRoot = relative(root, canonicalParent);
    return !pathFromRoot.startsWith("..") && !isAbsolute(pathFromRoot);
  })) fail("destination must be beneath a system temporary directory");
}

function databaseSchemaVersion() {
  const store = readFileSync(join(repositoryRoot, "src-tauri", "src", "store.rs"), "utf8");
  const version = Number(store.match(/^const DATABASE_VERSION: u32 = (\d+);$/m)?.[1]);
  if (!Number.isInteger(version)) fail("could not read the database schema version");
  return version;
}

export function createAcceptanceRun(options) {
  const destination = resolve(options.destination);
  const source = options.source || fixtureMetadata.sourceUrl;
  const revision = options.revision || fixtureMetadata.sourceRevision;
  assertSafeRunRoot(destination);
  const sourceBefore = localSourceSnapshot(source);
  let completed = false;
  try {
    mkdirSync(destination);
    const projectPath = join(destination, "project");
    const appDataPath = join(destination, "rubyn-harness-test-app-data");
    mkdirSync(appDataPath);
    const prepared = spawnSync(process.execPath, [fixtureScript, "--source", source, "--revision", revision, "--destination", projectPath], { encoding: "utf8" });
    if (prepared.status !== 0) fail((prepared.stderr || prepared.stdout || "fixture preparation failed").trim());
    const sourceAfter = localSourceSnapshot(source);
    if (JSON.stringify(sourceBefore) !== JSON.stringify(sourceAfter)) fail("the source fixture changed during acceptance preparation");

    const packageJson = JSON.parse(readFileSync(join(repositoryRoot, "package.json"), "utf8"));
    const manifest = {
      schemaVersion: 1,
      harnessVersion: packageJson.version,
      harnessCommit: git(["rev-parse", "HEAD"]),
      engineCommit: git(["-C", "engine/rubyn-code", "rev-parse", "HEAD"]),
      databaseSchemaVersion: databaseSchemaVersion(),
      fixture: {
        source: sourceBefore ? "local-git-checkout" : source,
        requestedRevision: revision,
        sourceSnapshot: sourceBefore,
        preparedRevision: git(["rev-parse", "HEAD"], projectPath),
        branch: git(["branch", "--show-current"], projectPath),
        pushUrl: git(["remote", "get-url", "--push", "origin"], projectPath),
      },
      paths: { project: projectPath, appData: appDataPath },
      checkpoints: Object.fromEntries("ABCDEFG".split("").map((name) => [name, { status: "pending", evidence: [] }])),
    };
    writeFileSync(join(destination, "acceptance-run.json"), JSON.stringify(manifest, null, 2) + "\n", { mode: 0o600 });
    completed = true;
    return manifest;
  } finally {
    if (!completed && existsSync(destination)) rmSync(destination, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const manifest = createAcceptanceRun(argumentsFor(process.argv.slice(2)));
    process.stdout.write(`Acceptance run: ${dirname(manifest.paths.project)}\nProject: ${manifest.paths.project}\nApp data: ${manifest.paths.appData}\nHarness: ${manifest.harnessVersion} @ ${manifest.harnessCommit}\nEngine: ${manifest.engineCommit}\nDatabase schema: ${manifest.databaseSchemaVersion}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
