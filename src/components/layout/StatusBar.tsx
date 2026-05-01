import { useStore } from "../../store";
import { formatSize } from "../../lib/utils";

export function StatusBar() {
  const stats = useStore((s) => s.stats);
  const captureStatus = useStore((s) => s.captureStatus);
  const sessions = useStore((s) => s.sessions);

  const httpsCount = Array.from(sessions.values()).filter(
    (s) => s.request.scheme === "Https" && s.request.raw_tls_info != null
  ).length;

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
        {captureStatus}
      </span>
      <span>Sessions: {stats.total_sessions}</span>
      <span>Active: {stats.active_sessions}</span>
      {httpsCount > 0 && (
        <span className="text-[var(--color-accent)]">
          🔒 Decrypted: {httpsCount}
        </span>
      )}
      <span>Data: {formatSize(stats.bytes_captured)}</span>
    </div>
  );
}
