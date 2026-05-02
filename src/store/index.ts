import { create } from "zustand";
import type { HttpMessage, HttpSession, EngineStats } from "../types";

export type FilterMethod = "ALL" | "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" | "CONNECT";
export type FilterScheme = "ALL" | "Http" | "Https";
export type FilterStatus = "ALL" | "2xx" | "3xx" | "4xx" | "5xx";

export interface TrafficFilter {
  searchText: string;
  method: FilterMethod;
  scheme: FilterScheme;
  status: FilterStatus;
}

const DEFAULT_FILTER: TrafficFilter = {
  searchText: "",
  method: "ALL",
  scheme: "ALL",
  status: "ALL",
};

interface StoreState {
  sessions: Map<number, HttpSession>;
  sessionList: number[];
  selectedId: number | null;
  captureStatus: string;
  stats: EngineStats;
  filter: TrafficFilter;
  filteredSessionList: number[];

  processMessage: (msg: HttpMessage) => void;
  selectRequest: (id: number | null) => void;
  clearRequests: () => void;
  setCaptureStatus: (status: string) => void;
  setStats: (stats: EngineStats) => void;
  setFilter: (filter: Partial<TrafficFilter>) => void;
  resetFilter: () => void;
}

function matchesFilter(session: HttpSession, filter: TrafficFilter): boolean {
  const req = session.request;
  const resp = session.response;

  if (filter.method !== "ALL" && req.method?.toUpperCase() !== filter.method) {
    return false;
  }

  if (filter.scheme !== "ALL" && req.scheme !== filter.scheme) {
    return false;
  }

  if (filter.status !== "ALL" && resp?.status_code != null) {
    const code = resp.status_code;
    const group = filter.status;
    if (group === "2xx" && (code < 200 || code >= 300)) return false;
    if (group === "3xx" && (code < 300 || code >= 400)) return false;
    if (group === "4xx" && (code < 400 || code >= 500)) return false;
    if (group === "5xx" && (code < 500 || code >= 600)) return false;
  }

  if (filter.searchText) {
    const search = filter.searchText.toLowerCase();
    const urlMatch = req.url?.toLowerCase().includes(search) ?? false;
    const hostMatch = req.headers.some(
      ([k, v]) => k.toLowerCase() === "host" && v.toLowerCase().includes(search)
    );
    const methodMatch = req.method?.toLowerCase().includes(search) ?? false;
    const processMatch = req.process_name?.toLowerCase().includes(search) ?? false;
    if (!urlMatch && !hostMatch && !methodMatch && !processMatch) {
      return false;
    }
  }

  return true;
}

function computeFiltered(
  sessionList: number[],
  sessions: Map<number, HttpSession>,
  filter: TrafficFilter
): number[] {
  if (
    filter.searchText === "" &&
    filter.method === "ALL" &&
    filter.scheme === "ALL" &&
    filter.status === "ALL"
  ) {
    return sessionList;
  }
  return sessionList.filter((id) => {
    const session = sessions.get(id);
    return session ? matchesFilter(session, filter) : false;
  });
}

export const useStore = create<StoreState>((set) => ({
  sessions: new Map(),
  sessionList: [],
  selectedId: null,
  captureStatus: "Idle",
  stats: {
    total_sessions: 0,
    active_sessions: 0,
    bytes_captured: 0,
    tls_handshakes: 0,
    hook_injections: 0,
    http1_requests: 0,
    http2_requests: 0,
    ws_frames: 0,
    filtered_out: 0,
  },
  filter: { ...DEFAULT_FILTER },
  filteredSessionList: [],

  processMessage: (msg: HttpMessage) =>
    set((state) => {
      const newSessions = new Map(state.sessions);
      const isRequest = msg.direction === "Request";

      if (isRequest) {
        const session: HttpSession = {
          id: msg.session_id,
          request: msg,
          response: null,
          created_at: msg.timestamp,
          completed_at: null,
        };
        newSessions.set(msg.session_id, session);
        const newSessionList = [...state.sessionList, msg.session_id];
        const newFiltered = computeFiltered(newSessionList, newSessions, state.filter);
        return {
          sessions: newSessions,
          sessionList: newSessionList,
          filteredSessionList: newFiltered,
        };
      } else {
        const existing = newSessions.get(msg.session_id);
        if (existing) {
          const updated = { ...existing, response: msg };
          newSessions.set(msg.session_id, updated);
        }
        const newFiltered = computeFiltered(state.sessionList, newSessions, state.filter);
        return {
          sessions: newSessions,
          filteredSessionList: newFiltered,
        };
      }
    }),

  selectRequest: (id) => set({ selectedId: id }),
  clearRequests: () =>
    set({
      sessions: new Map(),
      sessionList: [],
      selectedId: null,
      filteredSessionList: [],
    }),
  setCaptureStatus: (status) => set({ captureStatus: status }),
  setStats: (stats) => set({ stats }),

  setFilter: (partial) =>
    set((state) => {
      const newFilter = { ...state.filter, ...partial };
      const newFiltered = computeFiltered(state.sessionList, state.sessions, newFilter);
      return { filter: newFilter, filteredSessionList: newFiltered };
    }),

  resetFilter: () =>
    set((state) => {
      const newFiltered = computeFiltered(state.sessionList, state.sessions, DEFAULT_FILTER);
      return { filter: { ...DEFAULT_FILTER }, filteredSessionList: newFiltered };
    }),
}));
