import { useRef } from "react";
import { useStore } from "../../store";
import { methodColor, statusCodeColor, formatDuration, formatSize } from "../../lib/utils";

export function TrafficList() {
  const sessions = useStore((s) => s.sessions);
  const filteredSessionList = useStore((s) => s.filteredSessionList);
  const selectedId = useStore((s) => s.selectedId);
  const selectRequest = useStore((s) => s.selectRequest);

  const listRef = useRef<HTMLDivElement>(null);

  const getSession = (id: number) => sessions.get(id);

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <div className="grid grid-cols-[56px_72px_1fr_64px_72px_72px_96px] gap-0 px-3 py-1.5 text-[11px] text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] select-none shrink-0">
        <span>#</span>
        <span>方法</span>
        <span>URL</span>
        <span>状态</span>
        <span>大小</span>
        <span>耗时</span>
        <span>进程</span>
      </div>

      <div ref={listRef} className="flex-1 overflow-y-auto">
        {filteredSessionList.length === 0 ? (
          <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm">
            暂无抓包数据，点击"开始"启动抓包
          </div>
        ) : (
          <div className="flex flex-col">
            {filteredSessionList.map((sid) => {
              const session = getSession(sid);
              if (!session) return null;
              const req = session.request;
              const resp = session.response;
              const method = req.method || "-";
              const url = req.url || "-";
              const statusCode = resp?.status_code;
              const bodySize = (req.body_size || 0) + (resp?.body_size || 0);
              const duration = resp?.duration_us;
              const process = req.process_name || "";
              const isDecrypted = req.raw_tls_info != null;

              return (
                <div
                  key={sid}
                  onClick={() => selectRequest(sid)}
                  className={`grid grid-cols-[56px_72px_1fr_64px_72px_72px_96px] gap-0 px-3 py-[5px] text-[11px] cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] transition-colors items-center ${
                    selectedId === sid
                      ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]"
                      : ""
                  }`}
                >
                  <span className="text-[var(--color-text-secondary)] font-mono whitespace-nowrap">
                    {isDecrypted ? "🔓 " : ""}{sid}
                  </span>
                  <span
                    style={{ color: methodColor(method) }}
                    className="font-mono font-semibold whitespace-nowrap"
                  >
                    {method}
                  </span>
                  <span className="truncate" title={url}>
                    {url}
                  </span>
                  <span
                    style={{ color: statusCodeColor(statusCode ?? null) }}
                    className="font-mono whitespace-nowrap"
                  >
                    {statusCode ?? "..."}
                  </span>
                  <span className="text-[var(--color-text-secondary)] whitespace-nowrap">
                    {formatSize(bodySize)}
                  </span>
                  <span className="text-[var(--color-text-secondary)] whitespace-nowrap">
                    {formatDuration(duration ?? null)}
                  </span>
                  <span className="text-[var(--color-text-secondary)] truncate">
                    {process || "-"}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
