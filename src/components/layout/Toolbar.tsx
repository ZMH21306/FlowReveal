import { useState } from "react";
import { useStore, type StoreState } from "../../store";
import { startCapture, stopCapture, exportHar, requestElevation } from "../../lib/tauri-bindings";
import type { CaptureConfig } from "../../types";
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
  const setCaptureMode = useStore((s: StoreState) => s.setCaptureMode);
  const clearRequests = useStore((s: StoreState) => s.clearRequests);
  const [captureHttps, setCaptureHttps] = useState(true);
  const [globalCapture, setGlobalCapture] = useState(true);
  const [captureLocalhost, setCaptureLocalhost] = useState(false);
  const [capturePorts, setCapturePorts] = useState("80, 443");
  const [proxyPort, setProxyPort] = useState(8080);
  const [transparentPort, setTransparentPort] = useState(8081);
  const [pending, setPending] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [themeMode, setThemeMode] = useState<ThemeMode>(getStoredTheme());
  const [showPortConfig, setShowPortConfig] = useState(false);

  const handleStart = async () => {
    setPending(true);
    setErrorMsg(null);
    try {
      const ports = capturePorts.split(",").map(p => parseInt(p.trim())).filter(p => !isNaN(p) && p > 0 && p <= 65535);
      if (ports.length === 0) {
        setErrorMsg("请输入有效的捕获端口");
        setPending(false);
        return;
      }
      const config: CaptureConfig = {
        mode: globalCapture ? "Global" : "ProxyOnly",
        capture_http: true,
        capture_https: captureHttps,
        ports: [proxyPort, transparentPort],
        process_filters: [],
        host_filters: [],
        max_body_size: 5 * 1024 * 1024,
        ca_cert_path: null,
        ca_key_path: null,
        mitm_bypass_hosts: [],
        proxy_port: proxyPort,
        transparent_proxy_port: transparentPort,
        capture_ports: ports,
        exclude_pids: [],
        include_pids: [],
        capture_localhost: captureLocalhost,
      };
      await startCapture(config);
      setCaptureStatus("Running");
      setCaptureMode(globalCapture ? "Global" : "ProxyOnly");
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

  const handleRequestElevation = async () => {
    try {
      await requestElevation();
    } catch (e) {
      setErrorMsg(`提权失败: ${e}`);
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
        <label className="flex items-center gap-1 text-[11px] text-[var(--color-text-secondary)] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={globalCapture}
            onChange={(e) => setGlobalCapture(e.target.checked)}
            disabled={isRunning}
            className="accent-[var(--color-accent)]"
          />
          全局捕获
          {globalCapture && (
            <button
              onClick={handleRequestElevation}
              className="text-[9px] text-[var(--color-warning)] hover:text-[var(--color-accent)] underline cursor-pointer"
              title="点击请求管理员提权"
            >
              ⚠ 需管理员
            </button>
          )}
        </label>
        {globalCapture && (
          <label className="flex items-center gap-1 text-[11px] text-[var(--color-text-secondary)] cursor-pointer select-none">
            <input
              type="checkbox"
              checked={captureLocalhost}
              onChange={(e) => setCaptureLocalhost(e.target.checked)}
              disabled={isRunning}
              className="accent-[var(--color-accent)]"
            />
            localhost
          </label>
        )}
        <button
          onClick={() => setShowPortConfig(!showPortConfig)}
          className="px-1.5 py-1 text-[10px] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] rounded transition-all cursor-pointer"
          title="端口配置"
        >
          ⚙ 端口
        </button>
      </div>

      {showPortConfig && (
        <div className="flex items-center gap-2 px-2 py-1 bg-[var(--color-bg-tertiary)] rounded border border-[var(--color-border)] text-[10px]">
          <label className="flex items-center gap-1 text-[var(--color-text-secondary)]">
            代理:
            <input
              type="number"
              value={proxyPort}
              onChange={(e) => setProxyPort(parseInt(e.target.value) || 8080)}
              disabled={isRunning}
              className="w-14 px-1 py-0.5 bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)] text-[10px] text-center"
              min={1}
              max={65535}
            />
          </label>
          <label className="flex items-center gap-1 text-[var(--color-text-secondary)]">
            透明:
            <input
              type="number"
              value={transparentPort}
              onChange={(e) => setTransparentPort(parseInt(e.target.value) || 8081)}
              disabled={isRunning}
              className="w-14 px-1 py-0.5 bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)] text-[10px] text-center"
              min={1}
              max={65535}
            />
          </label>
          <label className="flex items-center gap-1 text-[var(--color-text-secondary)]">
            捕获端口:
            <input
              type="text"
              value={capturePorts}
              onChange={(e) => setCapturePorts(e.target.value)}
              disabled={isRunning}
              className="w-20 px-1 py-0.5 bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)] text-[10px]"
              placeholder="80, 443"
            />
          </label>
        </div>
      )}

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
