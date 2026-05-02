import { create } from "zustand";
import type { HttpMessage, HttpSession } from "../types";

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

export interface StoreState {
  sessions: Map<number, HttpSession>;
  sessionList: number[];
  selectedId: number | null;
  captureStatus: string;
  filter: TrafficFilter;
  filteredSessionList: number[];
  bookmarks: Set<number>;

  totalSessions: number;
  activeSessions: number;
  bytesCaptured: number;
  decryptedCount: number;

  processMessage: (msg: HttpMessage) => void;
  selectRequest: (id: number | null) => void;
  clearRequests: () => void;
  setCaptureStatus: (status: string) => void;
  setFilter: (filter: Partial<TrafficFilter>) => void;
  resetFilter: () => void;
  toggleBookmark: (id: number) => void;
  isBookmarked: (id: number) => boolean;
}

function matchesFilter(session: HttpSession, filter: TrafficFilter): boolean {
  const req = session.request;
  const resp = session.response;

  if (filter.method !== "ALL") {
    if (req.method?.toUpperCase() !== filter.method) return false;
  }

  if (filter.scheme !== "ALL") {
    if (req.scheme !== filter.scheme) return false;
  }

  if (filter.status !== "ALL" && resp?.status_code != null) {
    const code = resp.status_code;
    const group = filter.status;
    if (group === "2xx" && (code < 200 || code >= 300)) return false;
    if (group === "3xx" && (code < 300 || code >= 400)) return false;
    if (group === "4xx" && (code < 400 || code >= 500)) return false;
    if (group === "5xx" && (code < 500 || code >= 600)) return false;
  }

  if (filter.searchText.trim()) {
    const search = filter.searchText.toLowerCase().trim();
    const urlMatch = req.url?.toLowerCase().includes(search) ?? false;
    const hostMatch = req.headers.some(
      ([k, v]) => k.toLowerCase() === "host" && v.toLowerCase().includes(search)
    );
    const methodMatch = req.method?.toLowerCase().includes(search) ?? false;
    const processMatch = req.process_name?.toLowerCase().includes(search) ?? false;
    if (!urlMatch && !hostMatch && !methodMatch && !processMatch) return false;
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

function computeStats(sessions: Map<number, HttpSession>) {
  let total = 0;
  let active = 0;
  let bytes = 0;
  let decrypted = 0;

  for (const s of sessions.values()) {
    total++;
    if (!s.response) active++;
    bytes += (s.request.body_size || 0) + (s.response?.body_size || 0);
    if (s.request.raw_tls_info != null) decrypted++;
  }

  return { totalSessions: total, activeSessions: active, bytesCaptured: bytes, decryptedCount: decrypted };
}

export const useStore = create<StoreState>((set) => ({
  sessions: new Map(),
  sessionList: [],
  selectedId: null,
  captureStatus: "Idle",
  filter: { ...DEFAULT_FILTER },
  filteredSessionList: [],
  bookmarks: new Set<number>(),
  totalSessions: 0,
  activeSessions: 0,
  bytesCaptured: 0,
  decryptedCount: 0,

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
        const stats = computeStats(newSessions);
        return {
          sessions: newSessions,
          sessionList: newSessionList,
          filteredSessionList: newFiltered,
          ...stats,
        };
      } else {
        const existing = newSessions.get(msg.session_id);
        if (existing) {
          const updated = { ...existing, response: msg };
          newSessions.set(msg.session_id, updated);
        }
        const newFiltered = computeFiltered(state.sessionList, newSessions, state.filter);
        const stats = computeStats(newSessions);
        return {
          sessions: newSessions,
          filteredSessionList: newFiltered,
          ...stats,
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
      totalSessions: 0,
      activeSessions: 0,
      bytesCaptured: 0,
      decryptedCount: 0,
    }),
  setCaptureStatus: (status) => set({ captureStatus: status }),

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

  toggleBookmark: (id) =>
    set((state) => {
      const newBookmarks = new Set(state.bookmarks);
      if (newBookmarks.has(id)) { newBookmarks.delete(id); } else { newBookmarks.add(id); }
      return { bookmarks: newBookmarks };
    }),

  isBookmarked: (id: number): boolean => {
    return useStore.getState().bookmarks.has(id);
  },
}));
