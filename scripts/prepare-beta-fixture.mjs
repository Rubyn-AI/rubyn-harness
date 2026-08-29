#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, parse, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const metadataPath = join(repositoryRoot, "acceptance", "rubyn-test.json");
const metadata = JSON.parse(readFileSync(metadataPath, "utf8"));

function fail(message) {
  process.stderr.write(`Fixture preparation failed: ${message}\n`);
  process.exit(1);
}

function parseArguments(argv) {
  const options = { verifyRails: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--verify-rails") {
      options.verifyRails = true;
      continue;
    }
    if (!["--source", "--revision", "--destination"].includes(argument)) {
      fail(`unknown argument ${argument}`);
    }
    const value = argv[index + 1];
    if (!value) fail(`${argument} requires a value`);
    options[argument.slice(2)] = value;
    index += 1;
  }
  if (!options.destination) fail("--destination is required");
  return options;
}

function command(program, args, options = {}) {
  const result = spawnSync(program, args, {
    cwd: options.cwd,
    encoding: "utf8",
    env: options.env || process.env,
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.error) fail(`${program} could not start: ${result.error.message}`);
  if (result.status !== 0 && !options.allowFailure) {
    const detail = options.capture ? (result.stderr || result.stdout || "").trim() : "";
    fail(`${program} ${args.join(" ")} exited with ${result.status}${detail ? `: ${detail}` : ""}`);
  }
  return result;
}

function git(projectPath, args, options = {}) {
  return command("git", ["-C", projectPath, ...args], { ...options, capture: options.capture ?? true });
}

function output(result) {
  return result.stdout.trim();
}

function trackedFiles(projectPath) {
  return output(git(projectPath, ["ls-files", "-z"], { capture: true })).split("\0").filter(Boolean);
}

function assertSafeDestination(destination) {
  if (!isAbsolute(destination)) fail("--destination must be an absolute path");
  if (destination === parse(destination).root) fail("--destination cannot be a filesystem root");
  if (existsSync(destination)) fail(`destination already exists: ${destination}`);
  const parent = dirname(destination);
  mkdirSync(parent, { recursive: true });
  if (!lstatSync(parent).isDirectory()) fail(`destination parent is not a directory: ${parent}`);
}

function assertNoTrackedSecrets(projectPath) {
  const forbidden = trackedFiles(projectPath).filter((path) => {
    const name = path.split("/").at(-1);
    return name === ".env" || (name.startsWith(".env.") && name !== ".env.example");
  });
  if (forbidden.length) fail(`fixture tracks environment-secret files: ${forbidden.join(", ")}`);
}

function removeTrackedGeneratedState(projectPath) {
  const generated = trackedFiles(projectPath).filter((path) => {
    if (path.endsWith("/.keep")) return false;
    return path === ".rubyn" || path.startsWith(".rubyn/") || path === ".rubyn-code" || path.startsWith(".rubyn-code/") || path.startsWith("log/") || path.startsWith("tmp/") || path.startsWith("storage/");
  });
  if (!generated.length) return false;
  git(projectPath, ["rm", "-f", "--", ...generated], { capture: true });
  return true;
}

function removeFixtureOnlyRubynDependency(projectPath) {
  const gemfilePath = join(projectPath, "Gemfile");
  const lockfilePath = join(projectPath, "Gemfile.lock");
  const gemfile = readFileSync(gemfilePath, "utf8");
  const nextGemfile = gemfile.replace(/^gem ["']rubyn["'], path: ["']\.\.\/rubyn["']\s*\n/m, "");
  if (nextGemfile === gemfile) return false;

  const lockfile = readFileSync(lockfilePath, "utf8");
  const nextLockfile = lockfile
    .replace(/^PATH\n  remote: \.\.\/rubyn\n  specs:\n(?: {4}.*\n)+(?:\n|$)/m, "")
    .replace(/^  rubyn!\n/m, "");
  if (nextLockfile === lockfile) fail("Gemfile.lock did not contain the expected fixture-only Rubyn dependency");

  writeFileSync(gemfilePath, nextGemfile);
  writeFileSync(lockfilePath, nextLockfile);
  git(projectPath, ["add", "--", "Gemfile", "Gemfile.lock"], { capture: true });
  return true;
}

function verifyClone(projectPath, sourceRevision, acceptanceBranch) {
  const branch = output(git(projectPath, ["branch", "--show-current"]));
  if (branch !== acceptanceBranch) fail(`expected branch ${acceptanceBranch}, found ${branch || "detached HEAD"}`);
  const recordedRevision = output(git(projectPath, ["config", "--get", "rubynHarness.sourceRevision"]));
  if (recordedRevision !== sourceRevision) fail("recorded source revision does not match the requested revision");
  git(projectPath, ["merge-base", "--is-ancestor", sourceRevision, "HEAD"], { capture: true });
  const status = output(git(projectPath, ["status", "--porcelain"]));
  if (status) fail(`acceptance clone is not clean: ${status}`);
  const pushUrl = output(git(projectPath, ["remote", "get-url", "--push", "origin"]));
  if (pushUrl !== "disabled://rubyn-harness-acceptance") fail("acceptance clone still has a push-capable origin");
  assertNoTrackedSecrets(projectPath);
  for (const marker of metadata.expectedMarkers) {
    if (!existsSync(join(projectPath, marker))) fail(`Rails marker is missing: ${marker}`);
  }
}

function verifyRails(projectPath) {
  const hasRbenv = spawnSync("rbenv", ["--version"], { cwd: projectPath, encoding: "utf8" }).status === 0;
  const bundleProgram = hasRbenv ? "rbenv" : "bundle";
  const bundleArguments = hasRbenv ? ["exec", "bundle", "check"] : ["check"];
  const bundle = command(bundleProgram, bundleArguments, { cwd: projectPath, capture: true, allowFailure: true });
  if (bundle.status !== 0) {
    process.stdout.write("Rails baseline: SKIPPED — run `bundle install` in the acceptance clone, then rerun with --verify-rails.\n");
    return;
  }
  if (hasRbenv) {
    command("rbenv", ["exec", "ruby", "bin/rails", "test"], { cwd: projectPath });
  } else {
    command(join(projectPath, "bin", "rails"), ["test"], { cwd: projectPath });
  }
  process.stdout.write("Rails baseline: PASSED\n");
}

const options = parseArguments(process.argv.slice(2));
const source = options.source || metadata.sourceUrl;
const requestedRevision = options.revision || metadata.sourceRevision;
const destination = resolve(options.destination);

assertSafeDestination(destination);
let completed = false;
try {
  command("git", ["clone", "--no-checkout", "--", source, destination]);
  git(destination, ["checkout", "--detach", requestedRevision]);
  const sourceRevision = output(git(destination, ["rev-parse", "HEAD"]));
  git(destination, ["switch", "-c", metadata.acceptanceBranch]);
  assertNoTrackedSecrets(destination);
  git(destination, ["config", "user.name", "Rubyn Harness Acceptance"]);
  git(destination, ["config", "user.email", "acceptance@rubyn.invalid"]);
  git(destination, ["config", "rubynHarness.fixture", "true"]);
  git(destination, ["config", "rubynHarness.sourceRevision", sourceRevision]);
  git(destination, ["remote", "set-url", "--push", "origin", "disabled://rubyn-harness-acceptance"]);

  const generatedStateRemoved = removeTrackedGeneratedState(destination);
  const fixtureDependencyRemoved = removeFixtureOnlyRubynDependency(destination);
  const baselineChanged = generatedStateRemoved || fixtureDependencyRemoved;
  if (baselineChanged) {
    const sourceDate = output(git(destination, ["show", "-s", "--format=%aI", sourceRevision]));
    git(destination, ["commit", "-m", "Acceptance baseline: remove generated state"], {
      capture: true,
      env: { ...process.env, GIT_AUTHOR_DATE: sourceDate, GIT_COMMITTER_DATE: sourceDate },
    });
  }

  verifyClone(destination, sourceRevision, metadata.acceptanceBranch);
  if (options.verifyRails) verifyRails(destination);
  completed = true;
  process.stdout.write(`Acceptance clone: ${destination}\nSource revision: ${sourceRevision}\nBranch: ${metadata.acceptanceBranch}\n`);
} finally {
  if (!completed && existsSync(destination)) {
    rmSync(destination, { recursive: true, force: true });
  }
}
