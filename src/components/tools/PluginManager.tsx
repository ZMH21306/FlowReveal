import { useState, useCallback } from "react";

interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  enabled: boolean;
  hooks: string[];
}

const STORAGE_KEY = "flowreveal-plugins";

function getStoredPlugins(): PluginManifest[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : getDefaultPlugins();
  } catch {
    return getDefaultPlugins();
  }
}

function getDefaultPlugins(): PluginManifest[] {
  return [
    {
      id: "cors-helper",
      name: "CORS 助手",
      version: "1.0.0",
      description: "自动为响应添加 CORS 头，方便前端开发调试",
      author: "FlowReveal",
      enabled: false,
      hooks: ["onResponse"],
    },
    {
      id: "api-timer",
      name: "API 计时器",
      version: "1.0.0",
      description: "高亮显示超过阈值的慢请求",
      author: "FlowReveal",
      enabled: false,
      hooks: ["onRequest"],
    },
    {
      id: "json-formatter",
      name: "JSON 格式化",
      version: "1.0.0",
      description: "自动格式化 JSON 响应内容",
      author: "FlowReveal",
      enabled: true,
      hooks: ["onResponse"],
    },
    {
      id: "cache-blocker",
      name: "缓存阻止",
      version: "1.0.0",
      description: "自动添加禁用缓存的请求头",
      author: "FlowReveal",
      enabled: false,
      hooks: ["onRequest"],
    },
  ];
}

function savePlugins(plugins: PluginManifest[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(plugins));
}

export function PluginManager({ onClose }: { onClose: () => void }) {
  const [plugins, setPlugins] = useState<PluginManifest[]>(getStoredPlugins);
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null);

  const togglePlugin = useCallback((id: string) => {
    setPlugins((prev) => {
      const updated = prev.map((p) =>
        p.id === id ? { ...p, enabled: !p.enabled } : p
      );
      savePlugins(updated);
      return updated;
    });
  }, []);

  const removePlugin = useCallback((id: string) => {
    setPlugins((prev) => {
      const updated = prev.filter((p) => p.id !== id);
      savePlugins(updated);
      return updated;
    });
    if (selectedPlugin === id) setSelectedPlugin(null);
  }, [selectedPlugin]);

  const addPlugin = useCallback(() => {
    const id = `custom-${Date.now()}`;
    const newPlugin: PluginManifest = {
      id,
      name: "自定义插件",
      version: "0.1.0",
      description: "新创建的自定义插件",
      author: "用户",
      enabled: false,
      hooks: ["onRequest", "onResponse"],
    };
    setPlugins((prev) => {
      const updated = [...prev, newPlugin];
      savePlugins(updated);
      return updated;
    });
    setSelectedPlugin(id);
  }, []);

  const selected = plugins.find((p) => p.id === selectedPlugin);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[650px] max-h-[80vh] bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg shadow-2xl flex flex-col">
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--color-border)]">
          <div className="flex items-center gap-2">
            <span className="text-lg">🧩</span>
            <span className="text-sm font-semibold text-[var(--color-text-primary)]">插件管理</span>
            <span className="text-[10px] text-[var(--color-text-secondary)]">{plugins.filter((p) => p.enabled).length}/{plugins.length} 已启用</span>
          </div>
          <button onClick={onClose} className="text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] text-lg">✕</button>
        </div>

        <div className="flex items-center gap-2 px-4 py-2 border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)]">
          <button
            onClick={addPlugin}
            className="px-3 py-1 text-xs rounded bg-[var(--color-accent)] text-white hover:bg-[var(--color-accent-hover)]"
          >
            + 新建插件
          </button>
          <span className="text-[10px] text-[var(--color-text-secondary)]">插件可通过 Hooks 拦截和修改请求/响应</span>
        </div>

        <div className="flex flex-1 overflow-hidden">
          <div className="w-[220px] shrink-0 overflow-y-auto border-r border-[var(--color-border)]">
            {plugins.map((plugin) => (
              <div
                key={plugin.id}
                onClick={() => setSelectedPlugin(plugin.id)}
                className={`flex items-center gap-2 px-3 py-2 cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] ${
                  selectedPlugin === plugin.id ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]" : ""
                }`}
              >
                <div
                  className={`w-2 h-2 rounded-full shrink-0 ${plugin.enabled ? "bg-[var(--color-success)]" : "bg-[var(--color-text-secondary)]"}`}
                />
                <div className="flex-1 min-w-0">
                  <div className="text-[11px] text-[var(--color-text-primary)] truncate">{plugin.name}</div>
                  <div className="text-[9px] text-[var(--color-text-secondary)]">v{plugin.version}</div>
                </div>
              </div>
            ))}
          </div>

          <div className="flex-1 overflow-y-auto p-4">
            {selected ? (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-semibold text-[var(--color-text-primary)]">{selected.name}</h3>
                  <button
                    onClick={() => togglePlugin(selected.id)}
                    className={`px-2 py-1 text-[10px] rounded font-medium ${
                      selected.enabled
                        ? "bg-[var(--color-success)] text-white"
                        : "bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)]"
                    }`}
                  >
                    {selected.enabled ? "已启用" : "已禁用"}
                  </button>
                </div>
                <p className="text-xs text-[var(--color-text-secondary)]">{selected.description}</p>
                <div className="grid grid-cols-2 gap-2 text-[11px]">
                  <div>
                    <span className="text-[var(--color-text-secondary)]">版本:</span>{" "}
                    <span className="text-[var(--color-text-primary)]">{selected.version}</span>
                  </div>
                  <div>
                    <span className="text-[var(--color-text-secondary)]">作者:</span>{" "}
                    <span className="text-[var(--color-text-primary)]">{selected.author}</span>
                  </div>
                </div>
                <div>
                  <span className="text-[11px] text-[var(--color-text-secondary)]">Hooks:</span>
                  <div className="flex gap-1 mt-1">
                    {selected.hooks.map((hook) => (
                      <span key={hook} className="px-1.5 py-0.5 text-[9px] rounded bg-[var(--color-bg-tertiary)] text-[var(--color-accent)] font-mono">
                        {hook}
                      </span>
                    ))}
                  </div>
                </div>
                <div className="pt-2">
                  <button
                    onClick={() => removePlugin(selected.id)}
                    className="px-2 py-1 text-[10px] rounded bg-[var(--color-error)] text-white hover:opacity-90"
                  >
                    删除插件
                  </button>
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-sm">
                选择一个插件查看详情
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
