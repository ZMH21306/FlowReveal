import { useState, useRef, useEffect } from "react";
import { useStore } from "../../store";
import { startCapture, stopCapture } from "../../lib/tauri-bindings";
import type { CaptureConfig, CaptureMode } from "../../types";

const CAPTURE_MODES: { value: CaptureMode; label: string; desc: string }[] = [
  { value: "ForwardProxy", label: "Forward Proxy", desc: "Manual proxy configuration" },
  { value: "TransparentProxy", label: "Transparent", desc: "WFP transparent proxy (requires admin)" },
];

function ModeDropdown({
  value,
  onChange,
  disabled,
}: {
  value: CaptureMode;
  onChange: (v: CaptureMode) => void;
  disabled: boolean;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const current = CAPTURE_MODES.find((m) => m.value === value)!;

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-3 py-1.5 rounded border border-[var(--color-border)] disabled:opacity-50 cursor-pointer hover:border-[var(--color-accent)] transition-colors focus:outline-none focus:border-[var(--color-accent)]"
      >
        {current.label}
        <svg className="w-3 h-3 text-[var(--color-text-secondary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {open && !disabled && (
        <div className="absolute top-full left-0 mt-1 z-50 min-w-[200px] bg-[var(--color-bg-tertiary)] border border-[var(--color-border)] rounded shadow-lg overflow-hidden">
          {CAPTURE_MODES.map((mode) => (
            <button
              key={mode.value}
              type="button"
              onClick={() => {
                onChange(mode.value);
                setOpen(false);
              }}
              className={`w-full text-left px-3 py-2 text-xs transition-colors ${
                value === mode.value
                  ? "bg-[var(--color-accent)] text-white"
                  : "text-[var(--color-text-primary)] hover:bg-[var(--color-bg-secondary)]"
              }`}
            >
              <div className="font-medium">{mode.label}</div>
              <div className="text-[10px] text-[var(--color-text-secondary)] mt-0.5">{mode.desc}</div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function Toolbar() {
  const captureStatus = useStore((s) => s.captureStatus);
  const clearRequests = useStore((s) => s.clearRequests);
  const setCaptureStatus = useStore((s) => s.setCaptureStatus);
  const [captureMode, setCaptureMode] = useState<CaptureMode>("ForwardProxy");
  const [pending, setPending] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const isRunning = captureStatus === "Running";

  const handleToggleCapture = async () => {
    if (pending) return;
    setPending(true);
    setErrorMsg(null);
    try {
      if (isRunning) {
        await stopCapture();
        setCaptureStatus("Idle");
      } else {
        const config: CaptureConfig = {
          mode: captureMode,
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
      setErrorMsg(String(e));
      setTimeout(() => setErrorMsg(null), 5000);
    } finally {
      setPending(false);
    }
  };

  const currentMode = CAPTURE_MODES.find((m) => m.value === captureMode)!;

  return (
    <div className="flex items-center gap-3 px-4 py-2 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
      <button
        onClick={handleToggleCapture}
        disabled={pending}
        className={`px-4 py-1.5 rounded text-sm font-medium transition-colors disabled:opacity-50 ${
          isRunning
            ? "bg-[var(--color-error)] hover:bg-red-600 text-white"
            : "bg-[var(--color-success)] hover:bg-green-600 text-white"
        }`}
      >
        {pending ? "..." : isRunning ? "■ Stop" : "▶ Start"}
      </button>

      <ModeDropdown
        value={captureMode}
        onChange={setCaptureMode}
        disabled={isRunning || pending}
      />

      <button
        onClick={clearRequests}
        className="px-3 py-1.5 rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors"
      >
        Clear
      </button>

      {errorMsg && (
        <div className="text-xs text-[var(--color-error)] truncate max-w-[300px]" title={errorMsg}>
          {errorMsg}
        </div>
      )}

      <div className="flex-1" />

      {isRunning && (
        <div className="text-xs text-[var(--color-accent)] font-mono">
          {currentMode.label} · :40960
        </div>
      )}

      <div className="text-xs text-[var(--color-text-secondary)]">
        FlowReveal
      </div>
    </div>
  );
}
