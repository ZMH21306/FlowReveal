import { useRef, useEffect, useState, useCallback } from "react";
import { useStore } from "../../store";
import { methodColor, statusCodeColor, formatDuration, formatSize } from "../../lib/utils";
import type { HttpSession } from "../../types";

const ROW_HEIGHT = 28;
const OVERSCAN = 10;

export function TrafficList() {
  const sessions = useStore((s) => s.sessions);
  const filteredSessionList = useStore((s) => s.filteredSessionList);
  const selectedId = useStore((s) => s.selectedId);
  const selectRequest = useStore((s) => s.selectRequest);

  const containerRef = useRef<HTMLDivElement>(null);
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: 30 });

  const totalHeight = filteredSessionList.length * ROW_HEIGHT;

  const updateVisibleRange = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    const scrollTop = container.scrollTop;
    const viewportHeight = container.clientHeight;
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
    const end = Math.min(
      filteredSessionList.length,
      Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN
    );
    setVisibleRange({ start, end });
  }, [filteredSessionList.length]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    container.addEventListener("scroll", updateVisibleRange, { passive: true });
    updateVisibleRange();
    return () => container.removeEventListener("scroll", updateVisibleRange);
  }, [updateVisibleRange]);

  const getSession = (id: number): HttpSession | undefined => sessions.get(id);

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <div className="grid grid-cols-[50px_70px_1fr_60px_70px_70px_90px] gap-0 px-3 py-1.5 text-xs text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] sticky top-0 select-none z-10">
        <span>#</span>
        <span>方法</span>
        <span>URL</span>
        <span>状态</span>
        <span>大小</span>
        <span>耗时</span>
        <span>进程</span>
      </div>

      <div ref={containerRef} className="flex-1 overflow-y-auto relative">
        {filteredSessionList.length === 0 ? (
          <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm">
            暂无抓包数据，点击"开始"启动抓包
          </div>
        ) : (
          <div style={{ height: totalHeight, position: "relative" }}>
            {filteredSessionList.slice(visibleRange.start, visibleRange.end).map((sid) => {
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
              const isHttps = req.scheme === "Https";
              const isDecrypted = req.raw_tls_info != null;
              const rowIndex = filteredSessionList.indexOf(sid);

              return (
                <div
                  key={sid}
                  onClick={() => selectRequest(sid)}
                  style={{
                    position: "absolute",
                    top: rowIndex * ROW_HEIGHT,
                    left: 0,
                    right: 0,
                    height: ROW_HEIGHT,
                  }}
                  className={`grid grid-cols-[50px_70px_1fr_60px_70px_70px_90px] gap-0 px-3 py-1 text-xs cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] transition-colors ${
                    selectedId === sid
                      ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]"
                      : ""
                  }`}
                >
                  <span className="text-[var(--color-text-secondary)] font-mono leading-5">
                    {isDecrypted ? "🔓" : isHttps ? "🔒" : ""}{sid}
                  </span>
                  <span
                    style={{ color: methodColor(method) }}
                    className="font-mono font-semibold leading-5"
                  >
                    {method}
                  </span>
                  <span className="truncate leading-5" title={url}>
                    {url}
                  </span>
                  <span
                    style={{ color: statusCodeColor(statusCode ?? null) }}
                    className="font-mono leading-5"
                  >
                    {statusCode ?? "..."}
                  </span>
                  <span className="text-[var(--color-text-secondary)] leading-5">
                    {formatSize(bodySize)}
                  </span>
                  <span className="text-[var(--color-text-secondary)] leading-5">
                    {formatDuration(duration ?? null)}
                  </span>
                  <span className="text-[var(--color-text-secondary)] truncate leading-5">
                    {process}
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
