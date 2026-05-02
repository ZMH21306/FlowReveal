import { useStore } from "../../store";
import { formatSize } from "../../lib/utils";

const STATUS_LABELS: Record<string, string> = {
  Idle: "空闲",
  Running: "运行中",
  Error: "错误",
};

export function StatusBar() {
  const totalSessions = useStore((s) => s.totalSessions);
  const activeSessions = useStore((s) => s.activeSessions);
  const bytesCaptured = useStore((s) => s.bytesCaptured);
  const decryptedCount = useStore((s) => s.decryptedCount);
  const captureStatus = useStore((s) => s.captureStatus);

  return (
    <div className="flex items-center gap-6 px-4 py-1.5 bg-[var(--color-bg-secondary)] border-t border-[var(--color-border)] text-xs text-[var(--color-text-secondary)]">
      <span className="flex items-center gap-1.5">
        <span
          className={`w-2 h-2 rounded-full ${
            captureStatus === "Running"
              ? "bg-[var(--color-success)]"
              : captureStatus === "Error"
                ? "bg-[var(--color-error)]"
                : "bg-[var(--color-text-secondary)]"
          }`}
        />
        {STATUS_LABELS[captureStatus] ?? captureStatus}
      </span>
      <span>会话: {totalSessions}</span>
      <span>活跃: {activeSessions}</span>
      {decryptedCount > 0 && (
        <span className="text-[var(--color-accent)]" style={{ fontFamily: "'Segoe UI Emoji', 'Apple Color Emoji', sans-serif" }}>
          🔓 已解密: {decryptedCount}
        </span>
      )}
      <span>数据量: {formatSize(bytesCaptured)}</span>
    </div>
  );
}
