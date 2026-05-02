import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import type { HttpMessage } from "../types";
import { useStore, type StoreState } from "../store";

export function useTraffic() {
  const processMessage = useStore((s: StoreState) => s.processMessage);
  const processMessageRef = useRef(processMessage);
  processMessageRef.current = processMessage;

  useEffect(() => {
    const unlistenPromise = listen<HttpMessage>("traffic:request", (event) => {
      processMessageRef.current(event.payload);
    });

    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, []);
}
