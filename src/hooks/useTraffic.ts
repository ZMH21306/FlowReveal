import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import type { HttpMessage, EngineStats } from "../types";
import { useStore } from "../store";

export function useTraffic() {
  const processMessage = useStore((s) => s.processMessage);
  const setStats = useStore((s) => s.setStats);

  useEffect(() => {
    const unlistenRequest = listen<HttpMessage>("traffic:request", (event) => {
      processMessage(event.payload);
    });

    const unlistenStats = listen<EngineStats>("traffic:stats", (event) => {
      setStats(event.payload);
    });

    return () => {
      unlistenRequest.then((fn) => fn());
      unlistenStats.then((fn) => fn());
    };
  }, [processMessage, setStats]);
}
