import { create } from "zustand";
import type { HttpMessage, CaptureStatus, EngineStats } from "../types";

interface TrafficState {
  requests: HttpMessage[];
  selectedId: number | null;
  captureStatus: CaptureStatus;
  stats: EngineStats;

  addRequest: (msg: HttpMessage) => void;
  clearRequests: () => void;
  selectRequest: (id: number | null) => void;
  setCaptureStatus: (status: CaptureStatus) => void;
  setStats: (stats: EngineStats) => void;
}

export const useStore = create<TrafficState>((set) => ({
  requests: [],
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

  addRequest: (msg) =>
    set((state) => ({ requests: [...state.requests, msg] })),

  clearRequests: () => set({ requests: [], selectedId: null }),

  selectRequest: (id) => set({ selectedId: id }),

  setCaptureStatus: (status) => set({ captureStatus: status }),

  setStats: (stats) => set({ stats }),
}));
