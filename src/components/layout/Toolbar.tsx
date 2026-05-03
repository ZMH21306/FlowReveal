import { useState } from "react";
import { useStore, type StoreState } from "../../store";
import { startCapture, stopCapture, exportHar } from "../../lib/tauri-bindings";
import type { CaptureConfig, CaptureMode } from "../../types";
import { getStoredTheme, setStoredTheme, type ThemeMode } from "../../hooks/useTheme";

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
  const setCaptureStatus = useStore((s: StoreState) => s.setCaptureStatus);
  const clearRequests = useStore((s: StoreState) => s.clearRequests);
  const [captureMode, setCaptureMode] = useState<CaptureMode>("DualProxy");
  const [captureHttps, setCaptureHttps] = useState(true);
  const [pending, setPending] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [themeMode, setThemeMode] = useState<ThemeMode>(getStoredTheme());

  const handleStart = async () => {
    setPending(true);
    setErrorMsg(null);
    try {
      const config: CaptureConfig = {
        mode: captureMode,
        capture_http: true,
        capture_https: captureHttps,
        ports: [8080, 8081],
        process_filters: [],
        host_filters: [],
        max_body_size: 5 * 1024 * 1024,
        ca_cert_path: null,
        ca_key_path: null,
        mitm_bypass_hosts: [],
        proxy_port: 8080,
        transparent_proxy_port: 8081,
      };
      await startCapture(config);
      setCaptureStatus("Running");
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setPending(false);
    }
  };

  const handleStop = async () => {
    setPending(true);
    try {
      await stopCapture();
      setCaptureStatus("Idle");
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setPending(false);
    }
  };

  const handleExport = async () => {
    try {
      const har = await exportHar([]);
      const blob = new Blob([har], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `flowreveal-${Date.now()}.har`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Export HAR failed:", e);
    }
  };

  const isRunning = captureStatus === "Running";

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] shadow-[var(--shadow-sm)] shrink-0">
      <div className="flex items-center gap-1.5">
        <span className="text-sm font-bold tracking-tight text-[var(--color-accent)] mr-1">FlowReveal</span>
      </div>

      <div className="w-px h-5 bg-[var(--color-border)]" />

      <div className="flex items-center gap-1.5">
        {isRunning ? (
          <button
            onClick={handleStop}
            disabled={pending}
            className="px-3 py-1.5 text-xs font-medium rounded-[var(--radius-md)] bg-[var(--color-error)] text-white hover:opacity-90 disabled:opacity-50 transition-all shadow-[var(--shadow-sm)]"
          >
            ■ 停止
          </button>
        ) : (
          <button
            onClick={handleStart}
            disabled={pending}
            className="px-3 py-1.5 text-xs font-medium rounded-[var(--radius-md)] bg-[var(--color-success)] text-white hover:opacity-90 disabled:opacity-50 transition-all shadow-[var(--shadow-sm)]"
          >
            ▶ 开始
          </button>
        )}
        <select
          value={captureMode}
          onChange={(e) => setCaptureMode(e.target.value as CaptureMode)}
          disabled={isRunning}
          className="px-2 py-1.5 text-[11px] bg-[var(--color-bg-tertiary)] border border-[var(--color-border-subtle)] rounded-[var(--radius-md)] text-[var(--color-text-primary)] disabled:opacity-50"
        >
          <option value="DualProxy">双代理</option>
          <option value="ForwardProxy">正向代理</option>
          <option value="Transparent">透明代理</option>
        </select>
        <label className="flex items-center gap-1 text-[11px] text-[var(--color-text-secondary)] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={captureHttps}
            onChange={(e) => setCaptureHttps(e.target.checked)}
            disabled={isRunning}
            className="accent-[var(--color-accent)]"
          />
          HTTPS 解密
        </label>
      </div>

      <div className="w-px h-5 bg-[var(--color-border)]" />

      <div className="flex items-center gap-1">
        <button
          onClick={clearRequests}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="清空所有请求"
        >
          清空
        </button>
        <button
          onClick={handleExport}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="导出 HAR 文件"
        >
          ↓ 导出HAR
        </button>
      </div>

      <div className="w-px h-5 bg-[var(--color-border)]" />

      <div className="flex items-center gap-0.5">
        <button
          onClick={onToggleRules}
          className={`px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] transition-all ${
            showRules
              ? "bg-[var(--color-accent-muted)] text-[var(--color-accent)]"
              : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)]"
          }`}
          title="规则管理"
        >
          📋 规则
        </button>
        <button
          onClick={onToggleStats}
          className={`px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] transition-all ${
            showStats
              ? "bg-[var(--color-accent-muted)] text-[var(--color-accent)]"
              : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)]"
          }`}
          title="性能统计"
        >
          📊 统计
        </button>
        <button
          onClick={onOpenTransformer}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="数据转换器"
        >
          🔧 工具
        </button>
      </div>

      <div className="w-px h-5 bg-[var(--color-border)]" />

      <div className="flex items-center gap-0.5">
        <button
          onClick={onOpenAi}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="AI 智能分析"
        >
          🤖 AI
        </button>
        <button
          onClick={onOpenDiff}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="流量 Diff 对比"
        >
          🔄 Diff
        </button>
        <button
          onClick={onOpenVuln}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="漏洞扫描"
        >
          🛡️ 扫描
        </button>
        <button
          onClick={onOpenPlugins}
          className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          title="插件管理"
        >
          🧩 插件
        </button>
      </div>

      <div className="flex-1" />

      {errorMsg && (
        <span className="text-[10px] text-[var(--color-error)] bg-[var(--color-error-muted)] px-2 py-1 rounded-[var(--radius-sm)]">
          {errorMsg}
        </span>
      )}

      <button
        onClick={() => {
          const next: ThemeMode = themeMode === "dark" ? "light" : themeMode === "light" ? "system" : "dark";
          setThemeMode(next);
          setStoredTheme(next);
        }}
        className="px-2 py-1.5 rounded-[var(--radius-md)] text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all cursor-pointer"
        title={`主题: ${themeMode === "dark" ? "暗色" : themeMode === "light" ? "亮色" : "跟随系统"} (点击切换)`}
      >
        {themeMode === "dark" ? "🌙" : themeMode === "light" ? "☀️" : "💻"}
      </button>
    </div>
  );
}
