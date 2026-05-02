import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStore, type StoreState } from "../../store";

interface Anomaly {
  session_id: number;
  anomaly_type: string;
  description: string;
  severity: string;
}

interface PerformanceInsight {
  session_id: number | null;
  category: string;
  description: string;
  recommendation: string;
}

interface SecurityFinding {
  session_id: number;
  finding_type: string;
  description: string;
  severity: string;
  evidence: string;
}

interface AiAnalysisResult {
  summary: string;
  anomalies: Anomaly[];
  performance_insights: PerformanceInsight[];
  security_findings: SecurityFinding[];
}

function severityColor(severity: string): string {
  switch (severity) {
    case "high": return "var(--color-error)";
    case "medium": return "var(--color-warning)";
    case "low": return "var(--color-accent)";
    default: return "var(--color-text-secondary)";
  }
}

function severityLabel(severity: string): string {
  switch (severity) {
    case "high": return "高";
    case "medium": return "中";
    case "low": return "低";
    case "info": return "信息";
    default: return severity;
  }
}

type AiTab = "overview" | "anomalies" | "performance" | "security" | "query";

export function AiAssistant({ onClose }: { onClose: () => void }) {
  const [result, setResult] = useState<AiAnalysisResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<AiTab>("overview");
  const [nlQuery, setNlQuery] = useState("");
  const [queryResults, setQueryResults] = useState<number[]>([]);
  const [queryLoading, setQueryLoading] = useState(false);
  const selectRequest = useStore((s: StoreState) => s.selectRequest);
  const sessions = useStore((s: StoreState) => s.sessions);

  const runAnalysis = async () => {
    setLoading(true);
    try {
      const r = await invoke<AiAnalysisResult>("ai_analyze_traffic");
      setResult(r);
      setActiveTab("overview");
    } catch (e) {
      console.error("AI analysis error:", e);
    } finally {
      setLoading(false);
    }
  };

  const runNlQuery = async () => {
    if (!nlQuery.trim()) return;
    setQueryLoading(true);
    try {
      const ids = await invoke<number[]>("ai_natural_language_query", { query: nlQuery });
      setQueryResults(ids);
      setActiveTab("query");
    } catch (e) {
      console.error("NL query error:", e);
    } finally {
      setQueryLoading(false);
    }
  };

  const tabs: { key: AiTab; label: string; count?: number }[] = [
    { key: "overview", label: "总览" },
    { key: "anomalies", label: "异常", count: result?.anomalies.length },
    { key: "performance", label: "性能", count: result?.performance_insights.length },
    { key: "security", label: "安全", count: result?.security_findings.length },
    { key: "query", label: "自然语言查询" },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[720px] max-h-[85vh] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg shadow-2xl flex flex-col">
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--color-border)]">
          <div className="flex items-center gap-2">
            <span className="text-lg">🤖</span>
            <span className="text-sm font-semibold text-[var(--color-text-primary)]">AI 智能分析助手</span>
          </div>
          <button onClick={onClose} className="text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] text-lg">✕</button>
        </div>

        <div className="flex items-center gap-2 px-4 py-2 border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)]">
          <button
            onClick={runAnalysis}
            disabled={loading}
            className="px-3 py-1.5 text-xs font-medium rounded bg-[var(--color-accent)] text-white hover:bg-[var(--color-accent-hover)] disabled:opacity-50"
          >
            {loading ? "分析中..." : "🔍 开始分析"}
          </button>
          <div className="flex-1 flex items-center gap-1">
            <input
              value={nlQuery}
              onChange={(e) => setNlQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runNlQuery()}
              placeholder="自然语言查询：如 '找出所有慢请求' '错误请求' '安全相关'"
              className="flex-1 px-2 py-1 text-xs bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded text-[var(--color-text-primary)] placeholder:text-[var(--color-text-secondary)]"
            />
            <button
              onClick={runNlQuery}
              disabled={queryLoading}
              className="px-2 py-1 text-xs rounded bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] hover:bg-[var(--color-bg-hover)] disabled:opacity-50"
            >
              {queryLoading ? "..." : "查询"}
            </button>
          </div>
        </div>

        <div className="flex border-b border-[var(--color-border)]">
          {tabs.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`px-3 py-1.5 text-[11px] border-b-2 transition-colors ${
                activeTab === tab.key
                  ? "border-[var(--color-accent)] text-[var(--color-accent)]"
                  : "border-transparent text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]"
              }`}
            >
              {tab.label}
              {tab.count != null && tab.count > 0 && (
                <span className="ml-1 px-1 py-0 text-[9px] rounded-full bg-[var(--color-accent)] text-white">{tab.count}</span>
              )}
            </button>
          ))}
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {!result && activeTab !== "query" ? (
            <div className="flex flex-col items-center justify-center h-48 text-[var(--color-text-secondary)] text-sm">
              <span className="text-4xl mb-3">🤖</span>
              <p>点击"开始分析"对当前流量进行智能分析</p>
              <p className="text-xs mt-1">或使用自然语言查询快速筛选</p>
            </div>
          ) : (
            <>
              {activeTab === "overview" && result && (
                <div className="space-y-4">
                  <div className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)]">
                    <p className="text-sm text-[var(--color-text-primary)]">{result.summary}</p>
                  </div>
                  <div className="grid grid-cols-3 gap-3">
                    <div className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] text-center">
                      <div className="text-2xl font-bold" style={{ color: result.anomalies.length > 0 ? "var(--color-warning)" : "var(--color-success)" }}>{result.anomalies.length}</div>
                      <div className="text-[10px] text-[var(--color-text-secondary)]">异常</div>
                    </div>
                    <div className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] text-center">
                      <div className="text-2xl font-bold" style={{ color: result.performance_insights.length > 0 ? "var(--color-accent)" : "var(--color-success)" }}>{result.performance_insights.length}</div>
                      <div className="text-[10px] text-[var(--color-text-secondary)]">性能洞察</div>
                    </div>
                    <div className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] text-center">
                      <div className="text-2xl font-bold" style={{ color: result.security_findings.length > 0 ? "var(--color-error)" : "var(--color-success)" }}>{result.security_findings.length}</div>
                      <div className="text-[10px] text-[var(--color-text-secondary)]">安全发现</div>
                    </div>
                  </div>
                </div>
              )}

              {activeTab === "anomalies" && result && (
                <div className="space-y-2">
                  {result.anomalies.length === 0 ? (
                    <p className="text-center text-[var(--color-text-secondary)] text-sm py-8">未发现异常 ✅</p>
                  ) : result.anomalies.map((a, i) => (
                    <div key={i} className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] cursor-pointer hover:bg-[var(--color-bg-tertiary)]" onClick={() => selectRequest(a.session_id)}>
                      <div className="flex items-center gap-2">
                        <span className="px-1.5 py-0.5 text-[10px] rounded font-bold text-white" style={{ backgroundColor: severityColor(a.severity) }}>{severityLabel(a.severity)}</span>
                        <span className="text-[10px] text-[var(--color-text-secondary)]">#{a.session_id}</span>
                        <span className="text-[10px] text-[var(--color-text-secondary)]">{a.anomaly_type}</span>
                      </div>
                      <p className="text-xs text-[var(--color-text-primary)] mt-1">{a.description}</p>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === "performance" && result && (
                <div className="space-y-2">
                  {result.performance_insights.length === 0 ? (
                    <p className="text-center text-[var(--color-text-secondary)] text-sm py-8">性能良好 ✅</p>
                  ) : result.performance_insights.map((p, i) => (
                    <div key={i} className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)]">
                      <div className="flex items-center gap-2">
                        <span className="px-1.5 py-0.5 text-[10px] rounded bg-[var(--color-accent)] text-white font-bold">{p.category}</span>
                      </div>
                      <p className="text-xs text-[var(--color-text-primary)] mt-1">{p.description}</p>
                      <p className="text-[11px] text-[var(--color-success)] mt-1">💡 {p.recommendation}</p>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === "security" && result && (
                <div className="space-y-2">
                  {result.security_findings.length === 0 ? (
                    <p className="text-center text-[var(--color-text-secondary)] text-sm py-8">未发现安全问题 ✅</p>
                  ) : result.security_findings.map((s, i) => (
                    <div key={i} className="p-3 bg-[var(--color-bg-secondary)] rounded-lg border border-[var(--color-border)] cursor-pointer hover:bg-[var(--color-bg-tertiary)]" onClick={() => selectRequest(s.session_id)}>
                      <div className="flex items-center gap-2">
                        <span className="px-1.5 py-0.5 text-[10px] rounded font-bold text-white" style={{ backgroundColor: severityColor(s.severity) }}>{severityLabel(s.severity)}</span>
                        <span className="text-[10px] text-[var(--color-text-secondary)]">#{s.session_id}</span>
                        <span className="text-[10px] text-[var(--color-text-secondary)]">{s.finding_type}</span>
                      </div>
                      <p className="text-xs text-[var(--color-text-primary)] mt-1">{s.description}</p>
                      <p className="text-[10px] text-[var(--color-text-secondary)] mt-0.5 font-mono break-all">{s.evidence}</p>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === "query" && (
                <div>
                  {queryResults.length === 0 ? (
                    <p className="text-center text-[var(--color-text-secondary)] text-sm py-8">输入自然语言查询来筛选流量</p>
                  ) : (
                    <div>
                      <p className="text-xs text-[var(--color-text-secondary)] mb-2">找到 {queryResults.length} 条匹配请求</p>
                      <div className="space-y-1 max-h-[400px] overflow-y-auto">
                        {queryResults.map((id) => {
                          const session = sessions.get(id);
                          if (!session) return null;
                          return (
                            <div key={id} className="flex items-center gap-2 px-2 py-1 text-[11px] bg-[var(--color-bg-secondary)] rounded cursor-pointer hover:bg-[var(--color-bg-tertiary)]" onClick={() => selectRequest(id)}>
                              <span className="font-mono text-[var(--color-text-secondary)]">#{id}</span>
                              <span className="font-mono font-semibold" style={{ color: session.request.method === "GET" ? "var(--color-success)" : "var(--color-warning)" }}>{session.request.method}</span>
                              <span className="truncate text-[var(--color-text-primary)]">{session.request.url}</span>
                              {session.response?.status_code && (
                                <span className="font-mono" style={{ color: session.response.status_code >= 400 ? "var(--color-error)" : "var(--color-success)" }}>{session.response.status_code}</span>
                              )}
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
