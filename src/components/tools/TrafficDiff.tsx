import { useState } from "react";
import { useStore, type StoreState } from "../../store";

interface DiffEntry {
  field: string;
  left: string;
  right: string;
  changed: boolean;
}

function diffSessions(leftId: number, rightId: number, sessions: Map<number, import("../../types").HttpSession>): DiffEntry[] {
  const left = sessions.get(leftId);
  const right = sessions.get(rightId);
  if (!left || !right) return [];

  const entries: DiffEntry[] = [];
  const lr = left.request;
  const rr = right.request;
  const lresp = left.response;
  const rresp = right.response;

  const addDiff = (field: string, l: string, r: string) => {
    entries.push({ field, left: l, right: r, changed: l !== r });
  };

  addDiff("Method", lr.method || "-", rr.method || "-");
  addDiff("URL", lr.url || "-", rr.url || "-");
  addDiff("Scheme", lr.scheme, rr.scheme);

  if (lresp && rresp) {
    addDiff("Status", String(lresp.status_code ?? "-"), String(rresp.status_code ?? "-"));
    addDiff("Duration", lresp.duration_us != null ? `${(lresp.duration_us / 1000).toFixed(1)}ms` : "-", rresp.duration_us != null ? `${(rresp.duration_us / 1000).toFixed(1)}ms` : "-");
    addDiff("Body Size", `${lresp.body_size} B`, `${rresp.body_size} B`);
  }

  const leftHeaders = new Map(lr.headers.map(([k, v]) => [k.toLowerCase(), v]));
  const rightHeaders = new Map(rr.headers.map(([k, v]) => [k.toLowerCase(), v]));
  const allHeaderKeys = new Set([...leftHeaders.keys(), ...rightHeaders.keys()]);
  for (const key of allHeaderKeys) {
    const lv = leftHeaders.get(key) ?? "(missing)";
    const rv = rightHeaders.get(key) ?? "(missing)";
    addDiff(`Header: ${key}`, lv, rv);
  }

  return entries;
}

export function TrafficDiff({ onClose }: { onClose: () => void }) {
  const sessions = useStore((s: StoreState) => s.sessions);
  const sessionList = useStore((s: StoreState) => s.filteredSessionList);

  const [leftId, setLeftId] = useState<number | null>(null);
  const [rightId, setRightId] = useState<number | null>(null);

  const diffResult = leftId !== null && rightId !== null ? diffSessions(leftId, rightId, sessions) : [];
  const changedCount = diffResult.filter((d) => d.changed).length;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[800px] max-h-[85vh] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg shadow-2xl flex flex-col">
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--color-border)]">
          <div className="flex items-center gap-2">
            <span className="text-lg">🔄</span>
            <span className="text-sm font-semibold text-[var(--color-text-primary)]">流量 Diff 对比</span>
          </div>
          <button onClick={onClose} className="text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] text-lg">✕</button>
        </div>

        <div className="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)]">
          <div className="flex items-center gap-1">
            <span className="text-[11px] text-[var(--color-text-secondary)]">左侧:</span>
            <select
              value={leftId ?? ""}
              onChange={(e) => setLeftId(Number(e.target.value) || null)}
              className="px-2 py-1 text-[11px] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)] max-w-[280px]"
            >
              <option value="">选择请求...</option>
              {sessionList.slice(0, 100).map((id) => {
                const s = sessions.get(id);
                return (
                  <option key={id} value={id}>
                    #{id} {s?.request.method} {s?.request.url?.slice(0, 50)}
                  </option>
                );
              })}
            </select>
          </div>
          <span className="text-[var(--color-text-secondary)]">vs</span>
          <div className="flex items-center gap-1">
            <span className="text-[11px] text-[var(--color-text-secondary)]">右侧:</span>
            <select
              value={rightId ?? ""}
              onChange={(e) => setRightId(Number(e.target.value) || null)}
              className="px-2 py-1 text-[11px] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)] max-w-[280px]"
            >
              <option value="">选择请求...</option>
              {sessionList.slice(0, 100).map((id) => {
                const s = sessions.get(id);
                return (
                  <option key={id} value={id}>
                    #{id} {s?.request.method} {s?.request.url?.slice(0, 50)}
                  </option>
                );
              })}
            </select>
          </div>
          {changedCount > 0 && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-warning)] text-white font-bold">{changedCount} 处差异</span>
          )}
        </div>

        <div className="flex-1 overflow-y-auto">
          {diffResult.length === 0 ? (
            <div className="flex items-center justify-center h-48 text-[var(--color-text-secondary)] text-sm">
              选择两个请求进行对比
            </div>
          ) : (
            <table className="w-full text-[11px]">
              <thead>
                <tr className="bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
                  <th className="text-left px-3 py-1.5 text-[var(--color-text-secondary)] font-medium w-[140px]">字段</th>
                  <th className="text-left px-3 py-1.5 text-[var(--color-text-secondary)] font-medium">左侧 #{leftId}</th>
                  <th className="text-left px-3 py-1.5 text-[var(--color-text-secondary)] font-medium">右侧 #{rightId}</th>
                </tr>
              </thead>
              <tbody>
                {diffResult.map((d, i) => (
                  <tr key={i} className={`border-b border-[var(--color-border)] ${d.changed ? "bg-[var(--color-warning)]/10" : ""}`}>
                    <td className="px-3 py-1 text-[var(--color-text-secondary)] font-medium">{d.field}</td>
                    <td className={`px-3 py-1 break-all ${d.changed ? "text-[var(--color-error)] font-medium" : "text-[var(--color-text-primary)]"}`}>{d.left}</td>
                    <td className={`px-3 py-1 break-all ${d.changed ? "text-[var(--color-success)] font-medium" : "text-[var(--color-text-primary)]"}`}>{d.right}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
