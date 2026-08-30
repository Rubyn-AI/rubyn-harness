import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const nativePerformanceBudgets = {
  cleanLaunchMs: 3_000,
  trustedProjectOpenMs: 4_000,
};

export function summarizePerformance(samples) {
  const ordered = [...samples].sort((left, right) => left.nativeElapsedMs - right.nativeElapsedMs);
  return {
    samples: ordered,
    medianNativeMs: ordered[Math.floor(ordered.length / 2)].nativeElapsedMs,
    maximumNativeMs: ordered.at(-1).nativeElapsedMs,
    medianFrontendMs: [...samples].sort((left, right) => left.frontendElapsedMs - right.frontendElapsedMs)[Math.floor(samples.length / 2)].frontendElapsedMs,
  };
}

function argument(name, fallback) {
  const prefix = `--${name}=`;
  return process.argv.find((value) => value.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function safeOutputDirectory(candidate) {
  if (!path.isAbsolute(candidate) || !path.basename(candidate).startsWith("rubyn-harness-test-")) return false;
  const parent = realpathSync(path.dirname(candidate));
  return [realpathSync(os.tmpdir()), realpathSync("/private/tmp")]
    .some((root) => parent === root || parent.startsWith(`${root}${path.sep}`));
}

async function waitForRecord(file, timeoutMs = 15_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(file)) {
      const lines = readFileSync(file, "utf8").trim().split("\n").filter(Boolean);
      if (lines.length) return JSON.parse(lines.at(-1));
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error(`Native readiness marker was not written within ${timeoutMs}ms.`);
}

function trustedState(projectPath) {
  return {
    version: 6,
    app_state: {
      preferences: { defaultModel: "rubyn", parallelLimit: 3, autoCompaction: true, yoloEnabled: false },
      recentProjects: [{ path: projectPath, name: path.basename(projectPath) }],
      onboardingVersion: 1,
      trustedProjectPaths: [projectPath],
    },
  };
}

async function measureOnce(executable, root, label, projectPath, index) {
  const appData = path.join(root, `rubyn-harness-test-${label}-${index}`);
  mkdirSync(appData, { mode: 0o700 });
  if (projectPath) writeFileSync(path.join(appData, "harness-database.json"), `${JSON.stringify(trustedState(projectPath), null, 2)}\n`, { mode: 0o600 });
  const recordFile = path.join(appData, "native-performance.jsonl");
  const child = spawn(executable, [], {
    env: { ...process.env, RUBYN_HARNESS_TEST_APP_DATA_DIR: appData },
    stdio: "ignore",
  });
  try {
    const record = await waitForRecord(recordFile);
    if (record.projectOpened !== Boolean(projectPath)) {
      throw new Error(`${label} readiness reported projectOpened=${record.projectOpened}.`);
    }
    return record;
  } finally {
    child.kill("SIGTERM");
    await Promise.race([
      new Promise((resolve) => child.once("exit", resolve)),
      new Promise((resolve) => setTimeout(resolve, 2_000)),
    ]);
    if (child.exitCode === null) child.kill("SIGKILL");
  }
}

async function main() {
  if (process.platform !== "darwin") throw new Error("Native performance acceptance currently supports macOS only.");
  const executable = path.resolve(argument("app", "src-tauri/target/universal-apple-darwin/release/bundle/macos/Rubyn Harness.app/Contents/MacOS/Rubyn Harness"));
  const projectArgument = argument("project");
  if (!projectArgument) throw new Error("Pass an isolated Rails fixture with --project=/absolute/path.");
  const project = realpathSync(path.resolve(projectArgument));
  const runCount = Number(argument("runs", "3"));
  if (!Number.isInteger(runCount) || runCount < 1 || runCount > 10) throw new Error("--runs must be an integer from 1 to 10.");
  const outputArgument = argument("output");
  const output = outputArgument ?? mkdtempSync(path.join(os.tmpdir(), "rubyn-harness-test-performance-"));
  if (!existsSync(executable)) throw new Error(`Packaged app executable not found: ${executable}`);
  if (!safeOutputDirectory(output)) throw new Error("Performance output must be an absolute rubyn-harness-test-* directory under system temporary storage.");
  mkdirSync(output, { recursive: true, mode: 0o700 });

  const clean = [];
  const trustedProject = [];
  for (let index = 1; index <= runCount; index += 1) clean.push(await measureOnce(executable, output, "clean", undefined, index));
  for (let index = 1; index <= runCount; index += 1) trustedProject.push(await measureOnce(executable, output, "project", project, index));

  const report = {
    recordedAt: new Date().toISOString(),
    hardware: { platform: os.platform(), release: os.release(), architecture: os.arch(), cpu: os.cpus()[0]?.model ?? "unknown", logicalCpus: os.cpus().length, memoryBytes: os.totalmem() },
    executable,
    project,
    budgets: nativePerformanceBudgets,
    cleanLaunch: summarizePerformance(clean),
    trustedProjectOpen: summarizePerformance(trustedProject),
  };
  const failures = [];
  if (report.cleanLaunch.maximumNativeMs > nativePerformanceBudgets.cleanLaunchMs) failures.push(`clean launch maximum ${report.cleanLaunch.maximumNativeMs.toFixed(1)}ms exceeds ${nativePerformanceBudgets.cleanLaunchMs}ms`);
  if (report.trustedProjectOpen.maximumNativeMs > nativePerformanceBudgets.trustedProjectOpenMs) failures.push(`trusted project open maximum ${report.trustedProjectOpen.maximumNativeMs.toFixed(1)}ms exceeds ${nativePerformanceBudgets.trustedProjectOpenMs}ms`);
  report.passed = failures.length === 0;
  report.failures = failures;
  const reportPath = path.join(output, "native-performance-report.json");
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, { mode: 0o600 });
  console.log(`Native performance ${report.passed ? "passed" : "failed"}: clean ${report.cleanLaunch.medianNativeMs.toFixed(1)}ms median / ${report.cleanLaunch.maximumNativeMs.toFixed(1)}ms max; project ${report.trustedProjectOpen.medianNativeMs.toFixed(1)}ms median / ${report.trustedProjectOpen.maximumNativeMs.toFixed(1)}ms max.`);
  console.log(reportPath);
  if (!report.passed) process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main().catch((error) => { console.error(error.message); process.exitCode = 1; });
