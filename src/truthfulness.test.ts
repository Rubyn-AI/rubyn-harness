import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const productionSource = ["./App.tsx", "./store.ts", "./bridge.ts"]
  .map((path) => readFileSync(new URL(path, import.meta.url), "utf8"))
  .join("\n");

describe("production UI truthfulness", () => {
  it("does not ship the original showcase fixtures or synthetic metrics", () => {
    const forbidden = [
      "Harbor Ledger",
      "Cedar Commerce",
      "Atlas API",
      "Citrine",
      "Northstar",
      "Kite",
      "Tern",
      "LedgerExportsController",
      "68,840",
      "09:41 CDT",
      "12 delegated threads",
      "2 diffs awaiting review",
      "4 agents weaving",
      "preview fixture",
    ];

    for (const value of forbidden) {
      expect(productionSource, `Remove synthetic UI value: ${value}`).not.toContain(value);
    }
  });
});
