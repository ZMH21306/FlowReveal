import { useState, useRef, useEffect } from "react";
import { useStore, type StoreState } from "../../store";
import { startCapture, stopCapture, exportHar } from "../../lib/tauri-bindings";
import type { CaptureConfig, CaptureMode } from "../../types";
import { getStoredTheme, setStoredTheme, type ThemeMode } from "../../hooks/useTheme";

const CAPTURE_MODES: { value: CaptureMode; label: string; desc: string }[] = [
  { value: "DualProxy", label: "双代理模式", desc: "正向代理 + 透明代理，最大化流量捕获（推荐）" },
  { value: "ForwardProxy", label: "正向代理", desc: "自动代理 + MITM 解密" },
  { value: "TransparentProxy", label: "透明代理", desc: "WFP 透明代理（需管理员权限）" },
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
        className="flex items-center gap-1.5 bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-3 py-[5px] rounded border border-[var(--color-border)] disabled:opacity-50 cursor-pointer hover:border-[var(--color-accent)] hover:bg-[var(--color-bg-secondary)] transition-colors focus:outline-none focus:border-[var(--color-accent)]"
      >
        {current.label}
        <svg className="w-3 h-3 text-[var(--color-text-secondary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {open && !disabled && (
        <div className="absolute top-full left-0 mt-1 z-50 min-w-[260px] bg-[var(--color-bg-tertiary)] border border-[var(--color-border)] rounded shadow-lg overflow-hidden">
          {CAPTURE_MODES.map((mode) => (
            <button
              key={mode.value}
              type="button"
              onClick={() => {
                onChange(mode.value);
                setOpen(false);
              }}
              className={`w-full text-left px-3 py-2 text-xs transition-colors cursor-pointer ${
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

interface ToolbarProps {
  onToggleRules: () => void;
  showRules: boolean;
  onToggleStats: () => void;
  showStats: boolean;
  onOpenTransformer: () => void;
  onOpenAi: () => void;
  onOpenDiff: () => void;
  onOpenVuln: () => void;
  onOpenPlugins: () => void;
}

export function Toolbar({ onToggleRules, showRules, onToggleStats, showStats, onOpenTransformer, onOpenAi, onOpenDiff, onOpenVuln, onOpenPlugins }: ToolbarProps) {
  const captureStatus = useStore((s: StoreState) => s.captureStatus);
  const clearRequests = useStore((s: StoreState) => s.clearRequests);
  const setCaptureStatus = useStore((s: StoreState) => s.setCaptureStatus);
  const totalSessions = useStore((s: StoreState) => s.totalSessions);
  const [captureMode, setCaptureMode] = useState<CaptureMode>("DualProxy");
  const [captureHttps, setCaptureHttps] = useState(true);
  const [pending, setPending] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [themeMode, setThemeMode] = useState<ThemeMode>(getStoredTheme());

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
          capture_https: captureHttps,
          ports: [],
          process_filters: [],
          host_filters: [],
          max_body_size: 5 * 1024 * 1024,
          ca_cert_path: null,
          ca_key_path: null,
          mitm_bypass_hosts: [],
          proxy_port: 40960,
          transparent_proxy_port: 40961,
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

  const getPortDisplay = () => {
    if (captureMode === "DualProxy") {
      return "正向 :40960 | 透明 :40961";
    } else if (captureMode === "ForwardProxy") {
      return "正向 :40960";
    } else {
      return "透明 :40961";
    }
  };

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
      <button
        onClick={handleToggleCapture}
        disabled={pending}
        className={`px-4 py-[6px] rounded text-sm font-medium transition-colors disabled:opacity-50 cursor-pointer ${
          isRunning
            ? "bg-[var(--color-error)] hover:bg-red-600 active:bg-red-700 text-white"
            : "bg-[var(--color-success)] hover:bg-green-600 active:bg-green-700 text-white"
        }`}
      >
        {pending ? "..." : isRunning ? "■ 停止" : "▶ 开始"}
      </button>

      <ModeDropdown
        value={captureMode}
        onChange={setCaptureMode}
        disabled={isRunning || pending}
      />

      <label className="flex items-center gap-1.5 cursor-pointer select-none">
        <input
          type="checkbox"
          checked={captureHttps}
          onChange={(e) => setCaptureHttps(e.target.checked)}
          disabled={isRunning || pending}
          className="w-3.5 h-3.5 rounded border-[var(--color-border)] accent-[var(--color-accent)] cursor-pointer"
        />
        <span className="text-xs text-[var(--color-text-secondary)]">HTTPS 解密</span>
      </label>

      <button
        onClick={clearRequests}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] active:bg-[var(--color-bg-secondary)] transition-colors cursor-pointer"
      >
        清空
      </button>

      <button
        onClick={async () => {
          if (totalSessions === 0) return;
          try {
            const har = await exportHar([]);
            const blob = new Blob([har], { type: "application/json" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `flowreveal-${new Date().toISOString().slice(0, 19).replace(/:/g, "-")}.har`;
            a.click();
            URL.revokeObjectURL(url);
          } catch (e) {
            console.error("Export HAR failed:", e);
          }
        }}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] active:bg-[var(--color-bg-secondary)] transition-colors cursor-pointer"
        title="导出所有会话为 HAR 文件，可用 Chrome DevTools 打开分析"
        disabled={totalSessions === 0}
      >
        ↓ 导出HAR
      </button>

      {errorMsg && (
        <div className="text-xs text-[var(--color-error)] truncate max-w-[300px] px-2 py-1 bg-[var(--color-error)]/10 rounded" title={errorMsg}>
          ⚠ {errorMsg}
        </div>
      )}

      <div className="flex-1" />

      {isRunning && (
        <div className="text-xs text-[var(--color-accent)] font-mono flex items-center gap-1.5">
          <span>{getPortDisplay()}</span>
          {captureHttps && <span className="flex items-center gap-0.5" title="HTTPS 解密已启用" style={{ fontFamily: "'Segoe UI Emoji', 'Apple Color Emoji', sans-serif" }}>🔓 HTTPS</span>}
        </div>
      )}

      <button
        onClick={onToggleRules}
        className={`px-3 py-[6px] rounded text-sm transition-colors cursor-pointer ${
          showRules
            ? "bg-[var(--color-accent)] text-white"
            : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)]"
        }`}
        title="规则管理"
      >
        📋 规则
      </button>

      <button
        onClick={onToggleStats}
        className={`px-3 py-[6px] rounded text-sm transition-colors cursor-pointer ${
          showStats
            ? "bg-[var(--color-accent)] text-white"
            : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)]"
        }`}
        title="性能统计"
      >
        📊 统计
      </button>

      <button
        onClick={onOpenTransformer}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        title="数据转换器"
      >
        🔧 工具
      </button>

      <button
        onClick={onOpenAi}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        title="AI 智能分析"
      >
        🤖 AI
      </button>

      <button
        onClick={onOpenDiff}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        title="流量 Diff 对比"
      >
        🔄 Diff
      </button>

      <button
        onClick={onOpenVuln}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        title="漏洞扫描"
      >
        🛡️ 扫描
      </button>

      <button
        onClick={onOpenPlugins}
        className="px-3 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        title="插件管理"
      >
        🧩 插件
      </button>

      <button
        onClick={() => {
          const next: ThemeMode = themeMode === "dark" ? "light" : themeMode === "light" ? "system" : "dark";
          setThemeMode(next);
          setStoredTheme(next);
        }}
        className="px-2 py-[6px] rounded text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        title={`主题: ${themeMode === "dark" ? "暗色" : themeMode === "light" ? "亮色" : "跟随系统"} (点击切换)`}
      >
        {themeMode === "dark" ? "🌙" : themeMode === "light" ? "☀️" : "💻"}
      </button>

      <div className="text-xs text-[var(--color-text-secondary)]">
        FlowReveal
      </div>
    </div>
  );
}
