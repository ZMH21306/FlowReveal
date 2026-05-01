import { useStore } from "../../store";
import { formatSize } from "../../lib/utils";

export function StatusBar() {
  const stats = useStore((s) => s.stats);
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
        {captureStatus}
      </span>
      <span>Sessions: {stats.total_sessions}</span>
      <span>Active: {stats.active_sessions}</span>
      <span>Data: {formatSize(stats.bytes_captured)}</span>
    </div>
  );
}
