import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import type { HttpMessage } from "../types";
import { useStore } from "../store";

export function useTraffic() {
  const processMessage = useStore((s) => s.processMessage);

  useEffect(() => {
    const unlistenRequest = listen<HttpMessage>("traffic:request", (event) => {
      processMessage(event.payload);
    });

    return () => {
      unlistenRequest.then((fn) => fn());
    };
  }, [processMessage]);
}
