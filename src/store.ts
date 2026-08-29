import { create } from "zustand";
import type {
  LocalAppState,
  ModelCatalog,
  ProjectData,
  ProjectSummary,
  RunEventRecord,
  SkillSummary,
  AttachmentSelection,
  WayfinderMap,
  WayfinderMapData,
  WayfinderTicket,
} from "./bridge";

export type View =
  | "control"
  | "workboard"
  | "agents"
  | "team"
  | "skills"
  | "review"
  | "wayfinder"
  | "accounts"
  | "projects";

type EngineState = "checking" | "ready" | "unavailable";

interface HarnessStore {
  view: View;
  project?: ProjectSummary;
  projectData?: ProjectData;
  globalRuns: ProjectData["runs"];
  appState?: LocalAppState;
  modelCatalog?: ModelCatalog;
  skills: SkillSummary[];
  wayfinderMaps: WayfinderMap[];
  wayfinderBlockers: WayfinderTicket[];
  activeWayfinderMapId?: number;
  wayfinderData?: WayfinderMapData;
  selectedRunId?: number;
  activeConversationId?: number;
  newConversationDraft: string;
  newConversationTaskId?: number;
  conversationDrafts: Record<number, string>;
  newConversationAttachments: AttachmentSelection[];
  conversationAttachments: Record<number, AttachmentSelection[]>;
  runEvents: Record<number, RunEventRecord[]>;
  eventCursors: Record<number, number>;
  commandOpen: boolean;
  mobileOpen: boolean;
  reducedMotion: boolean;
  engineState: EngineState;
  engineDetail: string;
  loading: boolean;
  notice: string;
  setView: (view: View) => void;
  setProject: (project?: ProjectSummary) => void;
  setProjectData: (data?: ProjectData) => void;
  setGlobalRuns: (runs: ProjectData["runs"]) => void;
  setAppState: (state: LocalAppState) => void;
  setModelCatalog: (catalog: ModelCatalog) => void;
  setSkills: (skills: SkillSummary[]) => void;
  setWayfinderMaps: (maps: WayfinderMap[]) => void;
  setWayfinderBlockers: (blockers: WayfinderTicket[]) => void;
  openWayfinderMap: (mapId?: number) => void;
  setWayfinderData: (data?: WayfinderMapData) => void;
  selectRun: (runId?: number) => void;
  openConversation: (runId: number) => void;
  setNewConversationDraft: (draft: string) => void;
  setNewConversationTaskId: (taskId?: number) => void;
  setConversationDraft: (runId: number, draft: string) => void;
  setNewConversationAttachments: (attachments: AttachmentSelection[]) => void;
  setConversationAttachments: (runId: number, attachments: AttachmentSelection[]) => void;
  appendRunEvents: (runId: number, events: RunEventRecord[], cursor: number) => void;
  setCommandOpen: (open: boolean) => void;
  setMobileOpen: (open: boolean) => void;
  setReducedMotion: (enabled: boolean) => void;
  setEngine: (state: EngineState, detail: string) => void;
  setLoading: (loading: boolean) => void;
  setNotice: (notice: string) => void;
}

export const useHarnessStore = create<HarnessStore>((set) => ({
  view: "agents",
  skills: [],
  wayfinderMaps: [],
  wayfinderBlockers: [],
  runEvents: {},
  eventCursors: {},
  globalRuns: [],
  newConversationDraft: "",
  newConversationTaskId: undefined,
  conversationDrafts: {},
  newConversationAttachments: [],
  conversationAttachments: {},
  commandOpen: false,
  mobileOpen: false,
  reducedMotion: false,
  engineState: "checking",
  engineDetail: "Checking bundled Rubyn Code…",
  loading: true,
  notice: "",
  setView: (view) => set({ view, commandOpen: false, mobileOpen: false }),
  setProject: (project) => set({ project, projectData: undefined, wayfinderMaps: [], wayfinderBlockers: [], activeWayfinderMapId: undefined, wayfinderData: undefined, selectedRunId: undefined, activeConversationId: undefined, newConversationDraft: "", newConversationTaskId: undefined, conversationDrafts: {}, newConversationAttachments: [], conversationAttachments: {}, runEvents: {}, eventCursors: {} }),
  setProjectData: (projectData) => set({ projectData }),
  setGlobalRuns: (globalRuns) => set({ globalRuns }),
  setAppState: (appState) => set({ appState }),
  setModelCatalog: (modelCatalog) => set({ modelCatalog }),
  setSkills: (skills) => set({ skills }),
  setWayfinderMaps: (wayfinderMaps) => set({ wayfinderMaps }),
  setWayfinderBlockers: (wayfinderBlockers) => set({ wayfinderBlockers }),
  openWayfinderMap: (activeWayfinderMapId) => set({ activeWayfinderMapId, view: "wayfinder", commandOpen: false, mobileOpen: false }),
  setWayfinderData: (wayfinderData) => set({ wayfinderData }),
  selectRun: (selectedRunId) => set({ selectedRunId, view: "review" }),
  openConversation: (activeConversationId) => set({ activeConversationId, view: "agents", commandOpen: false, mobileOpen: false }),
  setNewConversationDraft: (newConversationDraft) => set({ newConversationDraft }),
  setNewConversationTaskId: (newConversationTaskId) => set({ newConversationTaskId }),
  setConversationDraft: (runId, draft) => set((state) => ({ conversationDrafts: { ...state.conversationDrafts, [runId]: draft } })),
  setNewConversationAttachments: (newConversationAttachments) => set({ newConversationAttachments }),
  setConversationAttachments: (runId, attachments) => set((state) => ({ conversationAttachments: { ...state.conversationAttachments, [runId]: attachments } })),
  appendRunEvents: (runId, events, cursor) => set((state) => {
    const unique = new Map(
      [...(state.runEvents[runId] || []), ...events].map((event) => [event.id, event]),
    );
    return {
      runEvents: { ...state.runEvents, [runId]: [...unique.values()].slice(-500) },
      eventCursors: { ...state.eventCursors, [runId]: cursor },
    };
  }),
  setCommandOpen: (commandOpen) => set({ commandOpen }),
  setMobileOpen: (mobileOpen) => set({ mobileOpen }),
  setReducedMotion: (reducedMotion) => set({ reducedMotion }),
  setEngine: (engineState, engineDetail) => set({ engineState, engineDetail }),
  setLoading: (loading) => set({ loading }),
  setNotice: (notice) => set({ notice }),
}));
