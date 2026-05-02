import { useStore, type StoreState, type FilterMethod, type FilterScheme, type FilterStatus } from "../../store";

const METHODS: { value: FilterMethod; label: string }[] = [
  { value: "ALL", label: "全部" },
  { value: "GET", label: "GET" },
  { value: "POST", label: "POST" },
  { value: "PUT", label: "PUT" },
  { value: "DELETE", label: "DELETE" },
  { value: "PATCH", label: "PATCH" },
  { value: "HEAD", label: "HEAD" },
  { value: "OPTIONS", label: "OPTIONS" },
  { value: "CONNECT", label: "CONNECT" },
];

const SCHEMES: { value: FilterScheme; label: string }[] = [
  { value: "ALL", label: "全部" },
  { value: "Http", label: "HTTP" },
  { value: "Https", label: "HTTPS" },
];

const STATUSES: { value: FilterStatus; label: string }[] = [
  { value: "ALL", label: "全部" },
  { value: "2xx", label: "2xx 成功" },
  { value: "3xx", label: "3xx 重定向" },
  { value: "4xx", label: "4xx 客户端错误" },
  { value: "5xx", label: "5xx 服务端错误" },
];

export function FilterBar() {
  const filter = useStore((s: StoreState) => s.filter);
  const setFilter = useStore((s: StoreState) => s.setFilter);
  const resetFilter = useStore((s: StoreState) => s.resetFilter);
  const total = useStore((s: StoreState) => s.sessionList.length);
  const filtered = useStore((s: StoreState) => s.filteredSessionList.length);
  const hasFilter = filter.searchText || filter.method !== "ALL" || filter.scheme !== "ALL" || filter.status !== "ALL";

  return (
    <div className="flex items-center gap-2 px-4 py-[6px] bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
      <div className="relative flex-1 max-w-[260px]">
        <svg className="absolute left-[10px] top-1/2 -translate-y-1/2 w-3 h-3 text-[var(--color-text-secondary)] pointer-events-none" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          value={filter.searchText}
          onChange={(e) => setFilter({ searchText: e.target.value })}
          placeholder="搜索 URL、主机、方法..."
          className="w-full bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs pl-8 pr-3 py-[5px] rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none placeholder:text-[var(--color-text-secondary)]"
        />
      </div>

      <select
        value={filter.method}
        onChange={(e) => setFilter({ method: e.target.value as FilterMethod })}
        className="bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-2 py-[5px] rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer min-w-[70px]"
      >
        {METHODS.map((m) => (
          <option key={m.value} value={m.value}>{m.label}</option>
        ))}
      </select>

      <select
        value={filter.scheme}
        onChange={(e) => setFilter({ scheme: e.target.value as FilterScheme })}
        className="bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-2 py-[5px] rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer min-w-[70px]"
      >
        {SCHEMES.map((s) => (
          <option key={s.value} value={s.value}>{s.label}</option>
        ))}
      </select>

      <select
        value={filter.status}
        onChange={(e) => setFilter({ status: e.target.value as FilterStatus })}
        className="bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-2 py-[5px] rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer min-w-[110px]"
      >
        {STATUSES.map((s) => (
          <option key={s.value} value={s.value}>{s.label}</option>
        ))}
      </select>

      {hasFilter && (
        <button
          onClick={resetFilter}
          className="text-xs px-2 py-[5px] rounded text-[var(--color-accent)] hover:bg-[var(--color-bg-tertiary)] transition-colors cursor-pointer"
        >
          重置
        </button>
      )}

      {hasFilter && (
        <span className="text-xs text-[var(--color-text-secondary)]">
          {filtered}/{total}
        </span>
      )}
    </div>
  );
}
