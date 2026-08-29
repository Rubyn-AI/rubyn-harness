// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { useHarnessStore } from "./store";

describe("harness UI cache", () => {
  beforeEach(() => useHarnessStore.setState({
    view: "control",
    project: undefined,
    projectData: undefined,
    runEvents: {},
    eventCursors: {},
    selectedRunId: undefined,
  }));

  it("changes the active native product surface", () => {
    useHarnessStore.getState().setView("agents");
    expect(useHarnessStore.getState().view).toBe("agents");
  });

  it("deduplicates persisted run events while moving the cursor", () => {
    const event = { id: 7, runId: 2, protocolSequence: 4, kind: "message", payload: {}, raw: "ok", createdAt: 1 };
    useHarnessStore.getState().appendRunEvents(2, [event, event], 7);
    expect(useHarnessStore.getState().runEvents[2]).toEqual([event]);
    expect(useHarnessStore.getState().eventCursors[2]).toBe(7);
  });

  it("clears run-local review state when the native project boundary changes", () => {
    const first = { path: "/work/first", name: "first", isRuby: true, isRails: false, hasRubynInstructions: false };
    const second = { path: "/work/second", name: "second", isRuby: true, isRails: true, hasRubynInstructions: true };
    useHarnessStore.getState().setProject(first);
    useHarnessStore.getState().appendRunEvents(4, [
      { id: 9, runId: 4, protocolSequence: 1, kind: "message", payload: {}, raw: "working", createdAt: 1 },
    ], 9);
    useHarnessStore.getState().selectRun(4);

    useHarnessStore.getState().setProject(second);

    expect(useHarnessStore.getState()).toMatchObject({
      project: second,
      selectedRunId: undefined,
      runEvents: {},
      eventCursors: {},
    });
  });

  it("selects a persisted run and routes directly to review", () => {
    useHarnessStore.getState().selectRun(12);
    expect(useHarnessStore.getState()).toMatchObject({ selectedRunId: 12, view: "review" });
  });
});
