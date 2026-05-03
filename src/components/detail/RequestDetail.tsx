import { useState, useMemo } from "react";
import { useStore, type StoreState } from "../../store";
import { formatDuration } from "../../lib/utils";
import { exportHar, replayRequest } from "../../lib/tauri-bindings";
import { HeadersTable } from "./HeadersTable";
import { BodyView } from "./BodyView";
import { CookiesView } from "./CookiesView";
import { TimingView } from "./TimingView";
import { TlsInfoBadge } from "./TlsInfoBadge";
import { WebSocketView } from "../traffic/WebSocketView";
import { GraphQLView, isGraphQLRequest } from "./GraphQLView";
import { GrpcView, isGrpcWebRequest } from "./GrpcView";

type DetailTab = "headers" | "body" | "cookies" | "timing" | "websocket" | "graphql" | "grpc";

export function RequestDetail() {
  const sessions = useStore((s: StoreState) => s.sessions);
  const selectedId = useStore((s: StoreState) => s.selectedId);
  const [activeTab, setActiveTab] = useState<DetailTab>("headers");
  const [replayStatus, setReplayStatus] = useState<string | null>(null);

  const session = selectedId !== null ? sessions.get(selectedId) : undefined;

  const isWebSocket = session?.request.protocol === "WebSocket";
  const isGraphQL = session ? isGraphQLRequest(session) : false;
  const isGrpc = session ? isGrpcWebRequest(session) : false;

  const tabs = useMemo(() => {
    const base: { key: DetailTab; label: string }[] = [
      { key: "headers", label: "请求头" },
      { key: "body", label: "请求体" },
      { key: "cookies", label: "Cookie" },
      { key: "timing", label: "耗时" },
    ];
    if (isGraphQL) base.push({ key: "graphql", label: "GraphQL" });
    if (isGrpc) base.push({ key: "grpc", label: "gRPC" });
    if (isWebSocket) base.push({ key: "websocket", label: "WebSocket" });
    return base;
  }, [isWebSocket, isGraphQL, isGrpc]);

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full bg-[var(--color-bg-primary)] text-[var(--color-text-muted)]">
        <span className="text-3xl mb-3 opacity-30">📋</span>
        <p className="text-sm">选择一个请求以查看详情</p>
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
      <div className="px-5 py-3 border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)]">
        <div className="flex items-center gap-3 mb-1.5">
          <span
            className="font-mono font-bold text-sm px-2 py-0.5 rounded-[var(--radius-sm)]"
            style={{
              color: req.method === "GET" ? "var(--color-method-get)" : req.method === "POST" ? "var(--color-method-post)" : "var(--color-text-primary)",
              backgroundColor: req.method === "GET" ? "var(--color-success-muted)" : req.method === "POST" ? "var(--color-warning-muted)" : "var(--color-bg-tertiary)",
            }}
          >
            {req.method}
          </span>
          <span className="text-[12px] text-[var(--color-text-primary)] break-all flex-1 leading-snug">
            {req.url}
          </span>
          <div className="flex items-center gap-1.5 shrink-0">
            <button
              onClick={handleReplay}
              className="px-2.5 py-1 text-[11px] font-medium bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)] rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] transition-all cursor-pointer"
              title="重放此请求"
            >
              ↻ 重放
            </button>
            <button
              onClick={handleExportHar}
              className="px-2.5 py-1 text-[11px] font-medium bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)] rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] transition-all cursor-pointer"
              title="导出为 HAR 文件"
            >
              ↓ HAR
            </button>
          </div>
        </div>
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-[var(--color-text-muted)]">
          {resp?.status_code && (
            <span style={{ color: resp.status_code >= 400 ? "var(--color-error)" : resp.status_code >= 300 ? "var(--color-warning)" : "var(--color-success)" }} className="font-medium">
              {resp.status_code} {resp.status_text ?? ""}
            </span>
          )}
          <span>{req.protocol}</span>
          <span>{req.scheme}</span>
          {resp?.duration_us && <span>{formatDuration(resp.duration_us)}</span>}
          {req.process_name && <span>{req.process_name} ({req.process_id})</span>}
        </div>
        {req.raw_tls_info && (
          <div className="mt-1.5">
            <TlsInfoBadge tlsInfo={req.raw_tls_info} />
          </div>
        )}
        {replayStatus && (
          <div className="mt-1.5 text-[11px] text-[var(--color-accent)] bg-[var(--color-accent-muted)] px-2 py-1 rounded-[var(--radius-sm)]">{replayStatus}</div>
        )}
      </div>

      <div className="flex border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)] px-2">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`px-3 py-2 text-[11px] font-medium transition-all cursor-pointer border-b-2 ${
              activeTab === tab.key
                ? "text-[var(--color-accent)] border-[var(--color-accent)]"
                : "text-[var(--color-text-muted)] border-transparent hover:text-[var(--color-text-secondary)]"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto px-5 py-4">
        {activeTab === "headers" && (
          <div className="space-y-5">
            <div>
              <h4 className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">请求头</h4>
              <HeadersTable headers={req.headers} />
            </div>
            {resp && (
              <div>
                <h4 className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">响应头</h4>
                <HeadersTable headers={resp.headers} />
              </div>
            )}
          </div>
        )}

        {activeTab === "body" && (
          <div className="space-y-5">
            <div>
              <h4 className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">请求体</h4>
              <BodyView body={req.body} bodySize={req.body_size} bodyTruncated={req.body_truncated} contentType={req.content_type} />
            </div>
            {resp && (
              <div>
                <h4 className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">响应体</h4>
                <BodyView body={resp.body} bodySize={resp.body_size} bodyTruncated={resp.body_truncated} contentType={resp.content_type} />
              </div>
            )}
          </div>
        )}

        {activeTab === "cookies" && (
          <div className="space-y-5">
            <div>
              <h4 className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">请求 Cookie</h4>
              <CookiesView headers={req.headers} />
            </div>
            {resp && (
              <div>
                <h4 className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">响应 Cookie</h4>
                <CookiesView headers={resp.headers} />
              </div>
            )}
          </div>
        )}

        {activeTab === "timing" && <TimingView req={req} resp={resp} />}
        {activeTab === "websocket" && isWebSocket && <WebSocketView frames={[]} />}
        {activeTab === "graphql" && isGraphQL && session && <GraphQLView session={session} />}
        {activeTab === "grpc" && isGrpc && session && <GrpcView session={session} />}
      </div>
    </div>
  );
}
