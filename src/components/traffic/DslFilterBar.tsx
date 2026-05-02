import { useState, useRef, useCallback } from "react";
import { useStore } from "../../store";
import { invoke } from "@tauri-apps/api/core";

const DSL_FIELDS = [
  { key: "method", label: "method", desc: "HTTP方法" },
  { key: "url", label: "url", desc: "URL" },
  { key: "host", label: "host", desc: "主机名" },
  { key: "path", label: "path", desc: "路径" },
  { key: "status", label: "status", desc: "状态码" },
  { key: "proc", label: "proc", desc: "进程名" },
  { key: "body", label: "body", desc: "请求体" },
  { key: "content-type", label: "content-type", desc: "内容类型" },
  { key: "scheme", label: "scheme", desc: "协议" },
  { key: "duration", label: "duration", desc: "耗时(ms)" },
  { key: "size", label: "size", desc: "大小(bytes)" },
];

const QUICK_FILTERS = [
  { label: "仅XHR", dsl: "method:GET OR method:POST OR method:PUT OR method:DELETE OR method:PATCH" },
  { label: "仅错误", dsl: "status:400..599" },
  { label: "仅图片", dsl: "content-type:image" },
  { label: "慢请求", dsl: "duration:>1000" },
  { label: "大响应", dsl: "size:>10240" },
];

const SAVED_FILTERS_KEY = "flowreveal-saved-filters";

function loadSavedFilters(): { name: string; dsl: string }[] {
  try {
    const raw = localStorage.getItem(SAVED_FILTERS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch { return []; }
}

function saveFilters(filters: { name: string; dsl: string }[]) {
  localStorage.setItem(SAVED_FILTERS_KEY, JSON.stringify(filters));
}

export function DslFilterBar() {
  const [dslText, setDslText] = useState("");
  const [parseError, setParseError] = useState<string | null>(null);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [suggestions, setSuggestions] = useState<{ key: string; label: string; desc: string }[]>([]);
  const [showQuickFilters, setShowQuickFilters] = useState(false);
  const [savedFilters, setSavedFilters] = useState(loadSavedFilters());
  const [saveName, setSaveName] = useState("");
  const [showSaveInput, setShowSaveInput] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const total = useStore((s: any) => s.sessionList.length);
  const filtered = useStore((s: any) => s.filteredSessionList.length);

  const applyDslFilter = useCallback(async (dsl: string) => {
    if (!dsl.trim()) {
      useStore.getState().resetFilter();
      setParseError(null);
      return;
    }
    try {
      await invoke<number[]>("filter_traffic_dsl", { dslExpression: dsl });
      useStore.getState().setFilter({ searchText: `__dsl__${dsl}` } as any);
      setParseError(null);
    } catch (e) {
      setParseError(String(e));
    }
  }, []);

  const handleInputChange = (value: string) => {
    setDslText(value);
    const lastWord = value.split(/\s+/).pop() || "";
    if (lastWord.includes(":") && !lastWord.startsWith(":")) {
      const prefix = lastWord.split(":")[0].toLowerCase();
      const matches = DSL_FIELDS.filter((f) => f.key.startsWith(prefix) || f.label.startsWith(prefix));
      setSuggestions(matches);
      setShowSuggestions(matches.length > 0);
    } else if (lastWord.length > 0 && !lastWord.includes(":")) {
      const matches = DSL_FIELDS.filter(
        (f) => f.key.startsWith(lastWord.toLowerCase()) || f.label.startsWith(lastWord.toLowerCase())
      );
      setSuggestions(matches);
      setShowSuggestions(matches.length > 0);
    } else {
      setShowSuggestions(false);
    }
  };

  const handleSuggestionClick = (field: { key: string; label: string }) => {
    const words = dslText.split(/\s+/);
    words[words.length - 1] = field.label + ":";
    setDslText(words.join(" "));
    setShowSuggestions(false);
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      applyDslFilter(dslText);
      setShowSuggestions(false);
    }
    if (e.key === "Escape") {
      setShowSuggestions(false);
      setShowQuickFilters(false);
    }
  };

  const handleQuickFilter = (dsl: string) => {
    setDslText(dsl);
    applyDslFilter(dsl);
    setShowQuickFilters(false);
  };

  const handleSaveFilter = () => {
    if (!saveName.trim() || !dslText.trim()) return;
    const newFilters = [...savedFilters, { name: saveName, dsl: dslText }];
    setSavedFilters(newFilters);
    saveFilters(newFilters);
    setSaveName("");
    setShowSaveInput(false);
  };

  const handleDeleteSaved = (idx: number) => {
    const newFilters = savedFilters.filter((_, i) => i !== idx);
    setSavedFilters(newFilters);
    saveFilters(newFilters);
  };

  const clearFilter = () => {
    setDslText("");
    setParseError(null);
    useStore.getState().resetFilter();
  };

  const hasFilter = dslText.trim().length > 0;

  return (
    <div className="flex items-center gap-2 px-4 py-[6px] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
      <div className="relative flex-1 max-w-[400px]">
        <svg className="absolute left-[10px] top-1/2 -translate-y-1/2 w-3 h-3 text-[var(--color-text-secondary)] pointer-events-none z-10" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          ref={inputRef}
          type="text"
          value={dslText}
          onChange={(e) => handleInputChange(e.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => { if (suggestions.length > 0) setShowSuggestions(true); }}
          onBlur={() => setTimeout(() => setShowSuggestions(false), 200)}
          placeholder="过滤表达式: method:GET host:api.com status:200..299"
          className="w-full bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs pl-8 pr-3 py-[5px] rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none placeholder:text-[var(--color-text-secondary)]"
          spellCheck={false}
        />
        {showSuggestions && (
          <div className="absolute top-full left-0 mt-1 z-50 min-w-[200px] bg-[var(--color-bg-tertiary)] border border-[var(--color-border)] rounded shadow-lg overflow-hidden">
            {suggestions.map((s) => (
              <button key={s.key} className="w-full text-left px-3 py-2 text-xs hover:bg-[var(--color-accent)] hover:text-white cursor-pointer flex justify-between" onMouseDown={() => handleSuggestionClick(s)}>
                <span className="font-mono text-[var(--color-accent)]">{s.label}:</span>
                <span className="text-[var(--color-text-secondary)] ml-2">{s.desc}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      {parseError && (
        <span className="text-xs text-[var(--color-error)] truncate max-w-[200px]" title={parseError}>⚠ 语法错误</span>
      )}

      <div className="relative">
        <button onClick={() => setShowQuickFilters(!showQuickFilters)} className="text-xs px-2 py-[5px] rounded text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] cursor-pointer">
          ⚡ 快捷
        </button>
        {showQuickFilters && (
          <div className="absolute top-full left-0 mt-1 z-50 min-w-[180px] bg-[var(--color-bg-tertiary)] border border-[var(--color-border)] rounded shadow-lg overflow-hidden">
            {QUICK_FILTERS.map((qf) => (
              <button key={qf.label} className="w-full text-left px-3 py-2 text-xs hover:bg-[var(--color-accent)] hover:text-white cursor-pointer" onMouseDown={() => handleQuickFilter(qf.dsl)}>
                {qf.label}
              </button>
            ))}
            {savedFilters.length > 0 && (
              <>
                <div className="px-3 py-1 text-[10px] text-[var(--color-text-secondary)] border-t border-[var(--color-border)]">已保存</div>
                {savedFilters.map((sf, idx) => (
                  <div key={idx} className="flex items-center px-3 py-2 text-xs hover:bg-[var(--color-bg-secondary)] cursor-pointer" onMouseDown={() => handleQuickFilter(sf.dsl)}>
                    <span className="flex-1">{sf.name}</span>
                    <button className="text-[var(--color-error)] ml-2" onClick={(e) => { e.stopPropagation(); handleDeleteSaved(idx); }}>×</button>
                  </div>
                ))}
              </>
            )}
          </div>
        )}
      </div>

      {hasFilter && (
        <>
          <button onClick={clearFilter} className="text-xs px-2 py-[5px] rounded text-[var(--color-accent)] hover:bg-[var(--color-bg-tertiary)] cursor-pointer">重置</button>
          <span className="text-xs text-[var(--color-text-secondary)]">{filtered}/{total}</span>
          {showSaveInput ? (
            <div className="flex items-center gap-1">
              <input value={saveName} onChange={(e) => setSaveName(e.target.value)} placeholder="名称" className="bg-[var(--color-bg-tertiary)] text-xs px-2 py-[3px] rounded border border-[var(--color-border)] w-[80px] text-[var(--color-text-primary)]" onKeyDown={(e) => e.key === "Enter" && handleSaveFilter()} />
              <button onClick={handleSaveFilter} className="text-xs text-[var(--color-accent)] cursor-pointer">✓</button>
            </div>
          ) : (
            <button onClick={() => setShowSaveInput(true)} className="text-xs px-2 py-[5px] rounded text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] cursor-pointer" title="保存过滤器">💾</button>
          )}
        </>
      )}
    </div>
  );
}
