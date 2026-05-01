import { useStore } from "../../store";
import { startCapture, stopCapture } from "../../lib/tauri-bindings";
import type { CaptureConfig } from "../../types";

export function Toolbar() {
  const captureStatus = useStore((s) => s.captureStatus);
  const clearRequests = useStore((s) => s.clearRequests);
  const setCaptureStatus = useStore((s) => s.setCaptureStatus);

  const isRunning = captureStatus === "Running";

  const handleToggleCapture = async () => {
    try {
      if (isRunning) {
        await stopCapture();
        setCaptureStatus("Idle");
      } else {
        const config: CaptureConfig = {
          mode: "ForwardProxy",
          capture_http: true,
          capture_https: false,
          ports: [],
          process_filters: [],
          host_filters: [],
          max_body_size: 5 * 1024 * 1024,
          ca_cert_path: null,
          ca_key_path: null,
          mitm_bypass_hosts: [],
          proxy_port: 40960,
        };
        await startCapture(config);
        setCaptureStatus("Running");
      }
    } catch (e) {
      console.error("Capture toggle failed:", e);
      setCaptureStatus("Error");
    }
  };

  return (
    <div className="flex items-center gap-3 px-4 py-2 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
      <button
        onClick={handleToggleCapture}
        className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
          isRunning
            ? "bg-[var(--color-error)] hover:bg-red-600 text-white"
            : "bg-[var(--color-success)] hover:bg-green-600 text-white"
        }`}
      >
        {isRunning ? "■ Stop" : "▶ Start"}
      </button>

      <button
        onClick={clearRequests}
        className="px-3 py-1.5 rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors"
      >
        Clear
      </button>

      <div className="flex-1" />

      <div className="text-xs text-[var(--color-text-secondary)]">
        FlowReveal
      </div>
    </div>
  );
}
