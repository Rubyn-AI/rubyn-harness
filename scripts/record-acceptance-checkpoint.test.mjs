import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { recordAcceptanceCheckpoint } from "./record-acceptance-checkpoint.mjs";

const repositoryRoot = path.resolve(import.meta.dirname, "..");

function git(args) {
  return execFileSync("git", args, { cwd: repositoryRoot, encoding: "utf8" }).trim();
}

function cleanIdentity() {
  return {
    harnessCommit: git(["rev-parse", "HEAD"]),
    engineCommit: git(["-C", "engine/rubyn-code", "rev-parse", "HEAD"]),
    harnessStatus: "",
    engineStatus: "",
  };
}

function fixture(overrides = {}) {
  const root = mkdtempSync(path.join(tmpdir(), "rubyn-harness-test-recorder-"));
  mkdirSync(path.join(root, "project"));
  mkdirSync(path.join(root, "rubyn-harness-test-app-data"));
  const manifest = {
    schemaVersion: 1,
    harnessCommit: git(["rev-parse", "HEAD"]),
    engineCommit: git(["-C", "engine/rubyn-code", "rev-parse", "HEAD"]),
    fixture: { pushUrl: "disabled://rubyn-harness-acceptance" },
    paths: { project: path.join(root, "project"), appData: path.join(root, "rubyn-harness-test-app-data") },
    checkpoints: Object.fromEntries("ABCDEFG".split("").map((name) => [name, { status: "pending", evidence: [] }])),
    ...overrides,
  };
  const manifestPath = path.join(root, "acceptance-run.json");
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, { mode: 0o600 });
  return { root, manifestPath };
}

test("atomically records structured evidence without claiming the run passed", () => {
  const { root, manifestPath } = fixture();
  const manifest = recordAcceptanceCheckpoint({ run: root, checkpoint: "A", status: "passed", evidence: ["Reviewed native diff", "Integrated commit abc123"] }, cleanIdentity());
  assert.equal(manifest.status, "in-progress");
  assert.equal(manifest.checkpoints.A.evidence.length, 2);
  assert.equal(manifest.checkpoints.A.evidence[0].note, "Reviewed native diff");
  assert.equal(manifest.completedAt, undefined);
  assert.equal(statSync(manifestPath).mode & 0o777, 0o600);
});

test("marks the run passed only after every checkpoint has evidence", () => {
  const { root } = fixture();
  let manifest;
  for (const checkpoint of "ABCDEFG") {
    manifest = recordAcceptanceCheckpoint({ run: root, checkpoint, status: "passed", evidence: [`Native evidence ${checkpoint}`] }, cleanIdentity());
  }
  assert.equal(manifest.status, "passed");
  assert.match(manifest.completedAt, /^\d{4}-\d{2}-\d{2}T/);
});

test("refuses stale candidate identity without modifying the manifest", () => {
  const { root, manifestPath } = fixture({ harnessCommit: "stale" });
  const before = readFileSync(manifestPath);
  assert.throws(
    () => recordAcceptanceCheckpoint({ run: root, checkpoint: "A", status: "passed", evidence: ["Not enough"] }, cleanIdentity()),
    /does not match current/,
  );
  assert.deepEqual(readFileSync(manifestPath), before);
});

test("refuses unsafe, incomplete, or unsupported evidence requests", () => {
  const { root } = fixture();
  const identity = cleanIdentity();
  assert.throws(() => recordAcceptanceCheckpoint({ run: root, checkpoint: "H", status: "passed", evidence: ["No"] }, identity), /A–G/);
  assert.throws(() => recordAcceptanceCheckpoint({ run: root, checkpoint: "A", status: "passed", evidence: [] }, identity), /evidence note/);
  assert.throws(() => recordAcceptanceCheckpoint({ run: root, checkpoint: "A", status: "pending", evidence: ["No"] }, identity), /passed, failed, or blocked/);
  assert.throws(() => recordAcceptanceCheckpoint({ run: repositoryRoot, checkpoint: "A", status: "passed", evidence: ["No"] }, identity), /run directory name/);
});

test("refuses to record evidence from a dirty candidate", () => {
  const { root, manifestPath } = fixture();
  const before = readFileSync(manifestPath);
  assert.throws(
    () => recordAcceptanceCheckpoint(
      { run: root, checkpoint: "A", status: "passed", evidence: ["Observed"] },
      { ...cleanIdentity(), harnessStatus: " M src/App.tsx" },
    ),
    /worktree is not clean/,
  );
  assert.deepEqual(readFileSync(manifestPath), before);
});
