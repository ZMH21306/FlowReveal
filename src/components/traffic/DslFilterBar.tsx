import { useState, useRef, useCallback } from "react";
import { useStore, type StoreState } from "../../store";
import { invoke } from "@tauri-apps/api/core";

export function DslFilterBar() {
  const [dslText, setDslText] = useState("");
  const [parseError, setParseError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const total = useStore((s: StoreState) => s.sessionList.length);
  const filtered = useStore((s: StoreState) => s.filteredSessionList.length);

  const applyDslFilter = useCallback(async (dsl: string) => {
    if (!dsl.trim()) {
      useStore.getState().clearDslFilter();
      setParseError(null);
      return;
    }
    try {
      const filteredIds = await invoke<number[]>("filter_traffic_dsl", { dslExpression: dsl });
      useStore.getState().setDslFilter(dsl, filteredIds);
      setParseError(null);
    } catch (e) {
      setParseError(String(e));
    }
  }, []);

  const handleInputChange = (value: string) => {
    setDslText(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => applyDslFilter(value), 400);
  };

  const clearFilter = () => {
    setDslText("");
    setParseError(null);
    useStore.getState().clearDslFilter();
  };

  return (
    <div className="flex items-center gap-2 px-4 py-1.5 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border-subtle)] shrink-0">
      <span className="text-[10px] font-semibold text-[var(--color-text-muted)] uppercase tracking-wider whitespace-nowrap">DSL</span>
      <div className="flex-1 relative">
        <input
          value={dslText}
          onChange={(e) => handleInputChange(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") applyDslFilter(dslText); }}
          placeholder="method:GET AND host:example.com | status:200..299 | body:json"
          className={`w-full px-3 py-1 text-[11px] font-mono bg-[var(--color-bg-tertiary)] border rounded-[var(--radius-md)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] transition-all focus:outline-none focus:ring-2 ${
            parseError
              ? "border-[var(--color-error)] focus:ring-[var(--color-error-muted)]"
              : "border-[var(--color-border-subtle)] focus:ring-[var(--color-accent-muted)] focus:border-[var(--color-accent)]"
          }`}
        />
        {parseError && (
          <span className="absolute right-2 top-1/2 -translate-y-1/2 text-[9px] text-[var(--color-error)] bg-[var(--color-error-muted)] px-1.5 py-0.5 rounded-[var(--radius-sm)]">
            语法错误
          </span>
        )}
      </div>
      {dslText && (
        <button
          onClick={clearFilter}
          className="px-2 py-1 text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] rounded-[var(--radius-sm)] hover:bg-[var(--color-bg-tertiary)] transition-all"
        >
          ✕
        </button>
      )}
      <div className="flex items-center gap-1.5 text-[10px] text-[var(--color-text-muted)] whitespace-nowrap">
        <span className="font-medium text-[var(--color-text-primary)]">{filtered}</span>
        <span>/</span>
        <span>{total}</span>
      </div>
    </div>
  );
}
