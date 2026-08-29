import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const script = join(repositoryRoot, "scripts", "prepare-beta-fixture.mjs");

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function createSource(root) {
  const source = join(root, "source");
  mkdirSync(join(source, "config"), { recursive: true });
  mkdirSync(join(source, "app", "models"), { recursive: true });
  mkdirSync(join(source, "app", "controllers"), { recursive: true });
  mkdirSync(join(source, ".rubyn"), { recursive: true });
  mkdirSync(join(source, "log"), { recursive: true });
  mkdirSync(join(source, "tmp", "cache"), { recursive: true });
  writeFileSync(join(source, "Gemfile"), 'gem "rails"\ngem "rubyn", path: "../rubyn"\n');
  writeFileSync(join(source, "Gemfile.lock"), "PATH\n  remote: ../rubyn\n  specs:\n    rubyn (0.1.8)\n\nGEM\n  remote: https://rubygems.org/\n  specs:\n    rails (8.1.2)\n\nDEPENDENCIES\n  rails\n  rubyn!\n");
  writeFileSync(join(source, "config", "application.rb"), "class TestApplication; end\n");
  writeFileSync(join(source, "app", "models", "post.rb"), "class Post; end\n");
  writeFileSync(join(source, "app", "controllers", "posts_controller.rb"), "class PostsController; end\n");
  writeFileSync(join(source, ".rubyn", "project.yml"), "project_id: prior\n");
  writeFileSync(join(source, "log", "development.log"), "generated\n");
  writeFileSync(join(source, "tmp", "cache", "state"), "generated\n");
  writeFileSync(join(source, ".env"), "SECRET=not-copied\n");
  git(source, "init", "-b", "main");
  git(source, "config", "user.name", "Fixture Test");
  git(source, "config", "user.email", "fixture@example.invalid");
  git(source, "add", "Gemfile", "Gemfile.lock", "config", "app", ".rubyn", "log", "tmp");
  git(source, "commit", "-m", "Fixture source");
  return source;
}

function prepare(source, revision, destination) {
  return execFileSync(process.execPath, [script, "--source", source, "--revision", revision, "--destination", destination], { encoding: "utf8" });
}

test("prepares independent clean clones without changing or leaking source state", () => {
  const root = mkdtempSync(join(tmpdir(), "rubyn-fixture-test-"));
  const source = createSource(root);
  const revision = git(source, "rev-parse", "HEAD");
  const beforeStatus = git(source, "status", "--porcelain=v1", "--untracked-files=all");
  const first = join(root, "acceptance-one");
  const second = join(root, "acceptance-two");

  const preparationOutput = prepare(source, revision, first);
  prepare(source, revision, second);

  assert.match(preparationOutput, new RegExp(`Source revision: ${revision}`));
  assert.equal(git(source, "status", "--porcelain=v1", "--untracked-files=all"), beforeStatus);
  assert.equal(git(first, "branch", "--show-current"), "rubyn-acceptance");
  assert.equal(git(first, "config", "--get", "rubynHarness.sourceRevision"), revision);
  assert.equal(git(first, "remote", "get-url", "--push", "origin"), "disabled://rubyn-harness-acceptance");
  assert.equal(git(first, "status", "--porcelain"), "");
  assert.equal(git(first, "rev-parse", "HEAD^{tree}"), git(second, "rev-parse", "HEAD^{tree}"));
  assert.equal(existsSync(join(first, ".env")), false);
  assert.equal(existsSync(join(first, ".rubyn")), false);
  assert.equal(existsSync(join(first, "log", "development.log")), false);
  assert.equal(existsSync(join(first, "tmp", "cache", "state")), false);
  assert.doesNotMatch(readFileSync(join(first, "Gemfile"), "utf8"), /path: "\.\.\/rubyn"/);
  assert.doesNotMatch(readFileSync(join(first, "Gemfile.lock"), "utf8"), /^PATH$/m);
  assert.equal(readFileSync(join(source, ".env"), "utf8"), "SECRET=not-copied\n");
});

test("refuses an existing destination without changing it", () => {
  const root = mkdtempSync(join(tmpdir(), "rubyn-fixture-refusal-"));
  const source = createSource(root);
  const revision = git(source, "rev-parse", "HEAD");
  const destination = join(root, "already-here");
  mkdirSync(destination);
  writeFileSync(join(destination, "sentinel"), "keep\n");

  const result = spawnSync(process.execPath, [script, "--source", source, "--revision", revision, "--destination", destination], { encoding: "utf8" });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /destination already exists/);
  assert.equal(readFileSync(join(destination, "sentinel"), "utf8"), "keep\n");
});
