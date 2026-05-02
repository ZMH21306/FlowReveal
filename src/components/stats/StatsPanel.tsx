import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend,
} from "recharts";

interface TrafficStats {
  total_requests: number;
  total_bytes_sent: number;
  total_bytes_received: number;
  avg_duration_us: number;
  error_rate: number;
  by_domain: { domain: string; request_count: number; total_bytes: number; avg_duration_us: number; error_rate: number }[];
  by_content_type: { content_type: string; count: number; total_bytes: number; percentage: number }[];
  by_status_code: Record<string, number>;
  by_method: Record<string, number>;
  duration_distribution: { range: string; count: number; percentage: number }[];
  size_distribution: { range: string; count: number; percentage: number }[];
}

const COLORS = ["#6366f1", "#22c55e", "#f59e0b", "#ef4444", "#3b82f6", "#a855f7", "#ec4899", "#14b8a6", "#f97316", "#64748b"];

function formatBytes(b: number): string {
  if (b < 1024) return `${b}B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)}KB`;
  return `${(b / 1048576).toFixed(1)}MB`;
}

function formatDuration(us: number): string {
  const ms = us / 1000;
  if (ms < 1) return `${us}μs`;
  if (ms < 1000) return `${ms.toFixed(0)}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

interface StatsPanelProps {
  onClose: () => void;
}

export function StatsPanel({ onClose }: StatsPanelProps) {
  const [stats, setStats] = useState<TrafficStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<"overview" | "domain" | "content" | "duration" | "size">("overview");

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const s = await invoke<TrafficStats>("get_traffic_stats");
      setStats(s);
    } catch (e) {
      console.error("Failed to load stats:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  if (loading && !stats) {
    return (
      <div className="stats-panel">
        <div className="stats-header"><h2>📊 性能统计</h2><button className="btn-icon" onClick={onClose}>✕</button></div>
        <div className="loading">加载中...</div>
      </div>
    );
  }

  if (!stats) return null;

  const methodData = Object.entries(stats.by_method).map(([name, value]) => ({ name, value }));
  const statusCodeData = Object.entries(stats.by_status_code).map(([name, value]) => ({ name: `${name}`, value }));

  return (
    <div className="stats-panel">
      <div className="stats-header">
        <h2>📊 性能统计</h2>
        <div className="flex items-center gap-2">
          <button className="btn-small" onClick={refresh}>🔄 刷新</button>
          <button className="btn-icon" onClick={onClose}>✕</button>
        </div>
      </div>

      <div className="stats-tabs">
        {(["overview", "domain", "content", "duration", "size"] as const).map((tab) => (
          <button key={tab} className={`tab-btn ${activeTab === tab ? "active" : ""}`} onClick={() => setActiveTab(tab)}>
            {tab === "overview" ? "总览" : tab === "domain" ? "域名" : tab === "content" ? "类型" : tab === "duration" ? "耗时" : "大小"}
          </button>
        ))}
      </div>

      <div className="stats-content">
        {activeTab === "overview" && (
          <>
            <div className="overview-cards">
              <div className="stat-card"><div className="stat-value">{stats.total_requests}</div><div className="stat-label">总请求数</div></div>
              <div className="stat-card"><div className="stat-value">{formatBytes(stats.total_bytes_sent + stats.total_bytes_received)}</div><div className="stat-label">总数据量</div></div>
              <div className="stat-card"><div className="stat-value">{formatDuration(stats.avg_duration_us)}</div><div className="stat-label">平均耗时</div></div>
              <div className="stat-card"><div className="stat-value" style={{ color: stats.error_rate > 5 ? "var(--color-error)" : "var(--color-success)" }}>{stats.error_rate.toFixed(1)}%</div><div className="stat-label">错误率</div></div>
            </div>
            <div className="chart-section">
              <h4>请求方法分布</h4>
              <ResponsiveContainer width="100%" height={200}>
                <PieChart><Pie data={methodData} dataKey="value" nameKey="name" cx="50%" cy="50%" outerRadius={70} label={({ name, percent }: any) => `${name} ${((percent || 0) * 100).toFixed(0)}%`}>
                  {methodData.map((_, i) => <Cell key={i} fill={COLORS[i % COLORS.length]} />)}
                </Pie><Tooltip /></PieChart>
              </ResponsiveContainer>
            </div>
            {statusCodeData.length > 0 && (
              <div className="chart-section">
                <h4>状态码分布</h4>
                <ResponsiveContainer width="100%" height={200}>
                  <BarChart data={statusCodeData}><XAxis dataKey="name" tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><YAxis tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><Tooltip /><Bar dataKey="value" fill="#6366f1" radius={[4, 4, 0, 0]} /></BarChart>
                </ResponsiveContainer>
              </div>
            )}
          </>
        )}

        {activeTab === "domain" && (
          <div className="chart-section">
            <h4>域名统计 Top 20</h4>
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={stats.by_domain} layout="vertical"><XAxis type="number" tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><YAxis dataKey="domain" type="category" width={140} tick={{ fontSize: 10, fill: "var(--color-text-primary)" }} /><Tooltip /><Bar dataKey="request_count" fill="#6366f1" name="请求数" radius={[0, 4, 4, 0]} /></BarChart>
            </ResponsiveContainer>
          </div>
        )}

        {activeTab === "content" && (
          <div className="chart-section">
            <h4>内容类型分布</h4>
            <ResponsiveContainer width="100%" height={300}>
              <PieChart><Pie data={stats.by_content_type} dataKey="count" nameKey="content_type" cx="50%" cy="50%" outerRadius={100} label={({ name, percent }: any) => `${name} ${((percent || 0) * 100).toFixed(0)}%`}>
                {stats.by_content_type.map((_, i) => <Cell key={i} fill={COLORS[i % COLORS.length]} />)}
              </Pie><Tooltip /><Legend /></PieChart>
            </ResponsiveContainer>
          </div>
        )}

        {activeTab === "duration" && (
          <div className="chart-section">
            <h4>耗时分布</h4>
            <ResponsiveContainer width="100%" height={250}>
              <BarChart data={stats.duration_distribution}><XAxis dataKey="range" tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><YAxis tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><Tooltip /><Bar dataKey="count" fill="#22c55e" name="请求数" radius={[4, 4, 0, 0]} /></BarChart>
            </ResponsiveContainer>
          </div>
        )}

        {activeTab === "size" && (
          <div className="chart-section">
            <h4>大小分布</h4>
            <ResponsiveContainer width="100%" height={250}>
              <BarChart data={stats.size_distribution}><XAxis dataKey="range" tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><YAxis tick={{ fontSize: 11, fill: "var(--color-text-secondary)" }} /><Tooltip /><Bar dataKey="count" fill="#f59e0b" name="请求数" radius={[4, 4, 0, 0]} /></BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>
    </div>
  );
}
