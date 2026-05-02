import { formatSize } from "../../lib/utils";

export function BodyView({ body, bodySize, bodyTruncated, contentType }: {
  body: number[] | null;
  bodySize: number;
  bodyTruncated: boolean;
  contentType: string | null;
}) {
  if (!body || bodySize === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic">无请求体</div>;
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
          <span className="text-xs text-[var(--color-warning)]">（已截断）</span>
        )}
        {contentType && (
          <span className="text-xs text-[var(--color-text-secondary)]">{contentType}</span>
        )}
      </div>
      {isText ? (
        <pre className="text-xs text-[var(--color-text-primary)] bg-[var(--color-bg-secondary)] p-3 rounded overflow-x-auto max-h-80 whitespace-pre-wrap break-all">
          {displayText}
        </pre>
      ) : (
        <div className="text-xs text-[var(--color-text-secondary)] bg-[var(--color-bg-secondary)] p-3 rounded">
          二进制数据（{formatSize(bodySize)}）
        </div>
      )}
    </div>
  );
}
