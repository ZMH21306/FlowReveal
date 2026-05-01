import { useState, useRef, useEffect, useCallback } from "react";
import { useStore } from "../../store";
import { startCapture, stopCapture, listRunningProcesses, injectHook, ejectHook, getInjectedPids } from "../../lib/tauri-bindings";
import type { CaptureConfig, CaptureMode, ProcessEntry } from "../../types";

const CAPTURE_MODES: { value: CaptureMode; label: string; desc: string }[] = [
  { value: "ForwardProxy", label: "Forward Proxy", desc: "Auto proxy + MITM" },
  { value: "TransparentProxy", label: "Transparent", desc: "WFP transparent proxy (requires admin)" },
  { value: "ApiHook", label: "API Hook", desc: "Inject hook DLL into target processes" },
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

function ProcessPicker({
  isOpen,
  onClose,
  onInject,
  onEject,
  injectedPids,
}: {
  isOpen: boolean;
  onClose: () => void;
  onInject: (pid: number) => void;
  onEject: (pid: number) => void;
  injectedPids: number[];
}) {
  const [processes, setProcesses] = useState<ProcessEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState("");
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  const loadProcesses = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await listRunningProcesses();
      setProcesses(list.map(p => ({
        ...p,
        is_injected: injectedPids.includes(p.pid),
      })));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [injectedPids]);

  useEffect(() => {
    if (isOpen) {
      loadProcesses();
    }
  }, [isOpen, loadProcesses]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    if (isOpen) {
      document.addEventListener("mousedown", handler);
      return () => document.removeEventListener("mousedown", handler);
    }
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const filtered = processes.filter(p =>
    p.name.toLowerCase().includes(search.toLowerCase()) ||
    p.pid.toString().includes(search)
  );

  return (
    <div ref={ref} className="absolute top-full left-0 mt-1 z-50 w-[360px] max-w-[calc(100vw-2rem)] bg-[var(--color-bg-tertiary)] border border-[var(--color-border)] rounded shadow-lg overflow-hidden">
      <div className="p-2 border-b border-[var(--color-border)]">
        <div className="flex items-center gap-2">
          <input
            type="text"
            placeholder="Search process..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="flex-1 bg-[var(--color-bg-secondary)] text-[var(--color-text-primary)] text-xs px-2 py-1.5 rounded border border-[var(--color-border)] focus:outline-none focus:border-[var(--color-accent)]"
            autoFocus
          />
          <button
            onClick={loadProcesses}
            disabled={loading}
            className="text-xs text-[var(--color-accent)] hover:underline disabled:opacity-50"
          >
            {loading ? "..." : "Refresh"}
          </button>
        </div>
        {error && <div className="text-xs text-[var(--color-error)] mt-1">{error}</div>}
      </div>
      <div className="max-h-[300px] overflow-y-auto">
        {filtered.length === 0 && (
          <div className="text-xs text-[var(--color-text-secondary)] p-3 text-center">
            {loading ? "Loading..." : "No processes found"}
          </div>
        )}
        {filtered.map((proc) => (
          <div
            key={proc.pid}
            className="flex items-center justify-between px-3 py-1.5 hover:bg-[var(--color-bg-secondary)] text-xs"
          >
            <div className="flex-1 min-w-0">
              <span className="text-[var(--color-text-primary)] font-mono">{proc.name}</span>
              <span className="text-[var(--color-text-secondary)] ml-2">PID: {proc.pid}</span>
            </div>
            {proc.is_injected ? (
              <button
                onClick={() => onEject(proc.pid)}
                className="ml-2 px-2 py-0.5 text-[10px] bg-[var(--color-error)] text-white rounded hover:opacity-80"
              >
                Eject
              </button>
            ) : (
              <button
                onClick={() => onInject(proc.pid)}
                className="ml-2 px-2 py-0.5 text-[10px] bg-[var(--color-accent)] text-white rounded hover:opacity-80"
              >
                Inject
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export function Toolbar() {
  const captureStatus = useStore((s) => s.captureStatus);
  const clearRequests = useStore((s) => s.clearRequests);
  const setCaptureStatus = useStore((s) => s.setCaptureStatus);
  const [captureMode, setCaptureMode] = useState<CaptureMode>("ForwardProxy");
  const [captureHttps, setCaptureHttps] = useState(true);
  const [pending, setPending] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [showProcessPicker, setShowProcessPicker] = useState(false);
  const [injectedPids, setInjectedPids] = useState<number[]>([]);

  const isRunning = captureStatus === "Running";
  const isHookMode = captureMode === "ApiHook";

  const refreshInjectedPids = useCallback(async () => {
    try {
      const pids = await getInjectedPids();
      setInjectedPids(pids);
    } catch {
      // ignore
    }
  }, []);

  const handleToggleCapture = async () => {
    if (pending) return;
    setPending(true);
    setErrorMsg(null);
    try {
      if (isRunning) {
        await stopCapture();
        setCaptureStatus("Idle");
        setInjectedPids([]);
      } else {
        const config: CaptureConfig = {
          mode: captureMode,
          capture_http: true,
          capture_https: captureHttps,
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
      setTimeout(() => setErrorMsg(null), 8000);
    } finally {
      setPending(false);
    }
  };

  const handleInject = async (pid: number) => {
    try {
      await injectHook(pid);
      await refreshInjectedPids();
    } catch (e) {
      setErrorMsg(String(e));
      setTimeout(() => setErrorMsg(null), 5000);
    }
  };

  const handleEject = async (pid: number) => {
    try {
      await ejectHook(pid);
      await refreshInjectedPids();
    } catch (e) {
      setErrorMsg(String(e));
      setTimeout(() => setErrorMsg(null), 5000);
    }
  };

  useEffect(() => {
    if (isRunning && isHookMode) {
      refreshInjectedPids();
    }
  }, [isRunning, isHookMode, refreshInjectedPids]);

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

      {!isHookMode && (
        <label className="flex items-center gap-1.5 cursor-pointer select-none">
          <input
            type="checkbox"
            checked={captureHttps}
            onChange={(e) => setCaptureHttps(e.target.checked)}
            disabled={isRunning || pending}
            className="w-3.5 h-3.5 rounded border-[var(--color-border)] accent-[var(--color-accent)]"
          />
          <span className="text-xs text-[var(--color-text-secondary)]">HTTPS</span>
        </label>
      )}

      {isRunning && isHookMode && (
        <div className="relative">
          <button
            onClick={() => setShowProcessPicker(!showProcessPicker)}
            className="flex items-center gap-1.5 bg-[var(--color-accent)] text-white text-xs px-3 py-1.5 rounded hover:opacity-90 transition-opacity"
          >
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            Inject Process
            {injectedPids.length > 0 && (
              <span className="bg-white/20 px-1.5 rounded text-[10px]">{injectedPids.length}</span>
            )}
          </button>
          <ProcessPicker
            isOpen={showProcessPicker}
            onClose={() => setShowProcessPicker(false)}
            onInject={handleInject}
            onEject={handleEject}
            injectedPids={injectedPids}
          />
        </div>
      )}

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
        <div className="text-xs text-[var(--color-accent)] font-mono flex items-center gap-1">
          <span>{currentMode.label}{isHookMode ? "" : ` · :40960`}</span>
          {!isHookMode && captureHttps && <span title="HTTPS decryption enabled (auto proxy + auto cert)">🔒HTTPS</span>}
          {isHookMode && injectedPids.length > 0 && <span>🎯{injectedPids.length} processes</span>}
        </div>
      )}

      <div className="text-xs text-[var(--color-text-secondary)]">
        FlowReveal
      </div>
    </div>
  );
}
