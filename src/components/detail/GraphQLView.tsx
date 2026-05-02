import { useState, useMemo } from "react";
import type { HttpSession } from "../../types";

interface GraphQLOperation {
  type: "query" | "mutation" | "subscription";
  name: string | null;
  operationText: string;
  variables: Record<string, unknown> | null;
}

function parseGraphQL(body: string | null | undefined): GraphQLOperation | null {
  if (!body) return null;
  try {
    const parsed = JSON.parse(body);
    if (typeof parsed === "object" && parsed !== null) {
      const query = parsed.query || parsed.operationName;
      if (typeof query === "string" && (query.includes("{") || query.startsWith("query") || query.startsWith("mutation") || query.startsWith("subscription"))) {
        const typeMatch = query.match(/^(query|mutation|subscription)\s*(\w*)/i);
        const type = typeMatch ? typeMatch[1].toLowerCase() as "query" | "mutation" | "subscription" : "query";
        const name = typeMatch?.[2] || parsed.operationName || null;
        return {
          type,
          name,
          operationText: query,
          variables: parsed.variables || null,
        };
      }
    }
  } catch {
  }
  if (typeof body === "string" && (body.trim().startsWith("query") || body.trim().startsWith("mutation") || body.trim().startsWith("{"))) {
    const typeMatch = body.match(/^(query|mutation|subscription)\s*(\w*)/i);
    const type = typeMatch ? typeMatch[1].toLowerCase() as "query" | "mutation" | "subscription" : "query";
    const name = typeMatch?.[2] || null;
    return { type, name, operationText: body, variables: null };
  }
  return null;
}

function isGraphQLRequest(session: HttpSession): boolean {
  const url = session.request.url || "";
  const ct = session.request.content_type || "";
  if (url.includes("/graphql") || url.includes("/gql")) return true;
  if (ct.includes("graphql")) return true;
  if (session.request.body) {
    const bodyStr = new TextDecoder().decode(new Uint8Array(session.request.body));
    try {
      const parsed = JSON.parse(bodyStr);
      if (parsed && typeof parsed.query === "string") return true;
    } catch {}
  }
  return false;
}

function formatGraphQL(text: string): string {
  let indent = 0;
  let result = "";
  let inString = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '"' && (i === 0 || text[i - 1] !== "\\")) {
      inString = !inString;
    }
    if (inString) {
      result += ch;
      continue;
    }
    if (ch === "{") {
      result += " {\n";
      indent++;
      result += "  ".repeat(indent);
    } else if (ch === "}") {
      indent = Math.max(0, indent - 1);
      result += "\n" + "  ".repeat(indent) + "}";
    } else if (ch === ",") {
      result += ",\n" + "  ".repeat(indent);
    } else {
      result += ch;
    }
  }
  return result;
}

function opTypeColor(type: string): string {
  switch (type) {
    case "query": return "var(--color-success)";
    case "mutation": return "var(--color-warning)";
    case "subscription": return "var(--color-accent)";
    default: return "var(--color-text-secondary)";
  }
}

export function GraphQLView({ session }: { session: HttpSession }) {
  const [viewMode, setViewMode] = useState<"formatted" | "raw">("formatted");

  const gqlOp = useMemo(() => {
    if (!session.request.body) return null;
    const bodyStr = new TextDecoder().decode(new Uint8Array(session.request.body));
    return parseGraphQL(bodyStr);
  }, [session.request.body]);

  if (!gqlOp) {
    return (
      <div className="p-4 text-[var(--color-text-secondary)] text-sm">
        无法解析 GraphQL 请求
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-3 py-1.5 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
        <span className="px-1.5 py-0.5 text-[10px] rounded font-bold text-white" style={{ backgroundColor: opTypeColor(gqlOp.type) }}>
          {gqlOp.type.toUpperCase()}
        </span>
        {gqlOp.name && <span className="text-xs font-mono text-[var(--color-text-primary)]">{gqlOp.name}</span>}
        <div className="flex-1" />
        <button
          onClick={() => setViewMode(viewMode === "formatted" ? "raw" : "formatted")}
          className="px-2 py-0.5 text-[10px] rounded bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-hover)]"
        >
          {viewMode === "formatted" ? "原始" : "格式化"}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        <div>
          <h4 className="text-[11px] font-semibold text-[var(--color-text-secondary)] mb-1">Query</h4>
          <pre className="bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded p-2 text-[11px] font-mono whitespace-pre-wrap break-all text-[var(--color-text-primary)] max-h-[300px] overflow-y-auto">
            {viewMode === "formatted" ? formatGraphQL(gqlOp.operationText) : gqlOp.operationText}
          </pre>
        </div>

        {gqlOp.variables && (
          <div>
            <h4 className="text-[11px] font-semibold text-[var(--color-text-secondary)] mb-1">Variables</h4>
            <pre className="bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded p-2 text-[11px] font-mono whitespace-pre-wrap break-all text-[var(--color-text-primary)] max-h-[200px] overflow-y-auto">
              {JSON.stringify(gqlOp.variables, null, 2)}
            </pre>
          </div>
        )}

        {session.response?.body && (
          <div>
            <h4 className="text-[11px] font-semibold text-[var(--color-text-secondary)] mb-1">Response</h4>
            <pre className="bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded p-2 text-[11px] font-mono whitespace-pre-wrap break-all text-[var(--color-text-primary)] max-h-[300px] overflow-y-auto">
              {(() => {
                try {
                  const respStr = new TextDecoder().decode(new Uint8Array(session.response!.body!));
                  return JSON.stringify(JSON.parse(respStr), null, 2);
                } catch {
                  return "(binary data)";
                }
              })()}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

export { isGraphQLRequest, parseGraphQL };
