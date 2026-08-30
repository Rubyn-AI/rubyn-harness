import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { gzipSync } from "node:zlib";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export const buildBudgets = Object.freeze({
  javascriptRaw: 900_000,
  javascriptGzip: 250_000,
  cssRaw: 110_000,
  cssGzip: 25_000,
});

function assetFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const candidate = path.join(directory, entry.name);
    return entry.isDirectory() ? assetFiles(candidate) : [candidate];
  });
}

export function inspectBuildBudget(distDirectory = path.join(projectRoot, "dist"), budgets = buildBudgets) {
  if (!existsSync(distDirectory)) return { errors: ["Production assets are missing. Run `pnpm build` first."], totals: {} };
  const files = assetFiles(distDirectory);
  const totals = {
    javascriptRaw: 0,
    javascriptGzip: 0,
    cssRaw: 0,
    cssGzip: 0,
  };
  for (const file of files) {
    const key = file.endsWith(".js") ? "javascript" : file.endsWith(".css") ? "css" : undefined;
    if (!key) continue;
    const contents = readFileSync(file);
    totals[`${key}Raw`] += statSync(file).size;
    totals[`${key}Gzip`] += gzipSync(contents).length;
  }
  const errors = Object.entries(budgets)
    .filter(([name, maximum]) => totals[name] > maximum)
    .map(([name, maximum]) => `${name} is ${totals[name]} bytes; budget is ${maximum} bytes.`);
  return { errors, totals };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = inspectBuildBudget();
  if (result.errors.length) {
    console.error("Production asset budget failed:\n" + result.errors.map((error) => `- ${error}`).join("\n"));
    process.exitCode = 1;
  } else {
    console.log(`Production asset budget passed: JS ${result.totals.javascriptRaw} raw / ${result.totals.javascriptGzip} gzip bytes; CSS ${result.totals.cssRaw} raw / ${result.totals.cssGzip} gzip bytes.`);
  }
}
