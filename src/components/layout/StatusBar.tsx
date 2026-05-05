import { useStore, type StoreState } from "../../store";
import { formatSize } from "../../lib/utils";

const STATUS_LABELS: Record<string, string> = {
  Idle: "空闲",
  Running: "运行中",
  Error: "错误",
};

const MODE_LABELS: Record<string, { label: string; color: string }> = {
  Global: { label: "全局捕获", color: "text-[var(--color-accent)]" },
  ProxyOnly: { label: "仅代理", color: "text-[var(--color-text-muted)]" },
};

const DIVERTER_LABELS: Record<string, { label: string; color: string }> = {
  NotAvailable: { label: "不可用", color: "text-[var(--color-text-muted)]" },
  Stopped: { label: "已停止", color: "text-[var(--color-text-muted)]" },
  Running: { label: "运行中", color: "text-[var(--color-success)]" },
  Error: { label: "错误", color: "text-[var(--color-error)]" },
};

export function StatusBar() {
  const totalSessions = useStore((s: StoreState) => s.totalSessions);
  const activeSessions = useStore((s: StoreState) => s.activeSessions);
  const bytesCaptured = useStore((s: StoreState) => s.bytesCaptured);
  const decryptedCount = useStore((s: StoreState) => s.decryptedCount);
  const captureStatus = useStore((s: StoreState) => s.captureStatus);
  const captureMode = useStore((s: StoreState) => s.captureMode);
  const diverterStatus = useStore((s: StoreState) => s.diverterStatus);
  const isElevated = useStore((s: StoreState) => s.isElevated);
  const isWifi = useStore((s: StoreState) => s.isWifi);

  const divertInfo = DIVERTER_LABELS[diverterStatus] ?? { label: diverterStatus, color: "text-[var(--color-text-muted)]" };
  const modeInfo = MODE_LABELS[captureMode] ?? { label: captureMode, color: "text-[var(--color-text-muted)]" };

  return (
    <div className="flex items-center gap-5 px-4 py-1 bg-[var(--color-bg-secondary)] border-t border-[var(--color-border)] text-[11px] text-[var(--color-text-muted)] shrink-0">
      <span className="flex items-center gap-1.5">
        <span
          className={`w-1.5 h-1.5 rounded-full ${
            captureStatus === "Running"
              ? "bg-[var(--color-success)] shadow-[0_0_6px_var(--color-success)]"
              : captureStatus === "Error"
                ? "bg-[var(--color-error)]"
                : "bg-[var(--color-text-muted)]"
          }`}
        />
        <span className="text-[var(--color-text-secondary)]">{STATUS_LABELS[captureStatus] ?? captureStatus}</span>
      </span>
      {captureStatus === "Running" && (
        <span className="flex items-center gap-1">
          <span>模式:</span>
          <span className={`font-medium ${modeInfo.color}`}>{modeInfo.label}</span>
        </span>
      )}
      <span className="flex items-center gap-1">
        <span>WinDivert:</span>
        <span className={`font-medium ${divertInfo.color}`}>{divertInfo.label}</span>
      </span>
      {!isElevated && captureStatus === "Running" && (
        <span className="text-[var(--color-warning)] flex items-center gap-0.5">
          ⚠ 非管理员
        </span>
      )}
      {isWifi && captureStatus === "Running" && (
        <span className="text-[var(--color-warning)] flex items-center gap-0.5">
          ⚠ Wi-Fi
        </span>
      )}
      <span>会话 <span className="text-[var(--color-text-primary)] font-medium">{totalSessions}</span></span>
      <span>活跃 <span className="text-[var(--color-text-primary)] font-medium">{activeSessions}</span></span>
      {decryptedCount > 0 && (
        <span className="text-[var(--color-accent)]">
          🔓 {decryptedCount}
        </span>
      )}
      <span>数据 <span className="text-[var(--color-text-primary)] font-medium">{formatSize(bytesCaptured)}</span></span>
      <div className="flex-1" />
      <span className="text-[9px] opacity-50">FlowReveal v1.0</span>
    </div>
  );
}
