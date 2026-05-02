import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStore, type StoreState } from "../../store";

interface Vulnerability {
  session_id: number;
  vuln_type: string;
  severity: "Critical" | "High" | "Medium" | "Low" | "Info";
  title: string;
  description: string;
  evidence: string;
  remediation: string;
}

interface ScanResult {
  total_scanned: number;
  vulnerabilities: Vulnerability[];
  summary: string;
}

function severityBg(sev: string): string {
  switch (sev) {
    case "Critical": return "bg-red-600";
    case "High": return "bg-red-500";
    case "Medium": return "bg-amber-500";
    case "Low": return "bg-blue-500";
    default: return "bg-gray-500";
  }
}

export function VulnScanner({ onClose }: { onClose: () => void }) {
  const [result, setResult] = useState<ScanResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<string>("all");
  const selectRequest = useStore((s: StoreState) => s.selectRequest);

  const runScan = async () => {
    setLoading(true);
    try {
      const r = await invoke<ScanResult>("scan_vulnerabilities");
      setResult(r);
    } catch (e) {
      console.error("Vuln scan error:", e);
    } finally {
      setLoading(false);
    }
  };

  const filtered = result
    ? filter === "all"
      ? result.vulnerabilities
      : result.vulnerabilities.filter((v) => v.severity === filter)
    : [];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[750px] max-h-[85vh] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg shadow-2xl flex flex-col">
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--color-border)]">
          <div className="flex items-center gap-2">
            <span className="text-lg">🛡️</span>
            <span className="text-sm font-semibold text-[var(--color-text-primary)]">自动漏洞扫描</span>
          </div>
          <button onClick={onClose} className="text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] text-lg">✕</button>
        </div>

        <div className="flex items-center gap-2 px-4 py-2 border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)]">
          <button
            onClick={runScan}
            disabled={loading}
            className="px-3 py-1.5 text-xs font-medium rounded bg-[var(--color-error)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {loading ? "扫描中..." : "🔍 开始扫描"}
          </button>
          {result && (
            <>
              <span className="text-[11px] text-[var(--color-text-secondary)]">{result.summary}</span>
              <div className="flex-1" />
              <select
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                className="px-2 py-1 text-[10px] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)]"
              >
                <option value="all">全部 ({result.vulnerabilities.length})</option>
                <option value="Critical">严重</option>
                <option value="High">高危</option>
                <option value="Medium">中危</option>
                <option value="Low">低危</option>
              </select>
            </>
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {!result ? (
            <div className="flex flex-col items-center justify-center h-48 text-[var(--color-text-secondary)] text-sm">
              <span className="text-4xl mb-3">🛡️</span>
              <p>点击"开始扫描"检测当前流量中的安全漏洞</p>
              <div className="mt-3 text-[10px] text-left space-y-1">
                <p>检测项目：</p>
                <p>• SQL 注入 • XSS 跨站脚本 • CORS 配置错误</p>
                <p>• 敏感信息泄露 • 缺少安全头 • 不安全传输</p>
              </div>
            </div>
          ) : filtered.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-48 text-[var(--color-success)] text-sm">
              <span className="text-4xl mb-3">✅</span>
              <p>未发现安全漏洞</p>
            </div>
          ) : (
            <div className="space-y-2">
              {filtered.map((v, i) => (
                <div
                  key={i}
                  className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] cursor-pointer hover:bg-[var(--color-bg-tertiary)]"
                  onClick={() => selectRequest(v.session_id)}
                >
                  <div className="flex items-center gap-2">
                    <span className={`px-1.5 py-0.5 text-[9px] rounded font-bold text-white ${severityBg(v.severity)}`}>
                      {v.severity}
                    </span>
                    <span className="text-[10px] text-[var(--color-text-secondary)]">#{v.session_id}</span>
                    <span className="text-[10px] px-1 py-0.5 rounded bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)]">{v.vuln_type}</span>
                    <span className="text-xs font-medium text-[var(--color-text-primary)]">{v.title}</span>
                  </div>
                  <p className="text-[11px] text-[var(--color-text-secondary)] mt-1">{v.description}</p>
                  <p className="text-[10px] text-[var(--color-text-secondary)] mt-0.5 font-mono break-all">证据: {v.evidence}</p>
                  <p className="text-[10px] text-[var(--color-success)] mt-0.5">💡 {v.remediation}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
