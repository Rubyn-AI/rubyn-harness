import { describe, expect, it, vi } from "vitest";
import { createExclusiveTask } from "./exclusiveTask";

describe("exclusive async work", () => {
  it("skips overlapping polling ticks and resumes after completion", async () => {
    let releaseFirst!: () => void;
    let active = 0;
    let peakActive = 0;
    const firstWait = new Promise<void>((resolve) => { releaseFirst = resolve; });
    const task = vi.fn(async () => {
      active += 1;
      peakActive = Math.max(peakActive, active);
      if (task.mock.calls.length === 1) await firstWait;
      active -= 1;
    });
    const poll = createExclusiveTask(task);

    const first = poll();
    await expect(poll()).resolves.toBe(false);
    expect(task).toHaveBeenCalledTimes(1);

    releaseFirst();
    await expect(first).resolves.toBe(true);
    await expect(poll()).resolves.toBe(true);

    expect(task).toHaveBeenCalledTimes(2);
    expect(peakActive).toBe(1);
  });

  it("releases the guard after a failed tick", async () => {
    const task = vi.fn()
      .mockRejectedValueOnce(new Error("temporary failure"))
      .mockResolvedValueOnce(undefined);
    const poll = createExclusiveTask(task);

    await expect(poll()).rejects.toThrow("temporary failure");
    await expect(poll()).resolves.toBe(true);
  });
});
