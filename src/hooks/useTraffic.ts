import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import type { HttpMessage, EngineStats } from "../types";
import { useStore } from "../store";

export function useTraffic() {
  const addRequest = useStore((s) => s.addRequest);
  const setStats = useStore((s) => s.setStats);

  useEffect(() => {
    const unlistenRequest = listen<HttpMessage>("traffic:request", (event) => {
      addRequest(event.payload);
    });

    const unlistenStats = listen<EngineStats>("traffic:stats", (event) => {
      setStats(event.payload);
    });

    return () => {
      unlistenRequest.then((fn) => fn());
      unlistenStats.then((fn) => fn());
    };
  }, [addRequest, setStats]);
}
