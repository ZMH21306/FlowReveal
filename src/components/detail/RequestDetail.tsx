import { useStore } from "../../store";

export function RequestDetail() {
  const requests = useStore((s) => s.requests);
  const selectedId = useStore((s) => s.selectedId);

  const req = requests.find((r) => r.id === selectedId);

  if (!req) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm bg-[var(--color-bg-primary)]">
        Select a request to view details
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)] overflow-y-auto">
      <div className="px-4 py-3 border-b border-[var(--color-border)]">
        <div className="flex items-center gap-3 mb-2">
          <span className="font-mono font-semibold text-sm" style={{ color: req.method === "GET" ? "var(--color-method-get)" : "var(--color-method-post)" }}>
            {req.method}
          </span>
          <span className="text-sm text-[var(--color-text-primary)] break-all">
            {req.url}
          </span>
        </div>
        <div className="flex gap-4 text-xs text-[var(--color-text-secondary)]">
          {req.status_code && <span>Status: {req.status_code}</span>}
          {req.protocol && <span>Protocol: {req.protocol}</span>}
          {req.scheme && <span>Scheme: {req.scheme}</span>}
          {req.process_name && <span>Process: {req.process_name} ({req.process_id})</span>}
        </div>
      </div>

      <div className="px-4 py-3">
        <h3 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">
          Request Headers
        </h3>
        <table className="w-full text-xs">
          <tbody>
            {req.headers.map(([key, value], i) => (
              <tr key={i} className="border-b border-[var(--color-border)]">
                <td className="py-1 pr-4 text-[var(--color-accent)] font-mono whitespace-nowrap">
                  {key}
                </td>
                <td className="py-1 text-[var(--color-text-primary)] break-all">
                  {value}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {req.body && req.body_size > 0 && (
        <div className="px-4 py-3 border-t border-[var(--color-border)]">
          <h3 className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase mb-2">
            Request Body ({req.body_size} bytes)
          </h3>
          <pre className="text-xs text-[var(--color-text-primary)] bg-[var(--color-bg-secondary)] p-3 rounded overflow-x-auto max-h-60">
            {new TextDecoder().decode(new Uint8Array(req.body))}
          </pre>
        </div>
      )}
    </div>
  );
}
