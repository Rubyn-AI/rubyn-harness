import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { inspectBuildBudget } from "./check-build-budget.mjs";

function fixture(javascript, css = "") {
  const root = mkdtempSync(path.join(tmpdir(), "rubyn-build-budget-"));
  const assets = path.join(root, "assets");
  mkdirSync(assets);
  writeFileSync(path.join(assets, "app.js"), javascript);
  writeFileSync(path.join(assets, "app.css"), css);
  return root;
}

test("accepts assets within raw and compressed budgets", () => {
  const result = inspectBuildBudget(fixture("const ready = true;", "body{color:white}"), {
    javascriptRaw: 100,
    javascriptGzip: 100,
    cssRaw: 100,
    cssGzip: 100,
  });
  assert.deepEqual(result.errors, []);
});

test("reports the measured asset and its exact budget", () => {
  const result = inspectBuildBudget(fixture("x".repeat(101)), {
    javascriptRaw: 100,
    javascriptGzip: 100,
    cssRaw: 100,
    cssGzip: 100,
  });
  assert.deepEqual(result.errors, ["javascriptRaw is 101 bytes; budget is 100 bytes."]);
});

test("fails clearly when production assets have not been built", () => {
  const result = inspectBuildBudget(path.join(tmpdir(), "rubyn-missing-build-budget"));
  assert.match(result.errors[0], /pnpm build/);
});
