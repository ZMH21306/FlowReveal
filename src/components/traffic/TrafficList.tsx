import { useStore } from "../../store";
import {
  methodColor,
  statusCodeColor,
  formatDuration,
  formatSize,
} from "../../lib/utils";

export function TrafficList() {
  const requests = useStore((s) => s.requests);
  const selectedId = useStore((s) => s.selectedId);
  const selectRequest = useStore((s) => s.selectRequest);

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <div className="grid grid-cols-[60px_70px_1fr_70px_80px_80px_100px] gap-0 px-3 py-1.5 text-xs text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] sticky top-0">
        <span>#</span>
        <span>Method</span>
        <span>URL</span>
        <span>Status</span>
        <span>Size</span>
        <span>Time</span>
        <span>Process</span>
      </div>

      <div className="flex-1 overflow-y-auto">
        {requests.length === 0 ? (
          <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm">
            No traffic captured yet. Click "Start" to begin.
          </div>
        ) : (
          requests.map((req) => (
            <div
              key={req.id}
              onClick={() => selectRequest(req.id)}
              className={`grid grid-cols-[60px_70px_1fr_70px_80px_80px_100px] gap-0 px-3 py-1 text-xs cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] transition-colors ${
                selectedId === req.id
                  ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]"
                  : ""
              }`}
            >
              <span className="text-[var(--color-text-secondary)]">
                {req.session_id}
              </span>
              <span
                style={{ color: methodColor(req.method) }}
                className="font-mono font-semibold"
              >
                {req.method || "-"}
              </span>
              <span className="truncate" title={req.url || ""}>
                {req.url || "-"}
              </span>
              <span
                style={{ color: statusCodeColor(req.status_code) }}
                className="font-mono"
              >
                {req.status_code ?? "-"}
              </span>
              <span className="text-[var(--color-text-secondary)]">
                {formatSize(req.body_size)}
              </span>
              <span className="text-[var(--color-text-secondary)]">
                {formatDuration(req.duration_us)}
              </span>
              <span className="text-[var(--color-text-secondary)] truncate">
                {req.process_name || "-"}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
