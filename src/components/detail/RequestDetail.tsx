import { useStore } from "../../store";
import { formatDuration, formatSize } from "../../lib/utils";

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

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)] overflow-y-auto">
      <div className="px-4 py-3 border-b border-[var(--color-border)]">
        <div className="flex items-center gap-3 mb-2">
          <span
            className="font-mono font-semibold text-sm"
            style={{ color: req.method === "GET" ? "var(--color-method-get)" : req.method === "POST" ? "var(--color-method-post)" : "var(--color-text-primary)" }}
          >
            {req.method}
          </span>
          <span className="text-sm text-[var(--color-text-primary)] break-all">
            {req.url}
          </span>
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
      </div>

      <div className="px-4 py-3 border-b border-[var(--color-border)]">
        <h3 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">
          Request Headers
        </h3>
        <HeadersTable headers={req.headers} />
      </div>

      {req.body_size > 0 && (
        <div className="px-4 py-3 border-b border-[var(--color-border)]">
          <h3 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">
            Request Body
          </h3>
          <BodyView
            body={req.body}
            bodySize={req.body_size}
            bodyTruncated={req.body_truncated}
            contentType={req.content_type}
          />
        </div>
      )}

      {resp && (
        <>
          <div className="px-4 py-3 border-b border-[var(--color-border)]">
            <h3 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">
              Response Headers
            </h3>
            <HeadersTable headers={resp.headers} />
          </div>

          {resp.body_size > 0 && (
            <div className="px-4 py-3">
              <h3 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">
                Response Body
              </h3>
              <BodyView
                body={resp.body}
                bodySize={resp.body_size}
                bodyTruncated={resp.body_truncated}
                contentType={resp.content_type}
              />
            </div>
          )}
        </>
      )}
    </div>
  );
}
