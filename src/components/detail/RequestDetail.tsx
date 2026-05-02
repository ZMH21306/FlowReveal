import { useState } from "react";
import { useStore } from "../../store";
import { formatDuration } from "../../lib/utils";
import { exportHar, replayRequest } from "../../lib/tauri-bindings";
import { HeadersTable } from "./HeadersTable";
import { BodyView } from "./BodyView";
import { CookiesView } from "./CookiesView";
import { TimingView } from "./TimingView";
import { TlsInfoBadge } from "./TlsInfoBadge";

type DetailTab = "headers" | "body" | "cookies" | "timing";

const TABS: { key: DetailTab; label: string }[] = [
  { key: "headers", label: "请求头" },
  { key: "body", label: "请求体" },
  { key: "cookies", label: "Cookie" },
  { key: "timing", label: "耗时" },
];

export function RequestDetail() {
  const sessions = useStore((s) => s.sessions);
  const selectedId = useStore((s) => s.selectedId);
  const [activeTab, setActiveTab] = useState<DetailTab>("headers");
  const [replayStatus, setReplayStatus] = useState<string | null>(null);

  const session = selectedId !== null ? sessions.get(selectedId) : undefined;

  if (!session) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm bg-[var(--color-bg-primary)]">
        选择一个请求以查看详情
      </div>
    );
  }

  const req = session.request;
  const resp = session.response;

  const handleReplay = async () => {
    if (!session) return;
    setReplayStatus("重放中...");
    try {
      const result = await replayRequest(session.id);
      setReplayStatus(result);
    } catch (e) {
      setReplayStatus(`错误: ${e}`);
    }
    setTimeout(() => setReplayStatus(null), 5000);
  };

  const handleExportHar = async () => {
    if (!session) return;
    try {
      const har = await exportHar([session.id]);
      const blob = new Blob([har], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `flowreveal-session-${session.id}.har`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Export HAR failed:", e);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)] overflow-hidden">
      <div className="px-4 py-3 border-b border-[var(--color-border)]">
        <div className="flex items-center gap-3 mb-2">
          <span
            className="font-mono font-semibold text-sm"
            style={{ color: req.method === "GET" ? "var(--color-method-get)" : req.method === "POST" ? "var(--color-method-post)" : "var(--color-text-primary)" }}
          >
            {req.method}
          </span>
          <span className="text-sm text-[var(--color-text-primary)] break-all flex-1">
            {req.url}
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={handleReplay}
              className="px-2 py-1 text-[10px] bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] rounded border border-[var(--color-border)] transition-colors"
              title="重放此请求"
            >
              ↻ 重放
            </button>
            <button
              onClick={handleExportHar}
              className="px-2 py-1 text-[10px] bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] rounded border border-[var(--color-border)] transition-colors"
              title="导出为 HAR 文件"
            >
              ↓ HAR
            </button>
          </div>
        </div>
        <div className="flex flex-wrap gap-3 text-xs text-[var(--color-text-secondary)]">
          {resp?.status_code && (
            <span style={{ color: resp.status_code >= 400 ? "var(--color-error)" : resp.status_code >= 300 ? "var(--color-warning)" : "var(--color-success)" }}>
              状态码: {resp.status_code}
            </span>
          )}
          <span>协议: {req.protocol}</span>
          <span>协议类型: {req.scheme}</span>
          {resp?.duration_us && <span>耗时: {formatDuration(resp.duration_us)}</span>}
          {req.process_name && (
            <span>进程: {req.process_name} ({req.process_id})</span>
          )}
        </div>
        {req.raw_tls_info && (
          <div className="mt-2">
            <TlsInfoBadge tlsInfo={req.raw_tls_info} />
          </div>
        )}
        {replayStatus && (
          <div className="mt-2 text-xs text-[var(--color-accent)]">{replayStatus}</div>
        )}
      </div>

      <div className="flex border-b border-[var(--color-border)]">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`px-4 py-1.5 text-xs transition-colors ${
              activeTab === tab.key
                ? "text-[var(--color-accent)] border-b-2 border-[var(--color-accent)]"
                : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3">
        {activeTab === "headers" && (
          <div className="space-y-4">
            <div>
              <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">请求头</h4>
              <HeadersTable headers={req.headers} />
            </div>
            {resp && (
              <div>
                <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">响应头</h4>
                <HeadersTable headers={resp.headers} />
              </div>
            )}
          </div>
        )}

        {activeTab === "body" && (
          <div className="space-y-4">
            <div>
              <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">请求体</h4>
              <BodyView
                body={req.body}
                bodySize={req.body_size}
                bodyTruncated={req.body_truncated}
                contentType={req.content_type}
              />
            </div>
            {resp && (
              <div>
                <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">响应体</h4>
                <BodyView
                  body={resp.body}
                  bodySize={resp.body_size}
                  bodyTruncated={resp.body_truncated}
                  contentType={resp.content_type}
                />
              </div>
            )}
          </div>
        )}

        {activeTab === "cookies" && (
          <div className="space-y-4">
            <div>
              <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">请求 Cookie</h4>
              <CookiesView headers={req.headers} />
            </div>
            {resp && (
              <div>
                <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">响应 Cookie</h4>
                <CookiesView headers={resp.headers} />
              </div>
            )}
          </div>
        )}

        {activeTab === "timing" && (
          <TimingView req={req} resp={resp} />
        )}
      </div>
    </div>
  );
}
