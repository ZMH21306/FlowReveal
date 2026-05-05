import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import type { HttpMessage } from "../types";
import { useStore, type StoreState } from "../store";
import { getDiverterStatus, checkElevated, checkWifiAdapter } from "../lib/tauri-bindings";

export function useTraffic() {
  const processMessage = useStore((s: StoreState) => s.processMessage);
  const processMessageRef = useRef(processMessage);
  processMessageRef.current = processMessage;

  useEffect(() => {
    const unlistenPromise = listen<HttpMessage>("traffic:request", (event) => {
      processMessageRef.current(event.payload);
    });

    (async () => {
      try {
        const [status, elevated, wifi] = await Promise.all([
          getDiverterStatus(),
          checkElevated(),
          checkWifiAdapter(),
        ]);
        useStore.setState({
          diverterStatus: status,
          isElevated: elevated,
          isWifi: wifi,
        });
      } catch {
        useStore.setState({ diverterStatus: "NotAvailable", isElevated: false, isWifi: false });
      }
    })();

    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, []);
}
