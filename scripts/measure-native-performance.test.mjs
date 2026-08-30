import assert from "node:assert/strict";
import test from "node:test";
import { summarizePerformance } from "./measure-native-performance.mjs";

test("summarizes native performance without hiding the slowest sample", () => {
  const summary = summarizePerformance([
    { nativeElapsedMs: 800, frontendElapsedMs: 300 },
    { nativeElapsedMs: 400, frontendElapsedMs: 100 },
    { nativeElapsedMs: 600, frontendElapsedMs: 200 },
  ]);
  assert.equal(summary.medianNativeMs, 600);
  assert.equal(summary.maximumNativeMs, 800);
  assert.equal(summary.medianFrontendMs, 200);
});
