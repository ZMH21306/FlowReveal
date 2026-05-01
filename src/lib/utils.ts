export function formatTimestamp(us: number): string {
  const d = new Date(us / 1000);
  return d.toLocaleTimeString("zh-CN", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    fractionalSecondDigits: 3,
  });
}

export function formatDuration(us: number | null): string {
  if (us === null) return "-";
  if (us < 1000) return `${us}μs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(1)}ms`;
  return `${(us / 1_000_000).toFixed(2)}s`;
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

export function methodColor(method: string | null): string {
  switch (method?.toUpperCase()) {
    case "GET":
      return "var(--color-method-get)";
    case "POST":
      return "var(--color-method-post)";
    case "PUT":
      return "var(--color-method-put)";
    case "DELETE":
      return "var(--color-method-delete)";
    case "PATCH":
      return "var(--color-method-patch)";
    case "HEAD":
    case "OPTIONS":
      return "var(--color-method-head)";
    case "CONNECT":
      return "var(--color-method-connect)";
    default:
      return "var(--color-text-secondary)";
  }
}

export function statusCodeColor(code: number | null): string {
  if (code === null) return "var(--color-text-secondary)";
  if (code >= 200 && code < 300) return "var(--color-success)";
  if (code >= 300 && code < 400) return "var(--color-accent)";
  if (code >= 400 && code < 500) return "var(--color-warning)";
  return "var(--color-error)";
}
