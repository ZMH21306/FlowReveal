import { useState } from "react";
import { useStore } from "../../store";
import { formatDuration, formatSize } from "../../lib/utils";
import { exportHar, replayRequest } from "../../lib/tauri-bindings";

type DetailTab = "headers" | "body" | "cookies" | "timing";

function HeadersTable({ headers }: { headers: [string, string][] }) {
  if (headers.length === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic">No headers</div>;
  }
  return (
    <table className="w-full text-xs">
      <tbody>
        {headers.map(([key, value], i) => (
          <tr key={i} className="border-b border-[var(--color-border)]">
            <td className="py-1 pr-4 text-[var(--color-accent)] font-mono whitespace-nowrap align-top">
              {key}
            </td>
            <td className="py-1 text-[var(--color-text-primary)] break-all">
              {value}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function BodyView({ body, bodySize, bodyTruncated, contentType }: {
  body: number[] | null;
  bodySize: number;
  bodyTruncated: boolean;
  contentType: string | null;
}) {
  if (!body || bodySize === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic">No body</div>;
  }

  const text = new TextDecoder().decode(new Uint8Array(body));
  const isJson = contentType?.includes("application/json");
  const isXml = contentType?.includes("xml");
  const isText = contentType?.startsWith("text/") || isJson || isXml;

  let displayText = text;
  if (isJson) {
    try {
      displayText = JSON.stringify(JSON.parse(text), null, 2);
    } catch {
      // not valid JSON, show raw
    }
  }

  return (
    <div>
      <div className="flex items-center gap-2 mb-1">
        <span className="text-xs text-[var(--color-text-secondary)]">
          {formatSize(bodySize)}
        </span>
        {bodyTruncated && (
          <span className="text-xs text-[var(--color-warning)]">
            (truncated)
          </span>
        )}
        {contentType && (
          <span className="text-xs text-[var(--color-text-secondary)]">
            {contentType}
          </span>
        )}
      </div>
      {isText ? (
        <pre className="text-xs text-[var(--color-text-primary)] bg-[var(--color-bg-secondary)] p-3 rounded overflow-x-auto max-h-80 whitespace-pre-wrap break-all">
          {displayText}
        </pre>
      ) : (
        <div className="text-xs text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] p-3 rounded">
          Binary data ({formatSize(bodySize)})
        </div>
      )}
    </div>
  );
}

function CookiesView({ headers }: { headers: [string, String][] }) {
  const cookies = headers
    .filter(([k]) => k.toLowerCase() === "cookie")
    .flatMap(([, v]) =>
      v.split(";").map((pair) => {
        const [name, ...rest] = pair.trim().split("=");
        return { name: name?.trim() || "", value: rest.join("=").trim() };
      })
    );

  const setCookies = headers
    .filter(([k]) => k.toLowerCase() === "set-cookie")
    .map(([, v]) => {
      const [nameValue, ...attrs] = v.split("; ");
      const [name, ...rest] = nameValue.split("=");
      return {
        name: name?.trim() || "",
        value: rest.join("=").trim(),
        attrs: attrs.join("; "),
      };
    });

  if (cookies.length === 0 && setCookies.length === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic">No cookies</div>;
  }

  return (
    <div className="space-y-3">
      {cookies.length > 0 && (
        <div>
          <div className="text-xs font-semibold text-[var(--color-text-secondary)] mb-1">Request Cookies</div>
          <table className="w-full text-xs">
            <tbody>
              {cookies.map((c, i) => (
                <tr key={i} className="border-b border-[var(--color-border)]">
                  <td className="py-1 pr-4 text-[var(--color-accent)] font-mono">{c.name}</td>
                  <td className="py-1 text-[var(--color-text-primary)] break-all">{c.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {setCookies.length > 0 && (
        <div>
          <div className="text-xs font-semibold text-[var(--color-text-secondary)] mb-1">Response Set-Cookie</div>
          <table className="w-full text-xs">
            <tbody>
              {setCookies.map((c, i) => (
                <tr key={i} className="border-b border-[var(--color-border)]">
                  <td className="py-1 pr-4 text-[var(--color-accent)] font-mono">{c.name}</td>
                  <td className="py-1 text-[var(--color-text-primary)] break-all">{c.value}</td>
                  <td className="py-1 pl-4 text-[var(--color-text-secondary)] text-[10px]">{c.attrs}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function TimingView({ req, resp }: { req: { timestamp: number }; resp: { timestamp: number; duration_us: number | null } | null }) {
  const reqTime = new Date(req.timestamp / 1000);
  const duration = resp?.duration_us;

  return (
    <div className="space-y-2 text-xs">
      <div className="flex justify-between">
        <span className="text-[var(--color-text-secondary)]">Request Started</span>
        <span className="text-[var(--color-text-primary)] font-mono">
          {reqTime.toLocaleTimeString("zh-CN", { hour12: false, fractionalSecondDigits: 3 })}
        </span>
      </div>
      {duration != null && (
        <div className="flex justify-between">
          <span className="text-[var(--color-text-secondary)]">Total Duration</span>
          <span className="text-[var(--color-text-primary)] font-mono">{formatDuration(duration)}</span>
        </div>
      )}
      {duration != null && (
        <div className="mt-2">
          <div className="h-2 bg-[var(--color-bg-tertiary)] rounded overflow-hidden">
            <div
              className="h-full bg-[var(--color-accent)] rounded"
              style={{ width: `${Math.min(100, (duration / 1_000_000) * 100)}%` }}
            />
          </div>
          <div className="flex justify-between mt-1 text-[10px] text-[var(--color-text-secondary)]">
            <span>0ms</span>
            <span>{(duration / 1_000_000).toFixed(2)}s</span>
          </div>
        </div>
      )}
    </div>
  );
}

function TlsInfoBadge({ tlsInfo }: { tlsInfo: { version: string; cipher_suite: string; server_name: string | null } }) {
  return (
    <div className="flex items-center gap-2 text-xs text-[var(--color-accent)] bg-[var(--color-bg-tertiary)] px-2 py-1 rounded">
      <span>🔓 {tlsInfo.version}</span>
      <span className="text-[var(--color-text-secondary)]">|</span>
      <span>{tlsInfo.cipher_suite}</span>
      {tlsInfo.server_name && (
        <>
          <span className="text-[var(--color-text-secondary)]">|</span>
          <span>SNI: {tlsInfo.server_name}</span>
        </>
      )}
    </div>
  );
}

export function RequestDetail() {
  const sessions = useStore((s) => s.sessions);
  const selectedId = useStore((s) => s.selectedId);
  const [activeTab, setActiveTab] = useState<DetailTab>("headers");
  const [replayStatus, setReplayStatus] = useState<string | null>(null);

  const session = selectedId !== null ? sessions.get(selectedId) : undefined;

  if (!session) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm bg-[var(--color-bg-primary)]">
        Select a request to view details
      </div>
    );
  }

  const req = session.request;
  const resp = session.response;

  const tabs: { key: DetailTab; label: string }[] = [
    { key: "headers", label: "Headers" },
    { key: "body", label: "Body" },
    { key: "cookies", label: "Cookies" },
    { key: "timing", label: "Timing" },
  ];

  const handleReplay = async () => {
    if (!session) return;
    setReplayStatus("Replaying...");
    try {
      const result = await replayRequest(session.id);
      setReplayStatus(result);
    } catch (e) {
      setReplayStatus(`Error: ${e}`);
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
              title="Replay this request"
            >
              ↻ Replay
            </button>
            <button
              onClick={handleExportHar}
              className="px-2 py-1 text-[10px] bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] rounded border border-[var(--color-border)] transition-colors"
              title="Export as HAR"
            >
              ↓ HAR
            </button>
          </div>
        </div>
        <div className="flex flex-wrap gap-3 text-xs text-[var(--color-text-secondary)]">
          {resp?.status_code && (
            <span style={{ color: resp.status_code >= 400 ? "var(--color-error)" : resp.status_code >= 300 ? "var(--color-warning)" : "var(--color-success)" }}>
              Status: {resp.status_code}
            </span>
          )}
          <span>Protocol: {req.protocol}</span>
          <span>Scheme: {req.scheme}</span>
          {resp?.duration_us && <span>Duration: {formatDuration(resp.duration_us)}</span>}
          {req.process_name && (
            <span>Process: {req.process_name} ({req.process_id})</span>
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
        {tabs.map((tab) => (
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
              <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">Request Headers</h4>
              <HeadersTable headers={req.headers} />
            </div>
            {resp && (
              <div>
                <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">Response Headers</h4>
                <HeadersTable headers={resp.headers} />
              </div>
            )}
          </div>
        )}

        {activeTab === "body" && (
          <div className="space-y-4">
            <div>
              <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">Request Body</h4>
              <BodyView
                body={req.body}
                bodySize={req.body_size}
                bodyTruncated={req.body_truncated}
                contentType={req.content_type}
              />
            </div>
            {resp && (
              <div>
                <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">Response Body</h4>
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
              <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">Request Cookies</h4>
              <CookiesView headers={req.headers} />
            </div>
            {resp && (
              <div>
                <h4 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">Response Cookies</h4>
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
