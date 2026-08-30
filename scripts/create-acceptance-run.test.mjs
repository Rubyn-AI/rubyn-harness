import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdtempSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const script = join(repositoryRoot, "scripts", "create-acceptance-run.mjs");

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function createSource(root) {
  const source = join(root, "source");
  mkdirSync(join(source, "config"), { recursive: true });
  mkdirSync(join(source, "app", "models"), { recursive: true });
  mkdirSync(join(source, "app", "controllers"), { recursive: true });
  writeFileSync(join(source, "Gemfile"), 'gem "rails"\n');
  writeFileSync(join(source, "Gemfile.lock"), "GEM\n  remote: https://rubygems.org/\n  specs:\n    rails (8.1.2)\n\nDEPENDENCIES\n  rails\n");
  writeFileSync(join(source, "config", "application.rb"), "class TestApplication; end\n");
  writeFileSync(join(source, "app", "models", "post.rb"), "class Post; end\n");
  writeFileSync(join(source, "app", "controllers", "posts_controller.rb"), "class PostsController; end\n");
  git(source, "init", "-b", "main");
  git(source, "config", "user.name", "Acceptance Test");
  git(source, "config", "user.email", "acceptance@example.invalid");
  git(source, "add", ".");
  git(source, "commit", "-m", "Fixture source");
  writeFileSync(join(source, "local-note"), "preexisting untracked state\n");
  return source;
}

test("creates an isolated, traceable acceptance run without changing its source", () => {
  const root = mkdtempSync(join(tmpdir(), "rubyn-harness-test-wrapper-"));
  const source = createSource(root);
  const destination = join(root, "rubyn-harness-test-run-one");
  const revision = git(source, "rev-parse", "HEAD");
  const statusBefore = git(source, "status", "--porcelain=v1", "--untracked-files=all");

  const output = execFileSync(process.execPath, [script, "--source", source, "--revision", revision, "--destination", destination], { encoding: "utf8" });
  const manifestPath = join(destination, "acceptance-run.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

  assert.match(output, /Acceptance run:/);
  assert.equal(manifest.schemaVersion, 1);
  assert.equal(manifest.harnessCommit, git(repositoryRoot, "rev-parse", "HEAD"));
  assert.equal(manifest.engineCommit, git(join(repositoryRoot, "engine", "rubyn-code"), "rev-parse", "HEAD"));
  assert.equal(manifest.databaseSchemaVersion, 9);
  assert.equal(manifest.fixture.source, "local-git-checkout");
  assert.equal(manifest.fixture.requestedRevision, revision);
  assert.equal(manifest.fixture.sourceSnapshot.head, revision);
  assert.equal(manifest.fixture.sourceSnapshot.statusEntryCount, 1);
  assert.equal(manifest.fixture.branch, "rubyn-acceptance");
  assert.equal(manifest.fixture.pushUrl, "disabled://rubyn-harness-acceptance");
  assert.equal(manifest.paths.project, join(destination, "project"));
  assert.equal(manifest.paths.appData, join(destination, "rubyn-harness-test-app-data"));
  assert.equal(lstatSync(manifest.paths.appData).isDirectory(), true);
  assert.equal(statSync(manifestPath).mode & 0o777, 0o600);
  assert.deepEqual(Object.keys(manifest.checkpoints), ["A", "B", "C", "D", "E", "F", "G"]);
  assert.equal(Object.values(manifest.checkpoints).every(({ status }) => status === "pending"), true);
  assert.equal(git(manifest.paths.project, "status", "--porcelain"), "");
  assert.equal(git(source, "status", "--porcelain=v1", "--untracked-files=all"), statusBefore);
});

test("refuses an existing destination without changing it", () => {
  const root = mkdtempSync(join(tmpdir(), "rubyn-harness-test-wrapper-refusal-"));
  const source = createSource(root);
  const destination = join(root, "rubyn-harness-test-already-here");
  mkdirSync(destination);
  writeFileSync(join(destination, "sentinel"), "keep\n");

  const result = spawnSync(process.execPath, [script, "--source", source, "--revision", git(source, "rev-parse", "HEAD"), "--destination", destination], { encoding: "utf8" });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /destination already exists/);
  assert.equal(readFileSync(join(destination, "sentinel"), "utf8"), "keep\n");
});

test("refuses destinations outside system temporary directories", () => {
  const destination = join(repositoryRoot, "rubyn-harness-test-unsafe-destination");
  const result = spawnSync(process.execPath, [script, "--destination", destination], { encoding: "utf8" });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /beneath a system temporary directory/);
  assert.equal(existsSync(destination), false);
});
