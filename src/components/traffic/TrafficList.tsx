import { useState } from "react";
import { useStore, type StoreState } from "../../store";
import { methodColor, statusCodeColor, formatDuration, formatSize } from "../../lib/utils";
import { ContextMenu } from "./ContextMenu";
import { useVirtualScroll } from "../../hooks/useVirtualScroll";
import type { HttpSession } from "../../types";

const ROW_HEIGHT = 28;

export function TrafficList() {
  const sessions = useStore((s: StoreState) => s.sessions);
  const filteredSessionList = useStore((s: StoreState) => s.filteredSessionList);
  const selectedId = useStore((s: StoreState) => s.selectedId);
  const selectRequest = useStore((s: StoreState) => s.selectRequest);
  const toggleBookmark = useStore((s: StoreState) => s.toggleBookmark);
  const bookmarks = useStore((s: StoreState) => s.bookmarks);
  const setFilter = useStore((s: StoreState) => s.setFilter);
  const currentPage = useStore((s: StoreState) => s.currentPage);
  const pageSize = useStore((s: StoreState) => s.pageSize);
  const setPage = useStore((s: StoreState) => s.setPage);
  const totalPages = Math.max(1, Math.ceil(filteredSessionList.length / pageSize));

  const [showContextMenu, setShowContextMenu] = useState(false);
  const [contextMenuPos, setContextMenuPos] = useState({ x: 0, y: 0 });
  const [selectedSession, setSelectedSession] = useState<HttpSession | null>(null);

  const { containerRef, visibleItems, offsetY, totalHeight } = useVirtualScroll({
    itemCount: filteredSessionList.length,
    itemHeight: ROW_HEIGHT,
    overscan: 10,
  });

  const getSession = (id: number) => sessions.get(id);

  const handleContextMenu = (e: React.MouseEvent, session: HttpSession) => {
    e.preventDefault();
    setContextMenuPos({ x: e.clientX, y: e.clientY });
    setSelectedSession(session);
    setShowContextMenu(true);
  };

  const handleCopyUrl = (session: HttpSession) => {
    navigator.clipboard.writeText(session.request.url || "");
  };

  const handleCopyResponseBody = (session: HttpSession) => {
    const body = session.response?.body;
    if (body) {
      navigator.clipboard.writeText(new TextDecoder().decode(new Uint8Array(body)));
    }
  };

  const handleCopyRequestBody = (session: HttpSession) => {
    const body = session.request.body;
    if (body) {
      navigator.clipboard.writeText(new TextDecoder().decode(new Uint8Array(body)));
    }
  };

  const handleCopyHeaders = (session: HttpSession) => {
    const headers = session.request.headers.map(([k, v]) => `${k}: ${v}`).join("\n");
    navigator.clipboard.writeText(headers);
  };

  const handleCopyCurl = (session: HttpSession) => {
    const req = session.request;
    let curl = `curl -X ${req.method || "GET"} '${req.url || ""}'`;
    req.headers.forEach(([k, v]) => {
      curl += ` \\\n  -H '${k}: ${v}'`;
    });
    if (req.body && req.body.length > 0) {
      const bodyStr = new TextDecoder().decode(new Uint8Array(req.body));
      curl += ` \\\n  -d '${bodyStr}'`;
    }
    navigator.clipboard.writeText(curl);
  };

  const handleFilterByHost = (session: HttpSession) => {
    const url = session.request.url || "";
    try {
      const host = new URL(url).hostname;
      setFilter({ searchText: host });
    } catch {
      const hostHeader = session.request.headers.find(([k]) => k.toLowerCase() === "host");
      if (hostHeader) setFilter({ searchText: hostHeader[1] });
    }
  };

  const handleFilterByUrl = (session: HttpSession) => {
    const url = session.request.url || "";
    try {
      const parsed = new URL(url);
      setFilter({ searchText: parsed.origin + parsed.pathname });
    } catch {
      setFilter({ searchText: url });
    }
  };

  const handleBookmark = (session: HttpSession) => {
    toggleBookmark(session.id);
  };

  const handleReplay = (session: HttpSession) => {
    console.log("Replay request:", session.id);
  };

  const handleOpenInBrowser = (session: HttpSession) => {
    const url = session.request.url;
    if (url) window.open(url, "_blank");
  };

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <div className="grid grid-cols-[28px_56px_72px_1fr_64px_72px_72px_96px] gap-0 px-3 py-1.5 text-[11px] text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] select-none shrink-0">
        <span></span>
        <span>#</span>
        <span>方法</span>
        <span>URL</span>
        <span>状态</span>
        <span>大小</span>
        <span>耗时</span>
        <span>进程</span>
      </div>

      <div ref={containerRef} className="flex-1 overflow-y-auto" style={{ position: "relative" }}>
        {filteredSessionList.length === 0 ? (
          <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm">
            暂无抓包数据，点击"开始"启动抓包
          </div>
        ) : (
          <div style={{ height: totalHeight, position: "relative" }}>
            <div style={{ position: "absolute", top: offsetY, left: 0, right: 0 }}>
              {visibleItems.map((index) => {
                const sid = filteredSessionList[index];
                if (sid === undefined) return null;
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
                const isHttps = req.scheme === "Https" || req.url?.startsWith("https://");
                const isBookmarked = bookmarks.has(sid);
                const protocol = req.protocol;
                const isH2 = protocol === "HTTP2";
                const isWs = protocol === "WebSocket";
                const streamId = req.stream_id;

                return (
                  <div
                    key={sid}
                    onClick={() => selectRequest(sid)}
                    onContextMenu={(e) => handleContextMenu(e, session)}
                    className={`grid grid-cols-[28px_56px_72px_1fr_64px_72px_72px_96px] gap-0 px-3 text-[11px] cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] transition-colors items-center ${
                      selectedId === sid
                        ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]"
                        : ""
                    }`}
                    style={{ height: ROW_HEIGHT }}
                  >
                    <span
                      className="flex items-center justify-center cursor-pointer"
                      onClick={(e) => { e.stopPropagation(); toggleBookmark(sid); }}
                      title={isBookmarked ? "移除书签" : "添加书签"}
                    >
                      {isBookmarked ? (
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--color-warning)" stroke="var(--color-warning)" strokeWidth="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" /></svg>
                      ) : (
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-secondary)" strokeWidth="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" /></svg>
                      )}
                    </span>
                    <span className="text-[var(--color-text-secondary)] whitespace-nowrap flex items-center gap-[2px]" title={streamId != null ? `Stream #${streamId}` : undefined}>
                      {isH2 && <span className="text-[9px] font-bold text-[var(--color-accent)] leading-none">h2</span>}
                      {isWs && <span className="text-[9px] font-bold text-[var(--color-success)] leading-none">WS</span>}
                      {isDecrypted && !isH2 && !isWs && <span className="text-[10px] leading-none" style={{ fontFamily: "'Segoe UI Emoji', 'Apple Color Emoji', sans-serif" }}>🔓</span>}
                      {!isDecrypted && isHttps && method === "CONNECT" && !isH2 && !isWs && <span className="text-[10px] leading-none" style={{ fontFamily: "'Segoe UI Emoji', 'Apple Color Emoji', sans-serif" }}>🔒</span>}
                      <span className="font-mono">{sid}</span>
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
          </div>
        )}
      </div>

      {filteredSessionList.length > pageSize && (
        <div className="flex items-center justify-between px-3 py-1 text-[11px] text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] border-t border-[var(--color-border)] shrink-0">
          <span>
            共 {filteredSessionList.length} 条 | 第 {currentPage}/{totalPages} 页
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setPage(1)}
              disabled={currentPage <= 1}
              className="px-1.5 py-0.5 rounded bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-40 disabled:cursor-not-allowed"
            >
              ⏮
            </button>
            <button
              onClick={() => setPage(currentPage - 1)}
              disabled={currentPage <= 1}
              className="px-1.5 py-0.5 rounded bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-40 disabled:cursor-not-allowed"
            >
              ◀
            </button>
            <button
              onClick={() => setPage(currentPage + 1)}
              disabled={currentPage >= totalPages}
              className="px-1.5 py-0.5 rounded bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-40 disabled:cursor-not-allowed"
            >
              ▶
            </button>
            <button
              onClick={() => setPage(totalPages)}
              disabled={currentPage >= totalPages}
              className="px-1.5 py-0.5 rounded bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-40 disabled:cursor-not-allowed"
            >
              ⏭
            </button>
          </div>
        </div>
      )}

      {showContextMenu && selectedSession && (
        <ContextMenu
          x={contextMenuPos.x}
          y={contextMenuPos.y}
          session={selectedSession}
          onClose={() => setShowContextMenu(false)}
          onCopyUrl={handleCopyUrl}
          onCopyResponseBody={handleCopyResponseBody}
          onCopyRequestBody={handleCopyRequestBody}
          onCopyHeaders={handleCopyHeaders}
          onCopyCurl={handleCopyCurl}
          onFilterByHost={handleFilterByHost}
          onFilterByUrl={handleFilterByUrl}
          onBookmark={handleBookmark}
          onReplay={handleReplay}
          onOpenInBrowser={handleOpenInBrowser}
        />
      )}
    </div>
  );
}
