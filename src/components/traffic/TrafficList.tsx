import { useStore } from "../../store";
import {
  methodColor,
  statusCodeColor,
  formatDuration,
  formatSize,
} from "../../lib/utils";
import type { HttpSession } from "../../types";

export function TrafficList() {
  const sessions = useStore((s) => s.sessions);
  const sessionList = useStore((s) => s.sessionList);
  const selectedId = useStore((s) => s.selectedId);
  const selectRequest = useStore((s) => s.selectRequest);

  const getSession = (id: number): HttpSession | undefined => sessions.get(id);

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <div className="grid grid-cols-[50px_70px_1fr_60px_70px_70px_90px] gap-0 px-3 py-1.5 text-xs text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] sticky top-0 select-none">
        <span>#</span>
        <span>Method</span>
        <span>URL</span>
        <span>Status</span>
        <span>Size</span>
        <span>Time</span>
        <span>Process</span>
      </div>

      <div className="flex-1 overflow-y-auto">
        {sessionList.length === 0 ? (
          <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm">
            No traffic captured yet. Click "Start" to begin.
          </div>
        ) : (
          sessionList.map((sid) => {
            const session = getSession(sid);
            if (!session) return null;
            const req = session.request;
            const resp = session.response;
            const method = req.method || "-";
            const url = req.url || "-";
            const statusCode = resp?.status_code;
            const bodySize = (req.body_size || 0) + (resp?.body_size || 0);
            const duration = resp?.duration_us;
            const process = req.process_name || "-";

            return (
              <div
                key={sid}
                onClick={() => selectRequest(sid)}
                className={`grid grid-cols-[50px_70px_1fr_60px_70px_70px_90px] gap-0 px-3 py-1 text-xs cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] transition-colors ${
                  selectedId === sid
                    ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]"
                    : ""
                }`}
              >
                <span className="text-[var(--color-text-secondary)] font-mono">
                  {sid}
                </span>
                <span
                  style={{ color: methodColor(method) }}
                  className="font-mono font-semibold"
                >
                  {method}
                </span>
                <span className="truncate" title={url}>
                  {url}
                </span>
                <span
                  style={{ color: statusCodeColor(statusCode ?? null) }}
                  className="font-mono"
                >
                  {statusCode ?? "..."}
                </span>
                <span className="text-[var(--color-text-secondary)]">
                  {formatSize(bodySize)}
                </span>
                <span className="text-[var(--color-text-secondary)]">
                  {formatDuration(duration ?? null)}
                </span>
                <span className="text-[var(--color-text-secondary)] truncate">
                  {process}
                </span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
