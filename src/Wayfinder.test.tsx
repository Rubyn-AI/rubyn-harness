// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WayfinderMapData } from "./bridge";

const native = vi.hoisted(() => ({
  listMaps: vi.fn(),
  listBlockers: vi.fn(),
  createMap: vi.fn(),
  getMap: vi.fn(),
  submitAnswers: vi.fn(),
  readSkill: vi.fn(),
  launchPrompt: vi.fn(),
  linkRun: vi.fn(),
  listRuns: vi.fn(),
  projectData: vi.fn(),
}));

vi.mock("./bridge", async () => {
  const actual = await vi.importActual<typeof import("./bridge")>("./bridge");
  return { ...actual, harnessBridge: { ...actual.harnessBridge, listWayfinderMaps: native.listMaps, listWayfinderBlockers: native.listBlockers, createWayfinderMap: native.createMap, getWayfinderMap: native.getMap, submitWayfinderAnswers: native.submitAnswers, readSkill: native.readSkill, launchPrompt: native.launchPrompt, linkWayfinderRun: native.linkRun, listRuns: native.listRuns, projectData: native.projectData } };
});

import { Wayfinder } from "./Wayfinder";
import { composeWayfinderChartPrompt, composeWayfinderLaunchPrompt } from "./wayfinderPrompts";
import { useHarnessStore } from "./store";

const data: WayfinderMapData = {
  map: { id: 4, projectId: 1, title: "Tenant isolation", idea: "Map tenant isolation", destination: "", notes: "", codeTaskStatus: "planning", status: "draft", createdAt: 1, updatedAt: 1 },
  tickets: [{ id: 8, mapId: 4, title: "Name the destination", question: "What does success look like?", information: "Map tenant isolation", outcome: "A destination is approved", ticketType: "grill", status: "frontier", dependsOn: [], briefVersion: 1, resolution: "", resultNote: "", modelRole: "sol", effort: "high", createdAt: 1, updatedAt: 1 }],
  questions: [{ id: 11, ticketId: 8, round: 1, title: "Destination", prompt: "What must be observably true?", cardinality: "single", options: [{ id: "specific", label: "Specific outcome", description: "Name measurable evidence", pros: "Clear completion", cons: "Needs precision", recommended: true }], answers: [], customAnswer: "", createdAt: 1 }],
  events: [{ id: 1, mapId: 4, ticketId: 8, kind: "map/created", actor: "user", payload: {}, createdAt: 1 }],
};

describe("Wayfinder workspace", () => {
  beforeEach(() => {
    native.listMaps.mockReset().mockResolvedValue([]);
    native.listBlockers.mockReset().mockResolvedValue([]);
    native.createMap.mockReset().mockResolvedValue(data);
    native.getMap.mockReset().mockResolvedValue(data);
    native.submitAnswers.mockReset().mockResolvedValue({ ...data, questions: data.questions.map((question) => ({ ...question, answers: ["specific"], answeredAt: 2 })) });
    native.readSkill.mockReset().mockImplementation(async (path: string) => ({ path, content: `---\nname: source\n---\n\nSOURCE ${path}` }));
    native.launchPrompt.mockReset().mockResolvedValue({ id: 31 });
    native.linkRun.mockReset().mockResolvedValue(data.tickets[0]);
    native.listRuns.mockReset().mockResolvedValue([]);
    native.projectData.mockReset().mockResolvedValue({ project: { id: 1 }, agents: [], columns: [{ id: 2, projectId: 1, key: "planning", name: "Planning", position: 1, terminal: false }], tasks: [], todos: [], runs: [], approvals: [] });
    useHarnessStore.setState({ project: { path: "/repo", name: "repo", isRuby: true, isRails: true, hasRubynInstructions: false }, modelCatalog: { models: [{ provider: "anthropic", model: "claude", tier: "sol" }], activeProvider: "anthropic", activeModel: "claude", modelMode: "manual", connectedProviders: ["anthropic"] }, globalRuns: [], wayfinderMaps: [], wayfinderBlockers: [], activeWayfinderMapId: undefined, wayfinderData: undefined, view: "wayfinder" });
  });
  afterEach(() => cleanup());

  it("creates a map from a loose idea and opens the durable bootstrap Grill", async () => {
    render(<Wayfinder />);
    fireEvent.click(await screen.findByRole("button", { name: "New map" }));
    const dialog = screen.getByRole("dialog", { name: "Start a Wayfinder map" });
    fireEvent.change(within(dialog).getByRole("textbox"), { target: { value: "Map tenant isolation" } });
    fireEvent.change(await within(dialog).findByRole("combobox", { name: "Code task column" }), { target: { value: "planning" } });
    fireEvent.click(within(dialog).getByRole("button", { name: /Start Grill/ }));
    await waitFor(() => expect(native.launchPrompt).toHaveBeenCalledWith(
      "/repo",
      expect.stringContaining('action "create_node"'),
      [],
      { provider: "anthropic", model: "claude", tier: "sol" },
    ));
    expect(native.linkRun).toHaveBeenCalledWith(8, 31);
    expect(native.createMap).toHaveBeenCalledWith("/repo", "Map tenant isolation", "planning");
  });

  it("submits a typed Grill answer through the native contract", async () => {
    useHarnessStore.setState({ activeWayfinderMapId: 4, wayfinderData: data });
    render(<Wayfinder />);
    const option = await screen.findByRole("radio", { name: /Specific outcome/ });
    fireEvent.click(option);
    fireEvent.click(screen.getByRole("button", { name: /Submit round/ }));
    await waitFor(() => expect(native.submitAnswers).toHaveBeenCalledWith(8, [{ questionId: 11, answers: ["specific"], customAnswer: "" }]));
  });

  it("builds grill launches from the Matt Pocock Wayfinder and Grilling skills", () => {
    const prompt = composeWayfinderLaunchPrompt(
      data.tickets[0],
      data,
      "---\nname: wayfinder\n---\n\nWAYFINDER SOURCE BODY",
      "---\nname: grilling\n---\n\nGRILLING SOURCE BODY",
    );

    expect(prompt).toContain('<skill name="wayfinder" source="mattpocock/skills">\nWAYFINDER SOURCE BODY');
    expect(prompt).toContain('<skill name="grilling" source="mattpocock/skills">\nGRILLING SOURCE BODY');
    expect(prompt).toContain("Rubyn Harness is the tracker adapter described by the Wayfinder skill");
    expect(prompt).toContain("Destination: Map tenant isolation");
    expect(prompt).toContain("Title: Name the destination");
    expect(prompt).not.toContain("name: wayfinder");
  });

  it("adapts Wayfinder charting into native map nodes and deferred Tasks", () => {
    const prompt = composeWayfinderChartPrompt(data, "WAYFINDER", "GRILLING");

    expect(prompt).toContain('wayfinder with action "update_map"');
    expect(prompt).toContain('action "create_node"');
    expect(prompt).toContain("blocked_by may contain exact titles");
    expect(prompt).toContain("Code nodes become real Harness Tasks after the human activates the map");
    expect(prompt).toContain('Code task column: planning');
    expect(prompt).toContain("Map ID: 4");
  });

  it("loads both authoritative skills before launching a grill ticket", async () => {
    const activeData = { ...data, map: { ...data.map, status: "active" as const } };
    useHarnessStore.setState({
      activeWayfinderMapId: 4,
      wayfinderData: activeData,
      modelCatalog: { models: [{ provider: "anthropic", model: "claude", tier: "sol" }], activeProvider: "anthropic", activeModel: "claude", modelMode: "manual", connectedProviders: ["anthropic"] },
      globalRuns: [],
    });
    render(<Wayfinder />);

    fireEvent.click(await screen.findByRole("button", { name: "Preview launch" }));
    expect(screen.getByText("Wayfinder + Grilling")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Launch isolated run/ }));

    await waitFor(() => expect(native.readSkill).toHaveBeenCalledTimes(2));
    expect(native.readSkill).toHaveBeenNthCalledWith(1, "wayfinder/wayfinder.md");
    expect(native.readSkill).toHaveBeenNthCalledWith(2, "wayfinder/grilling.md");
    await waitFor(() => expect(native.launchPrompt).toHaveBeenCalledWith(
      "/repo",
      expect.stringContaining("SOURCE wayfinder/wayfinder.md"),
      [],
      { provider: "anthropic", model: "claude", tier: "sol" },
    ));
  });
});
