import { useState } from "react";
import { useStore, type StoreState } from "../../store";
import { methodColor, statusCodeColor, formatDuration, formatSize } from "../../lib/utils";
import { ContextMenu } from "./ContextMenu";
import { useVirtualScroll } from "../../hooks/useVirtualScroll";
import type { HttpSession } from "../../types";

const ROW_HEIGHT = 30;

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

  const handleFilterByProcess = (session: HttpSession) => {
    const processName = session.request.process_name;
    if (processName) {
      setFilter({ searchText: processName });
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
      <div className="grid grid-cols-[30px_50px_64px_1fr_56px_64px_64px_80px] gap-0 px-4 py-2 text-[10px] font-semibold text-[var(--color-text-muted)] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] select-none shrink-0 uppercase tracking-wider">
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
          <div className="flex flex-col items-center justify-center h-full text-[var(--color-text-muted)]">
            <span className="text-3xl mb-3 opacity-40">📡</span>
            <p className="text-sm">暂无抓包数据</p>
            <p className="text-xs mt-1 opacity-60">点击"▶ 开始"启动抓包</p>
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
                const duration = resp?.duration_us ?? (resp ? (resp.timestamp - req.timestamp) : null);
                const process = req.process_name || resp?.process_name || "";
                const processId = req.process_id || resp?.process_id;
                const isDecrypted = req.raw_tls_info != null;
                const isHttps = req.scheme === "Https" || req.url?.startsWith("https://");
                const isBookmarked = bookmarks.has(sid);
                const protocol = req.protocol;
                const isH2 = protocol === "HTTP2";
                const isWs = protocol === "WebSocket";
                const streamId = req.stream_id;
                const isSelected = selectedId === sid;

                return (
                  <div
                    key={sid}
                    onClick={() => selectRequest(sid)}
                    onContextMenu={(e) => handleContextMenu(e, session)}
                    className={`grid grid-cols-[30px_50px_64px_1fr_56px_64px_64px_80px] gap-0 px-4 text-[11px] cursor-pointer border-b border-[var(--color-border-subtle)] transition-all duration-100 items-center ${
                      isSelected
                        ? "bg-[var(--color-accent-muted)] border-l-[3px] border-l-[var(--color-accent)]"
                        : "hover:bg-[var(--color-bg-tertiary)] border-l-[3px] border-l-transparent"
                    }`}
                    style={{ height: ROW_HEIGHT }}
                  >
                    <span
                      className="flex items-center justify-center cursor-pointer"
                      onClick={(e) => { e.stopPropagation(); toggleBookmark(sid); }}
                      title={isBookmarked ? "移除书签" : "添加书签"}
                    >
                      {isBookmarked ? (
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="var(--color-warning)" stroke="var(--color-warning)" strokeWidth="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" /></svg>
                      ) : (
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-muted)" strokeWidth="2" className="opacity-40 hover:opacity-100"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" /></svg>
                      )}
                    </span>
                    <span className="text-[var(--color-text-muted)] whitespace-nowrap flex items-center gap-[3px]" title={streamId != null ? `Stream #${streamId}` : undefined}>
                      {isH2 && <span className="text-[8px] font-bold text-[var(--color-accent)] leading-none px-0.5 rounded bg-[var(--color-accent-muted)]">h2</span>}
                      {isWs && <span className="text-[8px] font-bold text-[var(--color-success)] leading-none px-0.5 rounded bg-[var(--color-success-muted)]">WS</span>}
                      {isDecrypted && !isH2 && !isWs && <span className="text-[9px] leading-none" style={{ fontFamily: "'Segoe UI Emoji', 'Apple Color Emoji', sans-serif" }}>🔓</span>}
                      {!isDecrypted && isHttps && method === "CONNECT" && !isH2 && !isWs && <span className="text-[9px] leading-none" style={{ fontFamily: "'Segoe UI Emoji', 'Apple Color Emoji', sans-serif" }}>🔒</span>}
                      <span className="font-mono text-[var(--color-text-secondary)]">{index + 1}</span>
                    </span>
                    <span
                      style={{ color: methodColor(method) }}
                      className="font-mono font-bold whitespace-nowrap text-[11px]"
                    >
                      {method}
                    </span>
                    <span className="truncate text-[var(--color-text-primary)]" title={url}>
                      {url}
                    </span>
                    <span
                      style={{ color: statusCodeColor(statusCode ?? null) }}
                      className="font-mono whitespace-nowrap font-medium"
                    >
                      {statusCode ?? "—"}
                    </span>
                    <span className="text-[var(--color-text-muted)] whitespace-nowrap">
                      {formatSize(bodySize)}
                    </span>
                    <span className="text-[var(--color-text-muted)] whitespace-nowrap">
                      {formatDuration(duration ?? null)}
                    </span>
                    <span
                      className="text-[var(--color-text-muted)] truncate cursor-pointer hover:text-[var(--color-accent)] transition-colors"
                      title={process && processId ? `${process} (${processId})` : undefined}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (process) {
                          setFilter({ searchText: process });
                        }
                      }}
                    >
                      {process ? (processId ? `${process} (${processId})` : process) : "—"}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {filteredSessionList.length > pageSize && (
        <div className="flex items-center justify-between px-4 py-1.5 text-[11px] text-[var(--color-text-muted)] bg-[var(--color-bg-secondary)] border-t border-[var(--color-border)] shrink-0">
          <span>
            共 <span className="text-[var(--color-text-primary)] font-medium">{filteredSessionList.length}</span> 条 · 第 <span className="text-[var(--color-text-primary)] font-medium">{currentPage}</span>/{totalPages} 页
          </span>
          <div className="flex items-center gap-1">
            <button onClick={() => setPage(1)} disabled={currentPage <= 1} className="px-1.5 py-0.5 rounded-[var(--radius-sm)] bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-30 disabled:cursor-not-allowed transition-all text-[10px]">⏮</button>
            <button onClick={() => setPage(currentPage - 1)} disabled={currentPage <= 1} className="px-1.5 py-0.5 rounded-[var(--radius-sm)] bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-30 disabled:cursor-not-allowed transition-all text-[10px]">◀</button>
            <button onClick={() => setPage(currentPage + 1)} disabled={currentPage >= totalPages} className="px-1.5 py-0.5 rounded-[var(--radius-sm)] bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-30 disabled:cursor-not-allowed transition-all text-[10px]">▶</button>
            <button onClick={() => setPage(totalPages)} disabled={currentPage >= totalPages} className="px-1.5 py-0.5 rounded-[var(--radius-sm)] bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-30 disabled:cursor-not-allowed transition-all text-[10px]">⏭</button>
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
          onFilterByProcess={handleFilterByProcess}
          onBookmark={handleBookmark}
          onReplay={handleReplay}
          onOpenInBrowser={handleOpenInBrowser}
        />
      )}
    </div>
  );
}
