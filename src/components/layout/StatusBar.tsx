import { useStore, type StoreState } from "../../store";
import { formatSize } from "../../lib/utils";

const STATUS_LABELS: Record<string, string> = {
  Idle: "空闲",
  Running: "运行中",
  Error: "错误",
};

export function StatusBar() {
  const totalSessions = useStore((s: StoreState) => s.totalSessions);
  const activeSessions = useStore((s: StoreState) => s.activeSessions);
  const bytesCaptured = useStore((s: StoreState) => s.bytesCaptured);
  const decryptedCount = useStore((s: StoreState) => s.decryptedCount);
  const captureStatus = useStore((s: StoreState) => s.captureStatus);

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
