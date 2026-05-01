import { create } from "zustand";
import type { HttpMessage, HttpSession, CaptureStatus, EngineStats } from "../types";

interface TrafficState {
  sessions: Map<number, HttpSession>;
  sessionList: number[];
  selectedId: number | null;
  captureStatus: CaptureStatus;
  stats: EngineStats;

  processMessage: (msg: HttpMessage) => void;
  clearRequests: () => void;
  selectRequest: (id: number | null) => void;
  setCaptureStatus: (status: CaptureStatus) => void;
  setStats: (stats: EngineStats) => void;
}

export const useStore = create<TrafficState>((set) => ({
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

  processMessage: (msg) =>
    set((state) => {
      const newSessions = new Map(state.sessions);
      const newSessionList = [...state.sessionList];

      if (msg.direction === "Request") {
        const session: HttpSession = {
          id: msg.session_id,
          request: msg,
          response: null,
          created_at: msg.timestamp,
          completed_at: null,
        };
        newSessions.set(msg.session_id, session);
        newSessionList.push(msg.session_id);
      } else {
        const existing = newSessions.get(msg.session_id);
        if (existing) {
          const updated: HttpSession = {
            ...existing,
            response: msg,
            completed_at: msg.timestamp,
          };
          newSessions.set(msg.session_id, updated);
        }
      }

      return { sessions: newSessions, sessionList: newSessionList };
    }),

  clearRequests: () => set({ sessions: new Map(), sessionList: [], selectedId: null }),

  selectRequest: (id) => set({ selectedId: id }),

  setCaptureStatus: (status) => set({ captureStatus: status }),

  setStats: (stats) => set({ stats }),
}));
